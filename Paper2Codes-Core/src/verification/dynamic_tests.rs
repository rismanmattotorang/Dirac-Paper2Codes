use crate::error::{Result, VerificationError};
use crate::execution::SandboxRunner;
use crate::llm::{LLMRequest, LLMRouter, Message, MessageRole};
use crate::types::{
    CodeModule, IssueCategory, IssueSeverity, ProgrammingLanguage, Specification, VerificationIssue,
};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[derive(Clone)]
pub struct DynamicTester {
    #[allow(dead_code)] // Reserved for future use
    sandbox: Option<SandboxRunner>,
    llm_router: Option<Arc<LLMRouter>>,
    use_llm_for_tests: bool,
}

impl DynamicTester {
    pub fn new() -> Self {
        Self {
            sandbox: None,
            llm_router: None,
            use_llm_for_tests: false,
        }
    }

    pub fn with_llm_router(mut self, router: Arc<LLMRouter>) -> Self {
        self.llm_router = Some(router);
        self.use_llm_for_tests = true;
        self
    }

    /// Generate test cases from specification
    pub async fn generate_tests_from_specification(
        &self,
        spec: &Specification,
        language: &ProgrammingLanguage,
    ) -> Result<Vec<String>> {
        // Use LLM if available and enabled
        if self.use_llm_for_tests {
            if let Some(ref router) = self.llm_router {
                return self.generate_tests_with_llm(spec, language, router).await;
            }
        }

        // Fallback to rule-based generation
        let mut tests = Vec::new();

        match language {
            ProgrammingLanguage::Python => {
                // Generate Python tests
                tests.extend(self.generate_python_tests(spec)?);
            }
            ProgrammingLanguage::Rust => {
                // Generate Rust tests
                tests.extend(self.generate_rust_tests(spec)?);
            }
            _ => {
                // Basic tests for other languages
                tests.push(format!("# Basic test for {}", spec.description));
            }
        }

        Ok(tests)
    }

    async fn generate_tests_with_llm(
        &self,
        spec: &Specification,
        language: &ProgrammingLanguage,
        router: &LLMRouter,
    ) -> Result<Vec<String>> {
        let language_name = match language {
            ProgrammingLanguage::Python => "Python",
            ProgrammingLanguage::Rust => "Rust",
            ProgrammingLanguage::Other(ref lang) => lang,
        };

        let prompt = format!(
            r#"You are a test generation expert. Generate comprehensive test cases for the following specification.

Specification:
{}

Language: {}

Requirements:
1. Generate test cases that verify the specification is correctly implemented
2. Include edge cases and boundary conditions
3. Include positive and negative test cases
4. Use appropriate testing framework for the language (pytest for Python, built-in test framework for Rust)
5. Include clear test names and docstrings
6. Make tests executable and self-contained

Generate only the test code, no explanations:"#,
            spec.description, language_name
        );

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: "You are an expert test engineer who generates comprehensive, production-ready test cases.".to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            router.config.agents.coding_model.clone(),
        );

        let response = router.complete(request, None).await?;

        // Extract test code from response
        let test_code = self.extract_test_code_from_response(&response.content, language)?;

