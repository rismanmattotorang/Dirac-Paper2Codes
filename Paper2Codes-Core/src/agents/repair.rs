//! Self-debugging repair feedback for the verification → fix loop.
//!
//! When verification fails, the orchestrator schedules fix tasks. Previously it
//! emitted one fix task per issue with the raw issue message. Following the
//! self-debugging literature (Chen et al., 2023), more useful repair prompts
//! group all of a module's issues into a single, severity-prioritised,
//! explanation-oriented brief — letting the coder reason about the module as a
//! whole instead of reacting to one symptom at a time.
//!
//! This module turns a set of [`VerificationIssue`]s into such a brief
//! ([`format_repair_feedback`]) and provides a small capped-iteration policy
//! ([`should_retry`]) so repair loops terminate.

use crate::types::{IssueSeverity, VerificationIssue};

/// Group issues by their target module file, preserving first-seen order.
///
/// Issues without a location are collected under the returned `unlocated` list
/// so callers can still surface them.
pub fn group_by_module<'a>(
    issues: &'a [VerificationIssue],
) -> (Vec<(String, Vec<&'a VerificationIssue>)>, Vec<&'a VerificationIssue>) {
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<&VerificationIssue>> =
        std::collections::HashMap::new();
    let mut unlocated: Vec<&VerificationIssue> = Vec::new();

    for issue in issues {
        match &issue.location {
            Some(loc) => {
                let key = loc.file.to_string_lossy().to_string();
                if !groups.contains_key(&key) {
                    order.push(key.clone());
                }
                groups.entry(key).or_default().push(issue);
            }
            None => unlocated.push(issue),
        }
    }

    let grouped = order
        .into_iter()
        .map(|k| {
            let v = groups.remove(&k).unwrap_or_default();
            (k, v)
        })
        .collect();
    (grouped, unlocated)
}

fn severity_label(s: &IssueSeverity) -> &'static str {
    match s {
        IssueSeverity::Critical => "CRITICAL",
        IssueSeverity::Error => "ERROR",
        IssueSeverity::Warning => "WARNING",
        IssueSeverity::Info => "INFO",
    }
}

/// Build a self-debugging repair brief for a single module from its issues.
///
/// Issues are sorted most-severe first; each is rendered with its severity,
/// location line, message, and suggestion (when present). The brief opens with
/// a short rubber-duck instruction to encourage the model to reason about root
/// cause before editing.
pub fn format_repair_feedback(module: &str, issues: &[&VerificationIssue]) -> String {
    let mut sorted: Vec<&VerificationIssue> = issues.to_vec();
    // Severity derives Ord with Critical greatest; sort descending.
    sorted.sort_by(|a, b| b.severity.cmp(&a.severity));

    let mut out = String::new();
    out.push_str(&format!(
        "Fix `{}`. The verifier reported {} issue(s). Before editing, briefly \
         explain the likely root cause of each, then apply minimal, correct \
         changes that resolve all of them without breaking existing behaviour.\n\n",
        module,
        sorted.len()
    ));

    for (i, issue) in sorted.iter().enumerate() {
        let line = issue
            .location
            .as_ref()
            .map(|l| format!(" (line {})", l.line))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}. [{}] {}{}\n",
            i + 1,
            severity_label(&issue.severity),
            issue.message,
            line
        ));
        if let Some(suggestion) = &issue.suggestion {
            if !suggestion.trim().is_empty() {
                out.push_str(&format!("   Suggestion: {}\n", suggestion));
            }
        }
    }

    out
}

/// Capped-iteration policy for repair loops.
///
/// Retry only while under `max_attempts` and only when the previous round still
/// failed *and* made progress (fewer issues than before). Refusing to retry on
/// a plateau prevents the loop from burning iterations without improvement.
pub fn should_retry(attempt: usize, max_attempts: usize, last_failed: bool, improved: bool) -> bool {
    last_failed && improved && attempt + 1 < max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CodeLocation, IssueCategory};
    use std::path::PathBuf;

    fn issue(sev: IssueSeverity, msg: &str, file: Option<&str>, line: usize) -> VerificationIssue {
        VerificationIssue {
            severity: sev,
            category: IssueCategory::Logic,
            message: msg.to_string(),
            location: file.map(|f| CodeLocation {
                file: PathBuf::from(f),
                line,
                column: None,
            }),
            suggestion: None,
        }
    }

    #[test]
    fn groups_issues_by_module_and_collects_unlocated() {
        let issues = vec![
            issue(IssueSeverity::Error, "a", Some("foo.py"), 1),
            issue(IssueSeverity::Warning, "b", Some("bar.py"), 2),
            issue(IssueSeverity::Error, "c", Some("foo.py"), 3),
            issue(IssueSeverity::Info, "d", None, 0),
        ];
        let (groups, unlocated) = group_by_module(&issues);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "foo.py");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "bar.py");
        assert_eq!(unlocated.len(), 1);
    }

    #[test]
    fn feedback_orders_by_severity_and_includes_details() {
        let warn = issue(IssueSeverity::Warning, "minor style", Some("m.py"), 10);
        let crit = issue(IssueSeverity::Critical, "null deref", Some("m.py"), 5);
        let brief = format_repair_feedback("m.py", &[&warn, &crit]);
        let crit_pos = brief.find("null deref").unwrap();
        let warn_pos = brief.find("minor style").unwrap();
        assert!(crit_pos < warn_pos, "critical issue should be listed first");
        assert!(brief.contains("CRITICAL"));
        assert!(brief.contains("(line 5)"));
        assert!(brief.contains("2 issue(s)"));
    }

    #[test]
    fn retry_policy_respects_cap_and_progress() {
        assert!(should_retry(0, 3, true, true));
        assert!(!should_retry(2, 3, true, true)); // at cap
        assert!(!should_retry(0, 3, false, true)); // already passing
        assert!(!should_retry(0, 3, true, false)); // plateau, no improvement
    }
}
