//! Domain-specific **Skills** for Dirac-Paper2Codes.
//!
//! A *Skill* is a modular, reusable, user-editable bundle of domain expertise —
//! following the Agent Skills convention (a name + trigger description + a
//! procedural body, plus structured metadata). Selecting a skill specialises the
//! whole paper→code pipeline for a computational domain:
//!
//! * **retrieval** — domain keywords bias the hybrid retriever,
//! * **generation** — a domain expertise primer and language-specific library
//!   recommendations are injected into the coding prompt, and
//! * **verification** — domain invariants (e.g. energy conservation) are surfaced
//!   as hints.
//!
//! Skills are *choosable* (list/select), *reusable* (built-ins ship with the
//! engine), and *improvable* (users add or override skills as TOML files in a
//! skills directory). The main user scenario is: pick a domain skill, choose a
//! language, upload a paper, and receive a domain-idiomatic implementation.

mod builtin;

pub use builtin::builtin_skills;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Paper2CodesError, Result};

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_true() -> bool {
    true
}

/// A reusable domain-specialisation skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    /// Stable id / slug, e.g. `"computational-finance"`.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// Semantic version (lets users iterate on a skill).
    #[serde(default = "default_version")]
    pub version: String,
    /// One-line description / trigger summary.
    pub description: String,
    /// Optional grouping domain label.
    #[serde(default)]
    pub domain: Option<String>,
    /// Trigger / retrieval keywords for this domain.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Supported languages; the first entry is the default.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Recommended libraries per language (lowercase language keys).
    #[serde(default)]
    pub libraries: HashMap<String, Vec<String>>,
    /// Procedural body — the domain expertise primer injected into prompts.
    pub instructions: String,
    /// Domain invariants / properties the verifier should check.
    #[serde(default)]
    pub verification_hints: Vec<String>,
    /// True for engine-shipped skills; false for user-authored ones.
    #[serde(default = "default_true")]
    pub builtin: bool,
}

impl Skill {
    /// The default language for this skill (first listed, or `"python"`).
    pub fn default_language(&self) -> &str {
        self.languages
            .first()
            .map(|s| s.as_str())
            .unwrap_or("python")
    }

    /// Whether the skill explicitly supports `language` (case-insensitive). When
    /// the skill lists no languages, all languages are considered supported.
    pub fn supports_language(&self, language: &str) -> bool {
        if self.languages.is_empty() {
            return true;
        }
        self.languages
            .iter()
            .any(|l| l.eq_ignore_ascii_case(language))
    }

    /// Recommended libraries for `language` (case-insensitive lookup).
    pub fn libraries_for(&self, language: &str) -> Vec<String> {
        let lang = language.to_lowercase();
        self.libraries.get(&lang).cloned().unwrap_or_default()
    }

    /// Build the domain expertise primer to prepend to the coding/analysis
    /// prompt for a chosen `language`. Combines the procedural body, the
    /// recommended libraries, and the verification hints into a compact,
    /// deterministic block.
    pub fn system_prompt(&self, language: &str) -> String {
        let mut out = format!(
            "You are operating with the '{}' domain skill (v{}). {}\n",
            self.name, self.version, self.instructions
        );

        let libs = self.libraries_for(language);
        if !libs.is_empty() {
            out.push_str(&format!(
                "Prefer these idiomatic {} libraries when appropriate: {}.\n",
                language,
                libs.join(", ")
            ));
        }

        if !self.verification_hints.is_empty() {
            out.push_str("Ensure the implementation respects these domain properties:\n");
            for hint in &self.verification_hints {
                out.push_str(&format!("- {}\n", hint));
            }
        }
        out
    }

    /// Augment a task description with this skill's domain guidance for the
    /// chosen language. The result is fed to the coding agent (driving
    /// generation) and to the retriever (the added domain terms bias CPR).
    pub fn augment_task(&self, base_description: &str, language: &str) -> String {
        format!(
            "{}\n\n--- Domain guidance ({}) ---\n{}",
            base_description,
            self.name,
            self.system_prompt(language)
        )
    }

    /// Validate a skill's required fields (used on user upsert / load).
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(Paper2CodesError::Validation("Skill id cannot be empty".into()));
        }
        if self.name.trim().is_empty() {
            return Err(Paper2CodesError::Validation("Skill name cannot be empty".into()));
        }
        if self.instructions.trim().is_empty() {
            return Err(Paper2CodesError::Validation(format!(
                "Skill '{}' must have instructions",
                self.id
            )));
        }
        Ok(())
    }
}

/// An ordered, lookup-friendly collection of skills.
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: Vec<Skill>,
}