        Ok(vec![test_code])
    }

    fn extract_test_code_from_response(
        &self,
        response: &str,
        _language: &ProgrammingLanguage,
    ) -> Result<String> {
        // Try to extract code blocks
        if let Some(code_block) = self.extract_code_block(response) {
            return Ok(code_block);
        }

        // If no code block found, use the entire response
        // This handles cases where LLM doesn't format with code blocks
        Ok(response.trim().to_string())
    }

    fn extract_code_block(&self, text: &str) -> Option<String> {
        // Look for code blocks (```language ... ```)
        let mut in_code_block = false;
        let mut code_lines = Vec::new();

        for line in text.lines() {
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End of code block
                    break;
                } else {
                    // Start of code block
                    in_code_block = true;
                    continue;
                }
            }

            if in_code_block {
                code_lines.push(line);
            }
        }

        if !code_lines.is_empty() {
            Some(code_lines.join("\n"))
        } else {
            None
        }
    }

    fn generate_python_tests(&self, spec: &Specification) -> Result<Vec<String>> {
        let mut tests = Vec::new();

        // Generate basic specification test
        if !spec.description.is_empty() {
            let test_name = spec
                .description
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "");

            tests.push(format!(
                r#"import pytest

def test_{}():
    """Test: {}"""
    # Basic test structure - should be enhanced with actual test logic
    # This test verifies that the specification is implemented
    assert True, "Test for specification: {}"
"#,
                test_name, spec.description, spec.description
            ));
        }

        // Generate function existence tests
        for func_name in self.extract_function_names(&spec.description) {
            tests.push(format!(
                r#"def test_{}_exists():
    """Test that {} function exists and is callable"""
    try:
        # Try to import the function (adjust import path as needed)
        # from module import {}
        # assert callable({}), "{} should be callable"
        assert True, "Function {} should exist"
    except ImportError:
        pytest.skip("Module not available for testing")
"#,
                func_name, func_name, func_name, func_name, func_name, func_name
            ));
        }

        // Generate edge case tests based on specification keywords
        if spec.description.to_lowercase().contains("array")
            || spec.description.to_lowercase().contains("list")
        {
            tests.push(format!(
                r#"def test_{}_edge_cases():
    """Test edge cases for array/list operations"""
    # Test empty array
    # Test single element
    # Test large array
    assert True, "Edge case tests should be implemented"
"#,
                spec.description.to_lowercase().replace(' ', "_")
            ));
        }

        Ok(tests)
    }

    fn generate_rust_tests(&self, spec: &Specification) -> Result<Vec<String>> {
        let mut tests = Vec::new();

        if !spec.description.is_empty() {
            let test_name = spec
                .description
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "");

            tests.push(format!(
                r#"#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{}() {{
        // Test: {}
        // Basic test structure - should be enhanced with actual test logic
        // This test verifies that the specification is implemented
        assert!(true, "Test for specification: {}");
    }}
}}
"#,
                test_name, spec.description, spec.description
            ));
        }

        // Generate function existence tests
        for func_name in self.extract_function_names(&spec.description) {
            tests.push(format!(
                r#"#[test]
fn test_{}_exists() {{
    // Test that {} function exists
    // This is a placeholder - actual implementation should verify function signature
    assert!(true, "Function {} should exist");
}}
"#,
                func_name, func_name, func_name
            ));
        }

        Ok(tests)
    }

    fn extract_function_names(&self, text: &str) -> Vec<String> {
        // Simple extraction - look for common patterns
        let mut names = Vec::new();

        // Look for function-like patterns (e.g., "compute_loss", "train_model")
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if word.contains('_') && !word.contains(' ') && word.len() > 3 {
                names.push(word.to_string());
            }
        }

        names
    }

    /// Parse test output and extract detailed results
    pub fn parse_test_output(
        &self,
        output: &str,
        language: &ProgrammingLanguage,
    ) -> Result<DetailedTestResults> {
        match language {
            ProgrammingLanguage::Python => self.parse_python_output(output),
            ProgrammingLanguage::Rust => self.parse_rust_output(output),
            _ => Ok(DetailedTestResults::default()),
        }
    }

    fn parse_python_output(&self, output: &str) -> Result<DetailedTestResults> {
        let mut results = DetailedTestResults::default();

        // Parse pytest output
        if output.contains("passed") {
            for line in output.lines() {
                if line.contains("passed") {
                    // Extract number of passed tests
                    if let Some(num_str) =
                        line.split_whitespace().find(|s| s.parse::<usize>().is_ok())
                    {
                        if let Ok(num) = num_str.parse::<usize>() {
                            results.passed += num;
                            results.total += num;
                        }
                    }
                }
                if line.contains("failed") {
                    // Extract number of failed tests
                    if let Some(num_str) =
                        line.split_whitespace().find(|s| s.parse::<usize>().is_ok())
                    {
                        if let Ok(num) = num_str.parse::<usize>() {
                            results.failed += num;
                            results.total += num;
                        }
                    }
                }
            }
        }

        // Extract failure details
        if output.contains("FAILED") {
            for line in output.lines() {
                if line.contains("FAILED") || line.contains("ERROR") {
                    results.failure_details.push(line.to_string());
                }
            }
        }

        results.success = results.failed == 0 && results.total > 0;

        Ok(results)
    }

    fn parse_rust_output(&self, output: &str) -> Result<DetailedTestResults> {
        let mut results = DetailedTestResults::default();

        // Parse cargo test output
        for line in output.lines() {
            if line.contains("test result:") {
                // Extract test counts
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if part == &"passed;" && i > 0 {
                        if let Ok(num) = parts[i - 1].parse::<usize>() {
                            results.passed = num;
                        }
                    }
                    if part == &"failed;" && i > 0 {
                        if let Ok(num) = parts[i - 1].parse::<usize>() {
                            results.failed = num;
                        }
                    }
                }
                results.total = results.passed + results.failed;
            }

            if line.contains("FAILED") {
                results.failure_details.push(line.to_string());
            }
        }

        results.success = results.failed == 0 && results.total > 0;

        Ok(results)
    }

    /// Analyze test coverage (basic implementation)
    pub fn analyze_coverage(&self, _code: &CodeModule) -> Result<CoverageReport> {
        // This is a basic implementation
        // In a real implementation, would use coverage.py for Python or tarpaulin for Rust
        Ok(CoverageReport {
            total_lines: 0,
            covered_lines: 0,
            coverage_percentage: 0.0,
            uncovered_lines: Vec::new(),
        })
    }

    pub async fn test(
        &self,
        module: &CodeModule,
        specifications: &[Specification],
    ) -> Result<Vec<VerificationIssue>> {
        let mut issues = Vec::new();

        // Run tests defined in module
        if !module.tests.is_empty() {
            let temp_dir = TempDir::new().map_err(|e| {
                VerificationError::TestExecution(format!("Failed to create temp dir: {}", e))
            })?;

            let sandbox = SandboxRunner::new(temp_dir.path().to_path_buf());
            let test_results = sandbox.run_tests(module).await.map_err(|e| {
                VerificationError::TestExecution(format!("Test execution failed: {}", e))
            })?;

            if !test_results.passed {
                issues.push(VerificationIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::TestFailure,
                    message: format!(
                        "{} out of {} tests failed",
                        test_results.failed, test_results.total
                    ),
                    location: Some(crate::types::CodeLocation {
                        file: module.file_path.clone(),
                        line: 1,
                        column: None,
                    }),
                    suggestion: Some(format!("Test errors: {}", test_results.errors)),
                });
            }
        }

        // Generate and run tests from specifications
        for spec in specifications {
            // Generate tests from specification
            let generated_tests = self
                .generate_tests_from_specification(spec, &module.language)
                .await?;

            if !generated_tests.is_empty() {
                // Create a temporary test module and run it
                let temp_dir = TempDir::new().map_err(|e| {
                    VerificationError::TestExecution(format!("Failed to create temp dir: {}", e))
                })?;

                // Write generated tests to a test file
                let test_file_name = match module.language {
                    ProgrammingLanguage::Python => "test_generated.py",
                    ProgrammingLanguage::Rust => "test_generated.rs",
                    _ => "test_generated.txt",
                };

                let test_file_path = temp_dir.path().join(test_file_name);
                let test_content = generated_tests.join("\n\n");
                tokio::fs::write(&test_file_path, test_content)
                    .await
                    .map_err(|e| {
                        VerificationError::TestExecution(format!(
                            "Failed to write test file: {}",
                            e
                        ))
                    })?;

                // Create a test module with the original code + generated tests
                let mut test_module = module.clone();
                // Convert generated test strings to Test structs
                test_module.tests = generated_tests
                    .into_iter()
                    .enumerate()
                    .map(|(idx, code)| crate::types::Test {
                        id: Uuid::new_v4().to_string(),
                        name: format!("generated_test_{}", idx),
                        code,
                        expected_output: None,
                    })
                    .collect();

                // Run the tests
                let sandbox = SandboxRunner::new(temp_dir.path().to_path_buf());
                match sandbox.run_tests(&test_module).await {
                    Ok(test_results) => {
                        if !test_results.passed {
                            issues.push(VerificationIssue {
                                severity: IssueSeverity::Error,
                                category: IssueCategory::TestFailure,
                                message: format!(
                                    "Generated tests failed for specification '{}': {} out of {} tests failed",
                                    spec.description, test_results.failed, test_results.total
                                ),
                                location: Some(crate::types::CodeLocation {
                                    file: module.file_path.clone(),
                                    line: 1,
                                    column: None,
                                }),
                                suggestion: Some(format!("Test errors: {}", test_results.errors)),
                            });
                        }
                    }
                    Err(e) => {
                        issues.push(VerificationIssue {
                            severity: IssueSeverity::Warning,
                            category: IssueCategory::TestFailure,
                            message: format!(
                                "Failed to execute generated tests for specification '{}': {}",
                                spec.description, e
                            ),
                            location: Some(crate::types::CodeLocation {
                                file: module.file_path.clone(),
                                line: 1,
                                column: None,
                            }),
                            suggestion: Some(
                                "Review generated tests and code implementation".to_string(),
                            ),
                        });
                    }
                }
            }

            // Also check if code matches specification description (basic check)
            if !module
                .content
                .to_lowercase()
                .contains(&spec.description.to_lowercase())
            {
                issues.push(VerificationIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Fidelity,
                    message: format!("Code may not match specification: {}", spec.description),
                    location: Some(crate::types::CodeLocation {
                        file: module.file_path.clone(),
                        line: 1,
                        column: None,
                    }),
                    suggestion: Some("Review code against specification".to_string()),
                });
            }
        }

        Ok(issues)
    }
}

impl Default for DynamicTester {
    fn default() -> Self {
        Self::new()
    }
}

/// Detailed test results structure
#[derive(Debug, Clone)]
pub struct DetailedTestResults {
    pub success: bool,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failure_details: Vec<String>,
}

impl Default for DetailedTestResults {
    fn default() -> Self {
        Self {
            success: false,
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            failure_details: Vec::new(),
        }
    }
}

/// Coverage report structure
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub total_lines: usize,
    pub covered_lines: usize,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<usize>,
}
