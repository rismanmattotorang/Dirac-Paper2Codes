pub mod dynamic_tests;
pub mod static_analysis;
pub mod symbolic_verification;

pub use dynamic_tests::DynamicTester;
pub use static_analysis::StaticAnalyzer;
pub use symbolic_verification::SymbolicVerifier;

use crate::config::Config;
use crate::error::Result;
use crate::types::{Repository, Specification, VerificationMetrics, VerificationReport};

#[derive(Clone)]
pub struct SACVPipeline {
    static_analyzer: StaticAnalyzer,
    dynamic_tester: DynamicTester,
    symbolic_verifier: SymbolicVerifier,
    symbolic_enabled: bool,
}

impl SACVPipeline {
    pub fn new() -> Self {
        Self {
            static_analyzer: StaticAnalyzer::new(),
            dynamic_tester: DynamicTester::new(),
            symbolic_verifier: SymbolicVerifier::new(),
            symbolic_enabled: true,
        }
    }

    pub fn with_config(config: &Config) -> Self {
        Self {
            static_analyzer: StaticAnalyzer::new(),
            dynamic_tester: DynamicTester::new(),
            symbolic_verifier: SymbolicVerifier::with_config(&config.verification),
            symbolic_enabled: config.verification.enable_symbolic_verification,
        }
    }

    pub async fn verify(
        &self,
        repository: &Repository,
        specifications: &[Specification],
    ) -> Result<VerificationReport> {
        let mut all_issues = Vec::new();
        let mut total_checks = 0;
        let mut passed_checks = 0;
        let mut failed_checks = 0;
        let mut warnings = 0;

        // Phase 1: Static checks
        for module in &repository.modules {
            let static_issues = self.static_analyzer.analyze(module).await?;
            total_checks += static_issues.len();
            for issue in &static_issues {
                match issue.severity {
                    crate::types::IssueSeverity::Critical | crate::types::IssueSeverity::Error => {
                        failed_checks += 1;
                    }
                    crate::types::IssueSeverity::Warning => {
                        warnings += 1;
                    }
                    _ => {
                        passed_checks += 1;
                    }
                }
            }
            all_issues.extend(static_issues);
        }

        // If critical issues found, return early
        if all_issues
            .iter()
            .any(|i| i.severity == crate::types::IssueSeverity::Critical)
        {
            return Ok(VerificationReport {
                module_id: None,
                issues: all_issues,
                passed: false,
                metrics: VerificationMetrics {
                    total_checks,
                    passed_checks,
                    failed_checks,
                    warnings,
                },
            });
        }

        // Phase 2: Dynamic tests
        for module in &repository.modules {
            let test_issues = self.dynamic_tester.test(module, specifications).await?;
            total_checks += test_issues.len();
            for issue in &test_issues {
                match issue.severity {
                    crate::types::IssueSeverity::Critical | crate::types::IssueSeverity::Error => {
                        failed_checks += 1;
                    }
                    crate::types::IssueSeverity::Warning => {
                        warnings += 1;
                    }
                    _ => {
                        passed_checks += 1;
                    }
                }
            }
            all_issues.extend(test_issues);
        }

        // Phase 3: Symbolic verification
        if self.symbolic_enabled && self.symbolic_verifier.is_enabled() {
            for spec in specifications {
                let symbolic_issues = self.symbolic_verifier.verify(spec, repository).await?;
                total_checks += symbolic_issues.len();
                for issue in &symbolic_issues {
                    match issue.severity {
                        crate::types::IssueSeverity::Critical
                        | crate::types::IssueSeverity::Error => {
                            failed_checks += 1;
                        }
                        crate::types::IssueSeverity::Warning => {
                            warnings += 1;
                        }
                        _ => {
                            passed_checks += 1;
                        }
                    }
                }
                all_issues.extend(symbolic_issues);
            }
        }

        Ok(VerificationReport {
            module_id: None,
            issues: all_issues,
            passed: failed_checks == 0,
            metrics: VerificationMetrics {
                total_checks,
                passed_checks,
                failed_checks,
                warnings,
            },
        })
    }
}

impl Default for SACVPipeline {
    fn default() -> Self {
        Self::new()
    }
}