impl SkillRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// A registry pre-loaded with the engine's built-in domain skills.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        for skill in builtin_skills() {
            registry.upsert(skill);
        }
        registry
    }

    /// All skills in deterministic (insertion) order.
    pub fn list(&self) -> &[Skill] {
        &self.skills
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Look up a skill by id.
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.id == id)
    }

    /// Insert a skill, replacing any existing skill with the same id (preserving
    /// its position) — this is how users *improve* a skill.
    pub fn upsert(&mut self, skill: Skill) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.id == skill.id) {
            *existing = skill;
        } else {
            self.skills.push(skill);
        }
    }

    /// Suggest the best-matching skill for free text (e.g. a paper title /
    /// abstract) by counting keyword hits. Returns `None` on no match.
    pub fn match_skill(&self, text: &str) -> Option<&Skill> {
        let haystack = text.to_lowercase();
        self.skills
            .iter()
            .map(|s| {
                let score = s
                    .keywords
                    .iter()
                    .filter(|kw| haystack.contains(&kw.to_lowercase()))
                    .count();
                (s, score)
            })
            .filter(|(_, score)| *score > 0)
            .max_by_key(|(_, score)| *score)
            .map(|(s, _)| s)
    }

    /// Load user-authored skills (`*.toml`) from `dir`, upserting each so users
    /// can add new skills or override built-ins. Returns the count loaded.
    /// A missing directory is not an error (returns 0).
    pub fn load_user_dir(&mut self, dir: impl AsRef<Path>) -> Result<usize> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(0);
        }
        let mut loaded = 0;
        let entries = std::fs::read_dir(dir).map_err(Paper2CodesError::Io)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(Paper2CodesError::Io)?;
            let mut skill: Skill = toml::from_str(&content).map_err(|e| {
                Paper2CodesError::Config(ConfigError::Invalid(format!(
                    "Invalid skill file {}: {}",
                    path.display(),
                    e
                )))
            })?;
            skill.builtin = false;
            skill.validate()?;
            self.upsert(skill);
            loaded += 1;
        }
        Ok(loaded)
    }

    /// Persist a user skill to `dir/<id>.toml`, creating the directory if needed.
    pub fn save_skill(dir: impl AsRef<Path>, skill: &Skill) -> Result<PathBuf> {
        skill.validate()?;
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(Paper2CodesError::Io)?;
        let path = dir.join(format!("{}.toml", skill.id));
        let toml = toml::to_string_pretty(skill).map_err(|e| {
            Paper2CodesError::Config(ConfigError::Invalid(format!(
                "Failed to serialize skill '{}': {}",
                skill.id, e
            )))
        })?;
        std::fs::write(&path, toml).map_err(Paper2CodesError::Io)?;
        Ok(path)
    }

    /// Default user skills directory (`<config-dir>/paper2codes/skills`).
    pub fn user_skills_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir().ok_or_else(|| {
            Paper2CodesError::Config(ConfigError::NotFound("config directory".into()))
        })?;
        Ok(dir.join("paper2codes").join("skills"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Skill {
        let mut libraries = HashMap::new();
        libraries.insert("python".to_string(), vec!["numpy".to_string(), "scipy".to_string()]);
        Skill {
            id: "test-domain".into(),
            name: "Test Domain".into(),
            version: "1.0.0".into(),
            description: "d".into(),
            domain: Some("Testing".into()),
            keywords: vec!["wavelet".into(), "fourier".into()],
            languages: vec!["python".into(), "rust".into()],
            libraries,
            instructions: "Implement carefully.".into(),
            verification_hints: vec!["energy is conserved".into()],
            builtin: true,
        }
    }

    #[test]
    fn builtins_cover_requested_domains() {
        let reg = SkillRegistry::with_builtins();
        for id in [
            "computational-finance",
            "computational-physics",
            "computational-chemistry",
            "computational-biology-bioinformatics",
            "computational-genomics",
            "quantum-computation",
            "computational-fluid-dynamics",
            "computational-supply-chain",
        ] {
            assert!(reg.get(id).is_some(), "missing built-in skill: {}", id);
        }
        // Every built-in is valid and has python libraries + a primer.
        for skill in reg.list() {
            skill.validate().unwrap();
            assert!(!skill.libraries_for("python").is_empty(), "{} has no python libs", skill.id);
            assert!(skill.system_prompt("python").contains(&skill.name));
        }
    }

    #[test]
    fn language_and_library_accessors() {
        let s = sample();
        assert_eq!(s.default_language(), "python");
        assert!(s.supports_language("RUST"));
        assert!(!s.supports_language("go"));
        assert_eq!(s.libraries_for("Python"), vec!["numpy", "scipy"]);
        assert!(s.libraries_for("go").is_empty());
    }

    #[test]
    fn system_prompt_includes_libs_and_hints() {
        let prompt = sample().system_prompt("python");
        assert!(prompt.contains("Test Domain"));
        assert!(prompt.contains("numpy, scipy"));
        assert!(prompt.contains("energy is conserved"));
    }

    #[test]
    fn augment_task_prepends_base_and_adds_guidance() {
        let augmented = sample().augment_task("Implement module X", "python");
        assert!(augmented.starts_with("Implement module X"));
        assert!(augmented.contains("Domain guidance (Test Domain)"));
        assert!(augmented.contains("numpy"));
    }

    #[test]
    fn upsert_replaces_in_place_and_match_scores_keywords() {
        let mut reg = SkillRegistry::new();
        reg.upsert(sample());
        assert_eq!(reg.len(), 1);
        let mut improved = sample();
        improved.version = "2.0.0".into();
        reg.upsert(improved);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("test-domain").unwrap().version, "2.0.0");

        let matched = reg.match_skill("A study of the Fourier transform").unwrap();
        assert_eq!(matched.id, "test-domain");
        assert!(reg.match_skill("unrelated topic").is_none());
    }

    #[test]
    fn save_and_load_user_skill_roundtrip() {
        let dir = std::env::temp_dir().join(format!("p2c_skills_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut user = sample();
        user.id = "user-skill".into();
        user.builtin = true; // should be forced to false on load
        SkillRegistry::save_skill(&dir, &user).unwrap();

        let mut reg = SkillRegistry::new();
        let n = reg.load_user_dir(&dir).unwrap();
        assert_eq!(n, 1);
        let loaded = reg.get("user-skill").unwrap();
        assert_eq!(loaded.name, "Test Domain");
        assert!(!loaded.builtin, "loaded user skills must be marked non-builtin");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_user_dir_is_not_an_error() {
        let mut reg = SkillRegistry::new();
        let n = reg.load_user_dir("/nonexistent/path/xyz").unwrap();
        assert_eq!(n, 0);
    }
}
