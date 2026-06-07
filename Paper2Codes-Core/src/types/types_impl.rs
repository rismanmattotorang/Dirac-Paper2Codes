use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// Paper representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
    pub segments: Vec<PaperSegment>,
    pub algorithms: Vec<Algorithm>,
    pub equations: Vec<Equation>,
    pub figures: Vec<Figure>,
    pub tables: Vec<Table>,
    pub references: Vec<Reference>,
    pub metadata: PaperMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSegment {
    pub id: String,
    pub section: String,
    pub content: String,
    pub segment_type: SegmentType,
    pub embedding: Option<Vec<f32>>,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    Abstract,
    Introduction,
    Methodology,
    Algorithm,
    Experiment,
    Results,
    Conclusion,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Algorithm {
    pub id: String,
    pub name: String,
    pub pseudocode: String,
    pub description: String,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equation {
    pub id: String,
    pub latex: String,
    pub description: String,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figure {
    pub id: String,
    pub caption: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub caption: String,
    pub data: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub id: String,
    pub citation: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMetadata {
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub venue: Option<String>,
    pub keywords: Vec<String>,
    pub file_path: Option<PathBuf>,
}

// Implementation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    pub id: String,
    pub modules: Vec<Module>,
    pub dependencies: DependencyGraph,
    pub experiments: Vec<Experiment>,
    pub metadata: PlanMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub description: String,
    pub module_type: ModuleType,
    pub dependencies: Vec<String>,
    pub language: ProgrammingLanguage,
    pub status: ModuleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    Class,
    Function,
    Script,
    Configuration,
    Test,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Python,
    Rust,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleStatus {
    Pending,
    Analyzing,
    Coding,
    Verifying,
    Completed,
    Failed(String),
}

impl Default for ModuleStatus {
    fn default() -> Self {
        ModuleStatus::Pending
    }
}

fn default_module_timestamp() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub edges: Vec<(String, String)>, // (from, to)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub expected_results: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Repository representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub root_path: PathBuf,
    pub modules: Vec<CodeModule>,
    pub structure: RepositoryStructure,
    pub metadata: RepositoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModule {
    pub id: String,
    #[serde(default)]
    pub repository_id: Option<String>,
    pub file_path: PathBuf,
    pub language: ProgrammingLanguage,
    pub content: String,
    pub ast: Option<String>, // JSON representation of AST
    pub dependencies: Vec<String>,
    pub tests: Vec<Test>,
    #[serde(default)]
    pub status: ModuleStatus,
    #[serde(default = "default_module_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_module_timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStructure {
    pub directories: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub id: String,
    pub name: String,
    pub code: String,
    pub expected_output: Option<String>,
}

// Task system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub task_type: TaskType,
    pub description: String,
    pub context: TaskContext,
    pub dependencies: Vec<Uuid>,
    pub status: TaskStatus,
    pub agent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Planning,
    Analysis { module_id: String },
    Coding { module_id: String },
    Verification { module_id: Option<String> },
    Fix { module_id: String, feedback: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub paper_id: Option<String>,
    pub module_id: Option<String>,
    pub repository_id: Option<String>,
    pub dependencies: Vec<String>,
    pub metadata: serde_json::Value,
    pub paper_segments: Vec<String>,
    pub external_refs: Vec<Reference>,
    pub code_context: Option<String>,
    pub specifications: Vec<Specification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    pub id: String,
    pub description: String,
    pub function_signature: Option<String>,
    pub equations: Vec<String>,
    pub constraints: Vec<String>,
}

// Agent result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResult {
    Plan(ImplementationPlan),
    Analysis(Specification),
    Code(CodeModule),
    Verification(VerificationReport),
}

// Domain context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainContext {
    pub domain: String,
    pub subdomain: Option<String>,
    pub language: ProgrammingLanguage,
    pub libraries: Vec<String>,
    pub tools: Vec<String>,
}

// Verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub module_id: Option<String>,
    pub issues: Vec<VerificationIssue>,
    pub passed: bool,
    pub metrics: VerificationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub location: Option<CodeLocation>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    Syntax,
    Type,
    Logic,
    Performance,
    Fidelity,
    TestFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationMetrics {
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub warnings: usize,
}

// CPR (Contextual Paper Retrieval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedContext {
    pub segment: PaperSegment,
    pub score: f32,
    pub source: ContextSource,
    pub relevance_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSource {
    Paper,
    ExternalReference { citation: String },
    KnowledgeBase { source: String },
}

// Helper implementations
impl Repository {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            root_path,
            modules: Vec::new(),
            structure: RepositoryStructure {
                directories: Vec::new(),
                files: Vec::new(),
            },
            metadata: RepositoryMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }

    pub fn add_module(&mut self, mut module: CodeModule) {
        if module.repository_id.is_none() {
            module.repository_id = Some(self.id.clone());
        }
        module.updated_at = Utc::now();

        self.modules.push(module);
        self.metadata.updated_at = Utc::now();
    }

    pub fn get_module(&self, id: &str) -> Option<&CodeModule> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn get_module_mut(&mut self, id: &str) -> Option<&mut CodeModule> {
        self.modules.iter_mut().find(|m| m.id == id)
    }
}

impl Task {
    pub fn new(task_type: TaskType, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            description,
            context: TaskContext {
                paper_id: None,
                module_id: None,
                repository_id: None,
                dependencies: vec![],
                metadata: serde_json::Value::Null,
                paper_segments: Vec::new(),
                external_refs: Vec::new(),
                code_context: None,
                specifications: Vec::new(),
            },
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn mark_in_progress(&mut self, agent_id: String) {
        self.status = TaskStatus::InProgress;
        self.agent_id = Some(agent_id);
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed(error);
        self.updated_at = Utc::now();
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    pub fn add_dependency(&mut self, from: String, to: String) {
        if !self.edges.contains(&(from.clone(), to.clone())) {
            self.edges.push((from, to));
        }
    }

    pub fn get_dependencies(&self, module_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|(from, _)| from == module_id)
            .map(|(_, to)| to.clone())
            .collect()
    }

    pub fn get_dependents(&self, module_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|(_, to)| to == module_id)
            .map(|(from, _)| from.clone())
            .collect()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ImplementationPlan {
    pub fn specifications(&self) -> Vec<Specification> {
        // Extract specifications from modules
        // This is a placeholder - actual implementation would extract from module descriptions
        Vec::new()
    }
}
