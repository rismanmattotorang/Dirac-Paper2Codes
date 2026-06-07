use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct TestResults {
    pub passed: bool,
    pub total: usize,
    pub failed: usize,
    pub output: String,
    pub errors: String,
}
