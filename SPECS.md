# Paper2Codes Engine - Rust Implementation Specifications

## Table of Contents
1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Core Components](#core-components)
4. [Rust Crate Selection](#rust-crate-selection)
5. [Data Structures and Types](#data-structures-and-types)
6. [Module Specifications](#module-specifications)
7. [Algorithms Implementation](#algorithms-implementation)
8. [CLI Interface Design](#cli-interface-design)
9. [Error Handling](#error-handling)
10. [Configuration Management](#configuration-management)
11. [Testing Strategy](#testing-strategy)
12. [Performance Considerations](#performance-considerations)
13. [Dependencies and Build](#dependencies-and-build)

---

## Overview

Paper2Codes is a neuro-symbolic Retrieval-Augmented Generation (RAG) framework that automatically generates executable code and documentation from scientific literature. This specification details the Rust implementation of the engine, focusing on performance, type safety, and modularity.

**Note**: This document focuses on the Core engine implementation. For comprehensive API specifications, WebSocket protocols, and integration details between Core and WebUI, see [INTEGRATION.md](../INTEGRATION.md).

### Key Design Principles
- **Type Safety**: Leverage Rust's type system for compile-time guarantees
- **Async/Await**: Use async Rust for concurrent LLM API calls and I/O operations
- **Modularity**: Clear separation of concerns with trait-based abstractions
- **Error Handling**: Comprehensive error types with proper propagation
- **Performance**: Zero-cost abstractions and efficient memory management
- **Extensibility**: Plugin architecture for LLM providers and verification tools

---

## System Architecture

### High-Level Architecture

The Core engine supports both TUI and API server modes:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Paper2Codes-WebUI                        │
│                    (Next.js + Deno Tooling)                     │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   HTTP/REST API     │
                    │   WebSocket API     │
                    └──────────┬──────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    Paper2Codes-Core                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              API Server Layer (axum/warp)                │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │  │
│  │  │   REST     │  │ WebSocket  │  │   Auth     │        │  │
│  │  │  Handlers  │  │  Handlers  │  │  Middleware│        │  │
│  │  └────────────┘  └────────────┘  └────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Coordinator Service                      │  │
│  │  - Task Orchestration                                     │  │
│  │  - State Management                                       │  │
│  │  - Agent Coordination                                     │  │
│  └──────┬──────────┬──────────┬──────────┬─────────────────────┘
│         │          │          │          │
│     ┌───▼───┐  ┌───▼───┐  ┌───▼───┐  ┌───▼────┐
│     │Planning│  │Analysis│  │ Coding │  │Verify │
│     │ Agent  │  │ Agent  │  │ Agents │  │ Agent │
│     └───┬───┘  └───┬───┘  └───┬───┘  └───┬────┘
│         │          │          │          │
│  ┌──────▼──────────▼──────────▼──────────▼─────────────────────┐
│  │              Document Layer                                  │
│  │  - PDF/Text Parser                                           │
│  │  - Domain Classifier                                         │
│  │  - Segment Extraction                                        │
│  └──────┬───────────────────────────────────────────────────────┘
│         │
│  ┌──────▼───────────────────────────────────────────────────────┐
│  │              Execution & Tools Layer                          │
│  │  - Sandbox Runner (Docker/Isolated)                          │
│  │  - Vector Store (Embeddings)                                 │
│  │  - Symbolic Solver Interface                                 │
│  │  - LLM API Clients (OpenRouter, OpenAI, Anthropic, etc.)    │
│  └──────────────────────────────────────────────────────────────┘
│                               │
│                    ┌──────────▼──────────┐
│                    │   TUI Interface     │
│                    │   (ratatui)         │
│                    │   (Optional Mode)   │
│                    └─────────────────────┘
└──────────────────────────────────────────────────────────────────┘
```

For detailed API specifications, endpoint definitions, WebSocket protocols, and integration architecture, see [INTEGRATION.md](../INTEGRATION.md).

### Component Layers

1. **Presentation Layer**: 
   - Ratatui-based TUI for interactive CLI
   - HTTP REST API and WebSocket server for WebUI integration
2. **API Server Layer**: REST endpoints, WebSocket handlers, authentication middleware
3. **Orchestration Layer**: Multi-agent coordination and state management
4. **Agent Layer**: Specialized agents (Planning, Analysis, Coding, Verification)
5. **Document Layer**: Paper parsing, domain classification, content extraction
6. **Execution Layer**: Code execution, verification, symbolic reasoning
7. **Integration Layer**: LLM APIs, external tools, vector databases

---

## Core Components

### 1. Coordinator Service
- **Purpose**: Central orchestration of multi-agent workflow
- **Responsibilities**:
  - Maintain global state (repository, plan, tasks)
  - Route tasks to appropriate agents
  - Manage iteration loops and convergence
  - Handle feedback propagation
  - Coordinate parallel agent execution

### 2. Planning Agent
- **Purpose**: Generate high-level implementation plan from paper
- **Input**: Structured paper content, domain context
- **Output**: Implementation plan (modules, dependencies, experiments)
- **LLM**: GPT-4/GPT-5 or Claude 3 Opus (long context)

### 3. Analysis Agent
- **Purpose**: Extract detailed specifications from paper sections
- **Input**: Paper segments, module descriptions
- **Output**: Refined specifications (function signatures, equations, pseudocode)
- **LLM**: Claude 3 Opus (long context) or Grok-2 (mathematical)

### 4. Coding Agents
- **Purpose**: Generate code for specific modules/functions
- **Input**: Task description, context from CPR, analysis specs
- **Output**: Code modules (files, functions, classes)
- **LLM**: GPT-4 Turbo, Claude Sonnet, or Grok-2 (domain-specific)

### 5. Verification Agent
- **Purpose**: Validate generated code correctness
- **Input**: Repository, specifications
- **Output**: Feedback list (errors, mismatches, suggestions)
- **LLM**: Claude 3 Opus or GPT-4 (reasoning)

### 6. Contextual Paper Retrieval (CPR)
- **Purpose**: Retrieve relevant paper segments and external references
- **Algorithm**: Semantic similarity + keyword matching + structural cues
- **Output**: Ranked context segments

### 7. Symbolically-Augmented Code Verification (SACV)
- **Purpose**: Multi-layered verification (static, dynamic, symbolic)
- **Components**: Linters, type checkers, test runners, SMT solvers

---

## Rust Crate Selection

### Core Dependencies

#### CLI and UI
- **`ratatui`** (v0.26+): Terminal UI framework for interactive CLI
  - Widgets for status displays, progress bars, code viewers
  - Event handling for keyboard/mouse input
  - Layout management for multi-panel interfaces

- **`crossterm`** (v0.28+): Cross-platform terminal manipulation
  - Raw mode, alternate screen, cursor control
  - Event polling for keyboard/mouse

- **`tui-textarea`** (v0.4+): Text area widget for code editing/viewing

#### Async Runtime
- **`tokio`** (v1.35+): Async runtime
  - Async I/O, timers, channels
  - Task spawning and coordination
  - File system operations

#### HTTP and API Clients
- **`reqwest`** (v0.11+): HTTP client with async support
  - OpenRouter API integration
  - OpenAI, Anthropic, xAI API calls
  - Streaming responses for long generations

- **`serde`** (v1.0+) + **`serde_json`**: Serialization/deserialization
  - API request/response structures
  - Configuration files
  - State persistence

#### Document Processing
- **`pdf-extract`** or **`pdf`**: PDF parsing
  - Extract text, metadata, structure
  - Handle LaTeX-rendered papers

- **`regex`** (v1.10+): Pattern matching
  - Algorithm block detection
  - Equation extraction
  - Citation parsing

- **`pulldown-cmark`** (v0.9+): Markdown parsing (for documentation)

#### Vector Embeddings and Search
- **`candle-core`** + **`candle-transformers`**: On-device embeddings
  - Alternative: Use OpenAI/Claude embedding APIs via reqwest
  - Embedding generation for paper segments

- **`qdrant-client`** or **`hnsw-rs`**: Vector database
  - Store and query paper segment embeddings
  - Similarity search for CPR algorithm

#### Database and Storage
- **`surrealdb`** (v1.5+): Multi-model database for persistent storage
  - Document store for papers, repositories, modules, tasks
  - Graph database for dependency relationships
  - Real-time queries for UI updates
  - SQL-like query syntax
  - Official Rust SDK with async/await support

#### Symbolic Reasoning
- **`z3`** (via bindings): SMT solver integration
  - Formal verification of code properties
  - Constraint solving

- **`sympy`** (via Python FFI or Rust port): Computer Algebra System
  - Equation simplification
  - Symbolic math operations

#### Code Analysis and Execution
- **`tree-sitter`** (v0.20+): Code parsing
  - AST generation for multiple languages
  - Syntax validation
  - Code structure analysis

- **`docker-api`** or **`bollard`**: Docker client
  - Sandboxed code execution
  - Isolated test environments

#### Error Handling
- **`anyhow`** (v1.0+): Error handling
  - Contextual error propagation
  - Error chaining

- **`thiserror`** (v1.0+): Custom error types
  - Domain-specific error definitions
  - Error trait implementations

#### Configuration and CLI
- **`clap`** (v4.4+): Command-line argument parsing
  - Subcommands, flags, options
  - Help generation

- **`config`** (v0.14+): Configuration file management
  - TOML/YAML/JSON support
  - Environment variable overrides

#### Logging and Observability
- **`tracing`** (v0.1+): Structured logging
  - Span-based tracing
  - Async-compatible

- **`tracing-subscriber`**: Logging subscribers
  - Console, file, JSON outputs

#### Utilities
- **`uuid`** (v1.6+): Unique identifiers for tasks/agents
- **`chrono`** (v0.4+): Date/time handling
- **`dirs`** (v5.0+): Standard directory locations
- **`tempfile`** (v3.8+): Temporary file/directory management
- **`walkdir`** (v2.4+): Directory traversal
- **`sha2`** (v0.10+): Hashing for caching

---

## Data Structures and Types

### Core Domain Types

```rust
// Paper representation
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

pub struct PaperSegment {
    pub id: String,
    pub section: String,
    pub content: String,
    pub segment_type: SegmentType,
    pub embedding: Option<Vec<f32>>,
    pub line_range: (usize, usize),
}

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

// Implementation plan
pub struct ImplementationPlan {
    pub id: String,
    pub modules: Vec<Module>,
    pub dependencies: DependencyGraph,
    pub experiments: Vec<Experiment>,
    pub metadata: PlanMetadata,
}

pub struct Module {
    pub id: String,
    pub name: String,
    pub description: String,
    pub module_type: ModuleType,
    pub dependencies: Vec<String>,
    pub language: ProgrammingLanguage,
    pub status: ModuleStatus,
}

pub enum ModuleType {
    Class,
    Function,
    Script,
    Configuration,
    Test,
    Documentation,
}

pub enum ProgrammingLanguage {
    Python,
    Rust,
    Other(String),
}

pub enum ModuleStatus {
    Pending,
    Analyzing,
    Coding,
    Verifying,
    Completed,
    Failed(String),
}

// Repository representation
pub struct Repository {
    pub id: String,
    pub root_path: PathBuf,
    pub modules: Vec<CodeModule>,
    pub structure: RepositoryStructure,
    pub metadata: RepositoryMetadata,
}

pub struct CodeModule {
    pub id: String,
    pub file_path: PathBuf,
    pub language: ProgrammingLanguage,
    pub content: String,
    pub ast: Option<AstNode>,
    pub dependencies: Vec<String>,
    pub tests: Vec<Test>,
}

// Task system
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

pub enum TaskType {
    Planning,
    Analysis { module_id: String },
    Coding { module_id: String },
    Verification { module_id: Option<String> },
    Fix { module_id: String, feedback: String },
}

pub struct TaskContext {
    pub paper_segments: Vec<String>,
    pub external_refs: Vec<Reference>,
    pub code_context: Option<String>,
    pub specifications: Vec<Specification>,
}

// Agent system
pub trait Agent: Send + Sync {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse>;
    fn agent_type(&self) -> AgentType;
    fn required_llm(&self) -> LLMProvider;
}

pub enum AgentType {
    Planning,
    Analysis,
    Coding,
    Verification,
}

pub struct AgentContext {
    pub paper: Arc<Paper>,
    pub plan: Option<Arc<ImplementationPlan>>,
    pub repository: Arc<Repository>,
    pub cpr_engine: Arc<dyn CPREngine>,
    pub llm_client: Arc<dyn LLMClient>,
}

pub struct AgentResponse {
    pub task_id: Uuid,
    pub result: AgentResult,
    pub metadata: ResponseMetadata,
}

pub enum AgentResult {
    Plan(ImplementationPlan),
    Analysis(Specification),
    Code(CodeModule),
    Verification(VerificationReport),
}

// LLM integration
pub trait LLMClient: Send + Sync {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse>;
    async fn stream(&self, request: LLMRequest) -> Result<Box<dyn Stream<Item = Result<String>>>>;
    fn provider(&self) -> LLMProvider;
    fn model(&self) -> String;
}

pub enum LLMProvider {
    OpenRouter { model: String },
    OpenAI { model: String },
    Anthropic { model: String },
    XAI { model: String },
}

pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

pub enum MessageRole {
    System,
    User,
    Assistant,
}

// Verification
pub struct VerificationReport {
    pub module_id: Option<String>,
    pub issues: Vec<VerificationIssue>,
    pub passed: bool,
    pub metrics: VerificationMetrics,
}

pub struct VerificationIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub location: Option<CodeLocation>,
    pub suggestion: Option<String>,
}

pub enum IssueSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

pub enum IssueCategory {
    Syntax,
    Type,
    Logic,
    Performance,
    Fidelity,
    TestFailure,
}

pub struct CodeLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

// CPR (Contextual Paper Retrieval)
pub trait CPREngine: Send + Sync {
    async fn retrieve(
        &self,
        task: &Task,
        paper: &Paper,
        repository: &Repository,
        k: usize,
    ) -> Result<Vec<RetrievedContext>>;
}

pub struct RetrievedContext {
    pub segment: PaperSegment,
    pub score: f32,
    pub source: ContextSource,
    pub relevance_reason: String,
}

pub enum ContextSource {
    Paper,
    ExternalReference { citation: String },
    KnowledgeBase { source: String },
}
```

---

## Module Specifications

### 1. `paper2codes::coordinator`

**Purpose**: Central orchestration service

**Key Types**:
- `Coordinator`: Main orchestration service
- `CoordinatorState`: Global state management
- `TaskQueue`: Priority queue for task scheduling

**Key Functions**:
```rust
impl Coordinator {
    pub async fn new(config: CoordinatorConfig) -> Result<Self>;
    pub async fn process_paper(&mut self, paper: Paper) -> Result<Repository>;
    pub async fn execute_iteration(&mut self) -> Result<IterationResult>;
    pub fn get_state(&self) -> &CoordinatorState;
    pub async fn handle_feedback(&mut self, feedback: VerificationReport) -> Result<()>;
}
```

**Responsibilities**:
- Initialize agents and tools
- Maintain task queue and dependencies
- Route tasks to appropriate agents
- Manage iteration loops
- Handle convergence detection
- Coordinate parallel execution

### 2. `paper2codes::agents`

**Purpose**: Agent implementations

**Submodules**:
- `planning`: Planning agent
- `analysis`: Analysis agent
- `coding`: Coding agents (multiple instances)
- `verification`: Verification agent
- `base`: Base agent trait and utilities

**Key Functions**:
```rust
// Planning Agent
impl PlanningAgent {
    pub async fn generate_plan(
        &self,
        paper: &Paper,
        domain: &DomainContext,
    ) -> Result<ImplementationPlan>;
}

// Analysis Agent
impl AnalysisAgent {
    pub async fn analyze_module(
        &self,
        module: &Module,
        paper: &Paper,
        context: &[RetrievedContext],
    ) -> Result<Specification>;
}

// Coding Agent
impl CodingAgent {
    pub async fn generate_code(
        &self,
        task: &Task,
        context: &TaskContext,
    ) -> Result<CodeModule>;
}

// Verification Agent
impl VerificationAgent {
    pub async fn verify(
        &self,
        repository: &Repository,
        specifications: &[Specification],
    ) -> Result<VerificationReport>;
}
```

### 3. `paper2codes::document`

**Purpose**: Paper parsing and processing

**Submodules**:
- `parser`: PDF/text parsing
- `classifier`: Domain classification
- `extractor`: Content extraction (algorithms, equations, etc.)
- `segmenter`: Text segmentation

**Key Functions**:
```rust
pub struct DocumentProcessor {
    pub async fn parse_pdf(path: &Path) -> Result<Paper>;
    pub async fn classify_domain(paper: &Paper) -> Result<DomainContext>;
    pub fn extract_algorithms(paper: &Paper) -> Vec<Algorithm>;
    pub fn extract_equations(paper: &Paper) -> Vec<Equation>;
    pub fn segment_paper(paper: &Paper) -> Vec<PaperSegment>;
}
```

### 4. `paper2codes::retrieval`

**Purpose**: Contextual Paper Retrieval (CPR)

**Key Types**:
- `CPREngine`: Trait for retrieval engines
- `EmbeddingEngine`: Embedding generation
- `VectorStore`: Vector database interface

**Key Functions**:
```rust
impl CPREngine {
    pub async fn retrieve(
        &self,
        task: &Task,
        paper: &Paper,
        repository: &Repository,
        k: usize,
    ) -> Result<Vec<RetrievedContext>>;
    
    fn score_segment(
        &self,
        segment: &PaperSegment,
        task: &Task,
        query_keywords: &[String],
    ) -> f32;
    
    fn boost_algorithm_match(&self, segment: &PaperSegment, task: &Task) -> f32;
    fn deboost_implemented(&self, segment: &PaperSegment, repository: &Repository) -> f32;
}
```

**Algorithm Implementation**:
- Generate query keywords from task
- Compute semantic similarity (embeddings)
- Compute keyword overlap
- Apply boosts for algorithm/formula matches
- Apply de-boosts for already-implemented sections
- Rank and select top-k segments
- Retrieve external references if needed

### 5. `paper2codes::verification`

**Purpose**: Symbolically-Augmented Code Verification (SACV)

**Submodules**:
- `static`: Static analysis (linting, type checking)
- `dynamic`: Dynamic testing (test execution)
- `symbolic`: Symbolic verification (SMT, CAS)

**Key Functions**:
```rust
pub struct SACVPipeline {
    pub async fn verify(
        &self,
        repository: &Repository,
        specifications: &[Specification],
    ) -> Result<VerificationReport>;
    
    async fn static_checks(&self, code: &CodeModule) -> Result<Vec<VerificationIssue>>;
    async fn dynamic_tests(&self, code: &CodeModule, specs: &[Specification]) -> Result<Vec<VerificationIssue>>;
    async fn symbolic_verification(&self, code: &CodeModule, specs: &[Specification]) -> Result<Vec<VerificationIssue>>;
}
```

### 6. `paper2codes::llm`

**Purpose**: LLM API integration

**Submodules**:
- `openrouter`: OpenRouter client
- `openai`: OpenAI client
- `anthropic`: Anthropic client
- `xai`: xAI (Grok) client
- `router`: LLM routing logic

**Key Functions**:
```rust
pub struct LLMRouter {
    pub fn select_llm(&self, task: &Task, context: &AgentContext) -> LLMProvider;
    pub async fn complete(&self, request: LLMRequest, provider: LLMProvider) -> Result<LLMResponse>;
}

impl OpenRouterClient {
    pub async fn complete(&self, request: LLMRequest) -> Result<LLMResponse>;
    pub async fn stream(&self, request: LLMRequest) -> Result<Box<dyn Stream<Item = Result<String>>>>;
}
```

**Routing Strategy**:
- Planning tasks → GPT-4/5 or Claude 3 Opus (long context)
- Analysis tasks → Claude 3 Opus or Grok-2 (math-heavy)
- Coding tasks → GPT-4 Turbo, Claude Sonnet, or Grok-2 (domain-specific)
- Verification tasks → Claude 3 Opus or GPT-4 (reasoning)

### 7. `paper2codes::execution`

**Purpose**: Code execution and sandboxing

**Submodules**:
- `sandbox`: Docker-based sandbox
- `runner`: Code execution runner
- `test_runner`: Test execution

**Key Functions**:
```rust
pub struct SandboxRunner {
    pub async fn execute_code(
        &self,
        code: &CodeModule,
        input: Option<&str>,
    ) -> Result<ExecutionResult>;
    
    pub async fn run_tests(&self, code: &CodeModule) -> Result<TestResults>;
}

pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}
```

### 8. `paper2codes::symbolic`

**Purpose**: Symbolic reasoning integration

**Submodules**:
- `smt`: Z3 SMT solver integration
- `cas`: Computer Algebra System (SymPy via FFI or Rust port)

**Key Functions**:
```rust
pub struct SymbolicSolver {
    pub async fn verify_property(
        &self,
        code: &CodeModule,
        property: &Property,
    ) -> Result<VerificationResult>;
    
    pub async fn simplify_expression(&self, expr: &str) -> Result<String>;
    pub async fn solve_equation(&self, equation: &str) -> Result<Vec<String>>;
}
```

### 9. `paper2codes::ui`

**Purpose**: Ratatui-based CLI interface

**Submodules**:
- `app`: Main application state
- `widgets`: Custom widgets
- `events`: Event handling
- `screens`: Different UI screens

**Key Components**:
- Main dashboard with status overview
- Progress tracking for tasks
- Code viewer for generated modules
- Log viewer for agent outputs
- Interactive task management

**Key Functions**:
```rust
pub struct App {
    pub async fn run(&mut self) -> Result<()>;
    pub fn handle_event(&mut self, event: Event) -> Result<()>;
    pub fn render(&mut self, frame: &mut Frame);
}
```

### 10. `paper2codes::storage`

**Purpose**: Persistent storage abstraction with SurrealDB backend

**Key Types**:
- `Storage`: Trait for storage operations
- `SurrealStorage`: SurrealDB implementation
- `HybridStorage`: SurrealDB + Vector DB hybrid storage

**Key Functions**:
```rust
#[async_trait]
pub trait Storage: Send + Sync {
    // Connection management
    async fn connect(&mut self, config: &StorageConfig) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
    
    // Papers
    async fn save_paper(&self, paper: &Paper) -> Result<()>;
    async fn get_paper(&self, id: &str) -> Result<Option<Paper>>;
    async fn list_papers(&self, filters: PaperFilters) -> Result<Vec<Paper>>;
    async fn search_papers(&self, query: &str) -> Result<Vec<Paper>>;
    async fn delete_paper(&self, id: &str) -> Result<()>;
    
    // Segments
    async fn save_segment(&self, segment: &PaperSegment) -> Result<()>;
    async fn get_segment(&self, id: &str) -> Result<Option<PaperSegment>>;
    async fn get_segments_by_paper(&self, paper_id: &str) -> Result<Vec<PaperSegment>>;
    async fn get_segments_by_ids(&self, ids: &[String]) -> Result<Vec<PaperSegment>>;
    
    // Repositories
    async fn save_repository(&self, repo: &Repository) -> Result<()>;
    async fn get_repository(&self, id: &str) -> Result<Option<Repository>>;
    async fn list_repositories(&self) -> Result<Vec<Repository>>;
    async fn update_repository(&self, repo: &Repository) -> Result<()>;
    
    // Modules
    async fn save_module(&self, module: &CodeModule) -> Result<()>;
    async fn get_module(&self, id: &str) -> Result<Option<CodeModule>>;
    async fn get_modules_by_repository(&self, repo_id: &str) -> Result<Vec<CodeModule>>;
    async fn search_modules(&self, query: &str) -> Result<Vec<CodeModule>>;
    
    // Tasks
    async fn save_task(&self, task: &Task) -> Result<()>;
    async fn get_task(&self, id: &Uuid) -> Result<Option<Task>>;
    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
    async fn get_tasks_by_paper(&self, paper_id: &str) -> Result<Vec<Task>>;
    async fn update_task_status(&self, id: &Uuid, status: TaskStatus) -> Result<()>;
    
    // Dependencies (Graph)
    async fn add_dependency(&self, from: &str, to: &str) -> Result<()>;
    async fn remove_dependency(&self, from: &str, to: &str) -> Result<()>;
    async fn get_dependencies(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependents(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependency_chain(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependency_graph(&self, repo_id: &str) -> Result<DependencyGraph>;
    
    // Documents
    async fn save_document(&self, doc: &Document) -> Result<()>;
    async fn get_document(&self, id: &str) -> Result<Option<Document>>;
    async fn list_documents(&self, filters: DocumentFilters) -> Result<Vec<Document>>;
    async fn search_documents(&self, query: &str) -> Result<Vec<Document>>;
    
    // Vector operations (native SurrealDB)
    async fn save_embedding(&self, segment_id: &str, embedding: Vec<f32>) -> Result<()>;
    async fn get_embedding(&self, segment_id: &str) -> Result<Option<Vec<f32>>>;
    async fn vector_search(&self, query_embedding: Vec<f32>, k: usize, filters: Option<SearchFilters>) -> Result<Vec<SearchResult>>;
    
    // Graph analysis
    async fn analyze_dependencies(&self, repo_id: &str) -> Result<GraphAnalysis>;
    async fn find_circular_dependencies(&self, repo_id: &str) -> Result<Vec<Vec<String>>>;
    async fn get_module_impact(&self, module_id: &str) -> Result<ModuleImpact>;
}

pub struct SurrealStorage {
    db: Surreal<Client>,
    connected: bool,
}

impl SurrealStorage {
    pub async fn new(config: &StorageConfig) -> Result<Self>;
    async fn setup_schema(&self) -> Result<()>;
    async fn migrate(&self) -> Result<()>;
}
```

**Schema Definition**:
```rust
// SurrealDB schema setup
pub async fn setup_schema(db: &Surreal<Client>) -> Result<()> {
    db.query("
        // Papers table
        DEFINE TABLE paper SCHEMAFULL;
        DEFINE FIELD id ON paper TYPE string;
        DEFINE FIELD title ON paper TYPE string;
        DEFINE FIELD abstract_text ON paper TYPE string;
        DEFINE FIELD segments ON paper TYPE array<record<segment>>;
        DEFINE FIELD metadata ON paper TYPE object;
        DEFINE FIELD created_at ON paper TYPE datetime;
        DEFINE FIELD updated_at ON paper TYPE datetime;
        DEFINE INDEX title_search ON paper FIELDS title SEARCH ANALYZER ascii;
        DEFINE INDEX metadata_idx ON paper FIELDS metadata;
        
        // Segments table
        DEFINE TABLE segment SCHEMAFULL;
        DEFINE FIELD id ON segment TYPE string;
        DEFINE FIELD paper_id ON segment TYPE record<paper>;
        DEFINE FIELD section ON segment TYPE string;
        DEFINE FIELD content ON segment TYPE string;
        DEFINE FIELD segment_type ON segment TYPE string;
        DEFINE FIELD embedding ON segment TYPE array<float>;
        DEFINE FIELD line_range ON segment TYPE array<number>;
        DEFINE INDEX paper_id_idx ON segment FIELDS paper_id;
        DEFINE INDEX content_search ON segment FIELDS content SEARCH ANALYZER ascii;
        DEFINE INDEX embedding_idx ON segment FIELDS embedding VECTOR HNSW DIMENSION 1536 DIST COSINE;
        
        // Repositories table
        DEFINE TABLE repository SCHEMAFULL;
        DEFINE FIELD id ON repository TYPE string;
        DEFINE FIELD root_path ON repository TYPE string;
        DEFINE FIELD modules ON repository TYPE array<record<module>>;
        DEFINE FIELD metadata ON repository TYPE object;
        DEFINE FIELD created_at ON repository TYPE datetime;
        DEFINE FIELD updated_at ON repository TYPE datetime;
        
        // Modules table
        DEFINE TABLE module SCHEMAFULL;
        DEFINE FIELD id ON module TYPE string;
        DEFINE FIELD repository_id ON module TYPE record<repository>;
        DEFINE FIELD file_path ON module TYPE string;
        DEFINE FIELD language ON module TYPE string;
        DEFINE FIELD content ON module TYPE string;
        DEFINE FIELD ast ON module TYPE option<object>;
        DEFINE FIELD status ON module TYPE string;
        DEFINE INDEX repository_id_idx ON module FIELDS repository_id;
        DEFINE INDEX file_path_idx ON module FIELDS file_path;
        
        // Tasks table
        DEFINE TABLE task SCHEMAFULL;
        DEFINE FIELD id ON task TYPE string;
        DEFINE FIELD task_type ON task TYPE string;
        DEFINE FIELD description ON task TYPE string;
        DEFINE FIELD status ON task TYPE string;
        DEFINE FIELD paper_id ON task TYPE option<record<paper>>;
        DEFINE FIELD module_id ON task TYPE option<record<module>>;
        DEFINE FIELD agent_id ON task TYPE option<string>;
        DEFINE FIELD created_at ON task TYPE datetime;
        DEFINE FIELD updated_at ON task TYPE datetime;
        DEFINE FIELD completed_at ON task TYPE option<datetime>;
        DEFINE INDEX status_idx ON task FIELDS status;
        DEFINE INDEX created_at_idx ON task FIELDS created_at;
        DEFINE INDEX paper_id_idx ON task FIELDS paper_id;
        
        // Dependencies (Graph edges)
        DEFINE TABLE dependency SCHEMAFULL;
        DEFINE FIELD from ON dependency TYPE record<module>;
        DEFINE FIELD to ON dependency TYPE record<module>;
        DEFINE FIELD dependency_type ON dependency TYPE string;
        DEFINE INDEX from_idx ON dependency FIELDS from;
        DEFINE INDEX to_idx ON dependency FIELDS to;
        
        // Documents table
        DEFINE TABLE document SCHEMAFULL;
        DEFINE FIELD id ON document TYPE string;
        DEFINE FIELD name ON document TYPE string;
        DEFINE FIELD content ON document TYPE string;
        DEFINE FIELD content_type ON document TYPE string;
        DEFINE FIELD metadata ON document TYPE object;
        DEFINE FIELD created_at ON document TYPE datetime;
        DEFINE FIELD updated_at ON document TYPE datetime;
        DEFINE INDEX name_search ON document FIELDS name SEARCH ANALYZER ascii;
    ").await?;
    Ok(())
}
```

### 11. `paper2codes::config`

**Purpose**: Configuration management

**Key Types**:
- `Config`: Main configuration structure
- `LLMConfig`: LLM provider configurations
- `AgentConfig`: Agent-specific settings
- `VerificationConfig`: Verification tool settings
- `StorageConfig`: SurrealDB storage configuration

**Configuration File Format (TOML)**:
```toml
[llm]
openrouter_api_key = "sk-or-..."
default_provider = "openrouter"

[llm.providers.openrouter]
enabled = true
base_url = "https://openrouter.ai/api/v1"
models = ["openai/gpt-4-turbo", "anthropic/claude-3-opus", "x-ai/grok-2"]

[agents]
planning_model = "openai/gpt-4-turbo"
analysis_model = "anthropic/claude-3-opus"
coding_model = "openai/gpt-4-turbo"
verification_model = "anthropic/claude-3-opus"

[verification]
enable_static_checks = true
enable_dynamic_tests = true
enable_symbolic_verification = true

[execution]
sandbox_type = "docker"
timeout_seconds = 300

[storage]
# SurrealDB configuration
enabled = true
connection_string = "ws://localhost:8000"
# Or for embedded: "file://./data"
namespace = "paper2codes"
database = "main"
username = "root"
password = "root"
# Connection pool settings
max_connections = 10
connection_timeout_seconds = 30
# Schema management
auto_migrate = true
# Vector store integration
vector_store_type = "qdrant"  # or "pinecone", "weaviate"
vector_store_url = "http://localhost:6333"
```

---

## Algorithms Implementation

### 1. Contextual Paper Retrieval (CPR)

**Algorithm**: See Algorithm 1 in paper

**Rust Implementation**:
```rust
impl CPREngine {
    pub async fn retrieve(
        &self,
        task: &Task,
        paper: &Paper,
        repository: &Repository,
        k1: usize,  // top-k from paper
        k2: usize,  // top-k from external
    ) -> Result<Vec<RetrievedContext>> {
        // 1. Generate query keywords
        let query_keywords = self.generate_query_keywords(task)?;
        
        // 2. Score paper segments
        let mut scored_segments: Vec<(PaperSegment, f32)> = Vec::new();
        for segment in &paper.segments {
            let mut score = 0.0;
            
            // Semantic similarity
            if let (Some(seg_emb), Some(task_emb)) = 
                (&segment.embedding, &task.embedding) {
                score += self.cosine_similarity(seg_emb, task_emb)?;
            }
            
            // Keyword overlap
            score += self.lambda * self.keyword_overlap(&segment.content, &query_keywords);
            
            // Boost for algorithm/formula matches
            if self.is_algorithm_match(segment, task) {
                score += self.delta;
            }
            
            // De-boost for already implemented
            if self.is_already_implemented(segment, repository) {
                score -= self.gamma;
            }
            
            scored_segments.push((segment.clone(), score));
        }
        
        // 3. Select top-k1 from paper
        scored_segments.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let paper_contexts: Vec<RetrievedContext> = scored_segments
            .into_iter()
            .take(k1)
            .map(|(seg, score)| RetrievedContext {
                segment: seg,
                score,
                source: ContextSource::Paper,
                relevance_reason: String::new(),
            })
            .collect();
        
        // 4. Retrieve external references if needed
        let external_contexts = if self.has_external_references(task) {
            self.retrieve_external_refs(task, &query_keywords, k2).await?
        } else {
            Vec::new()
        };
        
        // 5. Combine and return
        let mut all_contexts = paper_contexts;
        all_contexts.extend(external_contexts);
        Ok(all_contexts)
    }
}
```

### 2. Multi-Agent Orchestration

**Algorithm**: See Algorithm 2 in paper

**Rust Implementation**:
```rust
impl Coordinator {
    pub async fn orchestrate(&mut self, paper: Paper) -> Result<Repository> {
        // 1. Initialize
        let domain = self.document_processor.classify_domain(&paper).await?;
        let mut repository = Repository::new();
        let mut feedback = Vec::new();
        
        // 2. Planning phase
        let plan = self.planning_agent.generate_plan(&paper, &domain).await?;
        let mut task_queue = TaskQueue::from_plan(&plan);
        
        // 3. Main iteration loop
        let max_iterations = self.config.max_iterations;
        for iteration in 0..max_iterations {
            // Process ready tasks
            let ready_tasks = task_queue.get_ready_tasks();
            
            for task in ready_tasks {
                match task.task_type {
                    TaskType::Analysis { module_id } => {
                        let context = self.cpr_engine
                            .retrieve(&task, &paper, &repository, 5, 2)
                            .await?;
                        let spec = self.analysis_agent
                            .analyze_module(&task, &context)
                            .await?;
                        task_queue.attach_specification(&task.id, spec);
                    }
                    TaskType::Coding { module_id } => {
                        let context = self.cpr_engine
                            .retrieve(&task, &paper, &repository, 5, 2)
                            .await?;
                        let code = self.coding_agent
                            .generate_code(&task, &context)
                            .await?;
                        repository.add_module(code);
                    }
                    _ => {}
                }
                task_queue.mark_completed(&task.id);
            }
            
            // 4. Verification phase
            let verification_report = self.verification_agent
                .verify(&repository, &plan.specifications())
                .await?;
            
            if verification_report.issues.is_empty() {
                break; // Success!
            }
            
            // 5. Handle feedback
            feedback.push(verification_report.clone());
            self.handle_verification_feedback(&mut task_queue, &verification_report)?;
            
            // Check convergence
            if self.has_converged(&feedback) {
                break;
            }
        }
        
        Ok(repository)
    }
}
```

### 3. Symbolically-Augmented Code Verification (SACV)

**Algorithm**: See Algorithm 3 in paper

**Rust Implementation**:
```rust
impl SACVPipeline {
    pub async fn verify(
        &self,
        repository: &Repository,
        specifications: &[Specification],
    ) -> Result<VerificationReport> {
        let mut issues = Vec::new();
        
        // Phase 1: Static checks
        for module in &repository.modules {
            let static_issues = self.static_checks(module).await?;
            issues.extend(static_issues);
        }
        
        if issues.iter().any(|i| i.severity == IssueSeverity::Critical) {
            return Ok(VerificationReport {
                issues,
                passed: false,
                ..Default::default()
            });
        }
        
        // Phase 2: Dynamic tests
        for spec in specifications {
            if let Some(test_issues) = self.dynamic_tests(repository, spec).await? {
                issues.extend(test_issues);
            }
        }
        
        // Phase 3: Symbolic verification
        for spec in specifications {
            if let Some(symbolic_issues) = self.symbolic_verification(repository, spec).await? {
                issues.extend(symbolic_issues);
            }
        }
        
        Ok(VerificationReport {
            issues,
            passed: issues.is_empty(),
            metrics: self.compute_metrics(&issues),
        })
    }
}
```

---

## CLI Interface Design

### Ratatui UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Paper2Codes Engine                    [Status: Processing]  │
├──────────────┬──────────────────────────────────────────────┤
│              │                                               │
│   TASKS      │            CODE VIEWER                        │
│              │                                               │
│  [✓] Plan    │  ┌─────────────────────────────────────┐    │
│  [→] Analyze │  │ src/main.rs                         │    │
│  [ ] Code    │  │                                     │    │
│  [ ] Verify  │  │ fn main() {                         │    │
│              │  │     println!("Hello");              │    │
│  Progress:   │  │ }                                   │    │
│  ████░░░░ 40%│  └─────────────────────────────────────┘    │
│              │                                               │
├──────────────┴──────────────────────────────────────────────┤
│ LOGS                                                         │
│ [INFO] Planning agent started...                            │
│ [INFO] Generated 5 modules                                   │
│ [WARN] Module 'optimizer' has dependency issues              │
└─────────────────────────────────────────────────────────────┘
```

### Key Bindings

- `q`: Quit application
- `Tab`: Switch between panels
- `↑/↓`: Navigate task list
- `Enter`: View selected task details
- `r`: Refresh status
- `s`: Save current state
- `l`: Toggle log view
- `?`: Show help

### Widgets

1. **Task List Widget**: Shows all tasks with status indicators
2. **Code Viewer Widget**: Displays generated code with syntax highlighting
3. **Progress Widget**: Shows overall progress and per-module progress
4. **Log Widget**: Real-time log output from agents
5. **Status Bar**: Current operation, iteration count, errors

---

## Error Handling

### Error Type Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum Paper2CodesError {
    #[error("Document processing error: {0}")]
    Document(#[from] DocumentError),
    
    #[error("LLM API error: {0}")]
    LLM(#[from] LLMError),
    
    #[error("Agent execution error: {0}")]
    Agent(#[from] AgentError),
    
    #[error("Verification error: {0}")]
    Verification(#[from] VerificationError),
    
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Model not available: {0}")]
    ModelUnavailable(String),
}
```

### Error Propagation Strategy

- Use `Result<T, Paper2CodesError>` for fallible operations
- Use `anyhow::Result<T>` for internal operations with context
- Provide detailed error messages with context
- Log errors at appropriate levels (error, warn, info)
- Retry transient errors (rate limits, network issues)

---

## Configuration Management

### Configuration Structure

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub llm: LLMConfig,
    pub agents: AgentConfig,
    pub verification: VerificationConfig,
    pub execution: ExecutionConfig,
    pub ui: UIConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LLMConfig {
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub planning_model: String,
    pub analysis_model: String,
    pub coding_model: String,
    pub verification_model: String,
    pub max_iterations: u32,
    pub parallel_tasks: usize,
}
```

### Configuration Loading

- Load from `~/.config/paper2codes/config.toml`
- Support environment variable overrides
- Validate configuration on startup
- Provide default configuration if file missing

---

## Testing Strategy

### Unit Tests

- Test each agent in isolation with mock LLM clients
- Test CPR algorithm with sample paper segments
- Test verification pipeline with known code samples
- Test document parsing with various paper formats

### Integration Tests

- Test full pipeline with small sample papers
- Test multi-agent coordination
- Test error handling and recovery
- Test LLM API integration (with test API keys)

### Property-Based Tests

- Test CPR retrieval properties (monotonicity, completeness)
- Test orchestration convergence properties
- Test verification soundness

### Test Utilities

```rust
pub mod test_utils {
    pub fn mock_paper() -> Paper;
    pub fn mock_llm_client() -> MockLLMClient;
    pub fn sample_code_module() -> CodeModule;
    pub fn create_test_repository() -> Repository;
}
```

---

## Performance Considerations

### Optimization Strategies

1. **Parallel Agent Execution**: Use `tokio::spawn` for independent tasks
2. **Caching**: Cache embeddings, retrieved contexts, LLM responses
3. **Incremental Verification**: Only verify changed modules
4. **Streaming**: Stream LLM responses for better UX
5. **Connection Pooling**: Reuse HTTP connections for LLM APIs
6. **Lazy Loading**: Load paper segments on demand

### Performance Targets

- Paper parsing: < 5 seconds for 20-page PDF
- Planning: < 30 seconds for typical paper
- Code generation: < 2 minutes per module
- Verification: < 10 seconds per module
- Total pipeline: < 15 minutes for typical paper

### Memory Management

- Use `Arc` for shared immutable data (Paper, Plan)
- Use `Rc` for single-threaded shared data
- Avoid cloning large structures unnecessarily
- Use streaming for large LLM responses

---

## Dependencies and Build

### Cargo.toml Structure

```toml
[package]
name = "paper2codes"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# HTTP client
reqwest = { version = "0.11", features = ["json", "stream"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# CLI and UI
ratatui = "0.26"
crossterm = "0.28"
tui-textarea = "0.4"
clap = { version = "4.4", features = ["derive"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Document processing
pdf = "0.8"
regex = "1.10"
pulldown-cmark = "0.9"

# Vector search (choose one)
# qdrant-client = "1.7"
# Or use embedding API via reqwest

# Database storage
surrealdb = { version = "1.5", features = ["kv-mem", "protocol-ws"] }

# Code analysis
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-python = "0.20"

# Execution
bollard = "0.15"  # Docker client

# Utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
tempfile = "3.8"
walkdir = "2.4"
sha2 = "0.10"

# Configuration
config = "0.14"

[dev-dependencies]
tokio-test = "0.4"
mockito = "1.2"
```

### Build Configuration

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

### Feature Flags

```toml
[features]
default = ["ui", "openrouter", "docker", "storage"]
ui = ["ratatui", "crossterm"]
openrouter = []
openai = []
anthropic = []
xai = []
docker = ["bollard"]
z3 = []  # Optional Z3 integration
storage = ["surrealdb"]  # SurrealDB storage backend
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (Weeks 1-2)
- Project setup and dependency management
- Configuration system
- Error handling framework
- Basic CLI structure
- Document parsing (PDF, text)

### Phase 2: LLM Integration (Weeks 3-4)
- LLM client traits and implementations
- OpenRouter integration
- Multi-LLM routing logic
- Request/response handling
- Streaming support

### Phase 3: Document Processing (Weeks 5-6)
- Paper parsing and segmentation
- Domain classification
- Algorithm/equation extraction
- Embedding generation
- Vector store integration

### Phase 4: Agents (Weeks 7-10)
- Agent trait and base implementation
- Planning agent
- Analysis agent
- Coding agents
- Verification agent

### Phase 5: Core Algorithms (Weeks 11-12)
- CPR implementation
- Orchestration loop
- SACV pipeline
- Task queue and dependency management

### Phase 6: Execution and Verification (Weeks 13-14)
- Sandbox implementation
- Code execution
- Static analysis integration
- Dynamic testing
- Symbolic verification (basic)

### Phase 7: UI and Polish (Weeks 15-16)
- Ratatui interface
- Progress tracking
- Code viewer
- Log display
- Error reporting

### Phase 8: Testing and Optimization (Weeks 17-18)
- Comprehensive testing
- Performance optimization
- Documentation
- Example papers and benchmarks

---

## Security Considerations

1. **API Key Management**: Store keys securely, never commit to version control
2. **Sandbox Isolation**: Ensure Docker containers are properly isolated
3. **Input Validation**: Validate all user inputs and paper content
4. **Rate Limiting**: Implement rate limiting for LLM API calls
5. **Resource Limits**: Set memory and CPU limits for code execution
6. **Network Security**: Use HTTPS for all API calls, validate certificates

---

## SurrealDB Integration Specifications

### Overview

SurrealDB serves as the primary persistent storage backend for Paper2Codes, providing multi-model database capabilities including document storage, graph relationships, vector embeddings, and real-time queries. SurrealDB 1.5.0+ includes native vector search with HNSW indexing, allowing us to use a single unified database for all data types.

### Architecture Decision

**Unified Storage Strategy** (Recommended):
- **SurrealDB**: Papers, repositories, modules, tasks, dependencies (graph), documents, **vector embeddings**
- **Redis (optional)**: LLM response cache with TTL

This approach provides:
- Single unified database for all data types
- Native vector similarity search with HNSW indexing
- Unified query interface for vectors, documents, and graphs
- Simplified deployment and maintenance
- Better consistency guarantees

**Optional Hybrid Storage Strategy** (For very large-scale):
- **SurrealDB**: Papers, repositories, modules, tasks, dependencies (graph), documents, metadata
- **Vector Database (Qdrant/Pinecone)**: Embedding vectors and similarity search (for millions+ vectors)
- **Redis (optional)**: LLM response cache with TTL

Use hybrid approach only if:
- You have millions+ vectors requiring extreme performance
- You need specialized vector database features
- You want to distribute vector search load separately

### Storage Features

#### 1. Connection Management

**Backend API**:
```rust
pub struct StorageManager {
    storage: Arc<RwLock<Option<Box<dyn Storage>>>>,
    config: StorageConfig,
}

impl StorageManager {
    // Connect to SurrealDB
    pub async fn connect(&self, config: &StorageConfig) -> Result<()>;
    
    // Disconnect from SurrealDB
    pub async fn disconnect(&self) -> Result<()>;
    
    // Check connection status
    pub fn is_connected(&self) -> bool;
    
    // Test connection
    pub async fn test_connection(&self) -> Result<()>;
    
    // Get connection info
    pub fn get_connection_info(&self) -> Option<ConnectionInfo>;
}
```

**Frontend Features**:
- Connection status indicator in UI
- Connect/disconnect buttons
- Connection configuration dialog
- Connection test functionality
- Connection error notifications
- Auto-reconnect on connection loss

#### 2. Data Management Features

**Papers Management**:
- Create, read, update, delete papers
- Full-text search across papers
- Filter by metadata (authors, year, domain)
- Bulk import/export
- Version history tracking
- Paper relationships (citations, references)

**Repositories Management**:
- Create and manage repositories
- Link repositories to papers
- Repository metadata management
- Repository search and filtering
- Repository cloning/duplication

**Modules Management**:
- CRUD operations for code modules
- Module search by content, path, or language
- Module versioning
- Module metadata (dependencies, tests, status)
- Module content diff viewing

**Tasks Management**:
- Task creation and tracking
- Task status updates
- Task filtering and search
- Task history and audit trail
- Task performance metrics

**Documents Management**:
- Document upload and storage
- Document metadata management
- Document search and retrieval
- Document versioning
- Document relationships

#### 3. Vector Store and Similarity Search

**Native SurrealDB Vector Storage** (Recommended):
```rust
pub struct SurrealVectorStorage {
    db: Surreal<Client>,
}

impl SurrealVectorStorage {
    // Save segment with embedding (all in SurrealDB)
    async fn save_segment_with_embedding(
        &self,
        segment: &PaperSegment,
        embedding: Vec<f32>,
    ) -> Result<()> {
        // Save segment with embedding directly in SurrealDB
        self.db.query("
            CREATE segment:$id CONTENT {
                id: $id,
                paper_id: $paper_id,
                section: $section,
                content: $content,
                segment_type: $segment_type,
                embedding: $embedding,
                line_range: $line_range
            };
        ")
        .bind(("id", &segment.id))
        .bind(("paper_id", &segment.paper_id))
        .bind(("section", &segment.section))
        .bind(("content", &segment.content))
        .bind(("segment_type", &segment.segment_type))
        .bind(("embedding", embedding))  // Vector stored directly
        .bind(("line_range", &segment.line_range))
        .await?;
        
        Ok(())
    }
    
    // Semantic search using SurrealDB's native vector search
    async fn semantic_search(
        &self,
        query_embedding: Vec<f32>,
        k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<PaperSegment>> {
        // Use SurrealDB's vector::similarity function with HNSW index
        let mut query = format!("
            SELECT *, vector::similarity::cosine(embedding, $query_embedding) AS score
            FROM segment
            WHERE embedding != NONE
        ");
        
        // Add filters if provided
        if let Some(filters) = filters {
            if let Some(paper_id) = filters.paper_id {
                query.push_str(&format!(" AND paper_id = '{}'", paper_id));
            }
            if let Some(segment_type) = filters.segment_type {
                query.push_str(&format!(" AND segment_type = '{}'", segment_type));
            }
        }
        
        query.push_str(&format!(" ORDER BY score DESC LIMIT {}", k));
        
        let result: Vec<(PaperSegment, f32)> = self.db
            .query(&query)
            .bind(("query_embedding", query_embedding))
            .await?
            .take(0)?;
        
        Ok(result.into_iter().map(|(seg, _)| seg).collect())
    }
}
```

**Optional Hybrid Vector Storage** (For very large-scale):
```rust
pub struct HybridStorage {
    surreal: Arc<SurrealStorage>,
    vector_store: Arc<dyn VectorStore>,  // Optional external vector DB
}

impl HybridStorage {
    // Use external vector DB only if configured
    async fn semantic_search(
        &self,
        query_embedding: Vec<f32>,
        k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<PaperSegment>> {
        // Check if using external vector DB
        if self.vector_store.is_some() {
            // Use hybrid approach
            // ... (previous hybrid implementation)
        } else {
            // Use SurrealDB native vector search
            self.surreal.semantic_search(query_embedding, k, filters).await
        }
    }
}
```

**Frontend Features**:
- Semantic search interface
- Search result ranking display
- Similarity score visualization
- Filter by paper, section, or metadata
- Search history
- Saved searches

#### 4. Document Storage

**Document Storage Features**:
```rust
pub struct DocumentStorage {
    storage: Arc<dyn Storage>,
}

impl DocumentStorage {
    // Upload document
    async fn upload_document(
        &self,
        name: &str,
        content: Vec<u8>,
        content_type: &str,
        metadata: DocumentMetadata,
    ) -> Result<String>;
    
    // Retrieve document
    async fn get_document(&self, id: &str) -> Result<Document>;
    
    // Search documents
    async fn search_documents(
        &self,
        query: &str,
        filters: DocumentFilters,
    ) -> Result<Vec<Document>>;
    
    // Document versioning
    async fn create_version(&self, doc_id: &str) -> Result<String>;
    async fn list_versions(&self, doc_id: &str) -> Result<Vec<DocumentVersion>>;
    async fn get_version(&self, doc_id: &str, version: &str) -> Result<Document>;
}
```

**Frontend Features**:
- Document upload interface
- Document viewer
- Document metadata editor
- Document search
- Document version history
- Document download/export

#### 5. Graph-Based Analysis Features

**Dependency Graph Management**:
```rust
pub struct GraphAnalyzer {
    storage: Arc<dyn Storage>,
}

impl GraphAnalyzer {
    // Build dependency graph
    async fn build_dependency_graph(
        &self,
        repo_id: &str,
    ) -> Result<DependencyGraph>;
    
    // Find circular dependencies
    async fn find_circular_dependencies(
        &self,
        repo_id: &str,
    ) -> Result<Vec<Vec<String>>>;
    
    // Get module impact analysis
    async fn analyze_module_impact(
        &self,
        module_id: &str,
    ) -> Result<ModuleImpact> {
        let dependents = self.storage.get_dependents(module_id).await?;
        let dependencies = self.storage.get_dependencies(module_id).await?;
        let chain = self.storage.get_dependency_chain(module_id).await?;
        
        Ok(ModuleImpact {
            direct_dependents: dependents.len(),
            transitive_dependents: chain.len(),
            direct_dependencies: dependencies.len(),
            affected_modules: chain,
        })
    }
    
    // Get dependency statistics
    async fn get_dependency_stats(
        &self,
        repo_id: &str,
    ) -> Result<DependencyStats> {
        let graph = self.build_dependency_graph(repo_id).await?;
        
        Ok(DependencyStats {
            total_modules: graph.nodes.len(),
            total_dependencies: graph.edges.len(),
            max_depth: graph.max_depth(),
            average_dependencies: graph.average_dependencies(),
            isolated_modules: graph.isolated_nodes().len(),
        })
    }
    
    // Visualize dependency graph
    async fn export_graph_visualization(
        &self,
        repo_id: &str,
        format: GraphFormat,
    ) -> Result<Vec<u8>>;
}
```

**Frontend Features**:
- Interactive dependency graph visualization
- Graph layout algorithms (hierarchical, force-directed)
- Module impact analysis display
- Circular dependency detection and highlighting
- Dependency statistics dashboard
- Graph export (PNG, SVG, DOT)
- Graph filtering and search
- Module relationship explorer

**Graph Analysis Queries**:
```sql
-- Get all dependencies of a module
SELECT ->depends_on->module FROM module:mod_123;

-- Get all dependents of a module
SELECT <-depends_on<-module FROM module:mod_123;

-- Get full dependency chain
SELECT ->depends_on->->depends_on->module FROM module:mod_123;

-- Find modules with no dependencies
SELECT * FROM module WHERE NONE(->depends_on->module);

-- Find modules with most dependents
SELECT id, count(<-depends_on<-module) AS dependent_count 
FROM module 
ORDER BY dependent_count DESC 
LIMIT 10;

-- Detect circular dependencies (requires recursive query)
SELECT * FROM module WHERE id IN (
    SELECT VALUE id FROM (
        SELECT ->depends_on->module AS deps FROM module:mod_123
        WHERE deps.id = 'mod_123'
    )
);
```

### Real-Time Features

**Live Queries**:
```rust
// Real-time task updates
pub async fn subscribe_to_tasks(
    &self,
    filters: TaskFilters,
) -> Result<Box<dyn Stream<Item = TaskUpdate>>> {
    let mut stream = self.db
        .select("task")
        .live()
        .await?;
    
    // Filter and transform stream
    Ok(Box::new(stream.filter_map(|notification| {
        // Process and filter notifications
        Some(TaskUpdate::from_notification(notification))
    })))
}
```

**Frontend Real-Time Features**:
- Live task status updates
- Real-time progress tracking
- Live repository updates
- Real-time collaboration (future)

### Backend API Endpoints

**Storage Management**:
- `POST /api/storage/connect` - Connect to SurrealDB
- `POST /api/storage/disconnect` - Disconnect from SurrealDB
- `GET /api/storage/status` - Get connection status
- `POST /api/storage/test` - Test connection

**Data Management**:
- `GET /api/papers` - List papers
- `POST /api/papers` - Create paper
- `GET /api/papers/:id` - Get paper
- `PUT /api/papers/:id` - Update paper
- `DELETE /api/papers/:id` - Delete paper
- `GET /api/papers/search` - Search papers

- `GET /api/repositories` - List repositories
- `POST /api/repositories` - Create repository
- `GET /api/repositories/:id` - Get repository
- `PUT /api/repositories/:id` - Update repository

- `GET /api/modules` - List modules
- `POST /api/modules` - Create module
- `GET /api/modules/:id` - Get module
- `GET /api/modules/search` - Search modules

- `GET /api/tasks` - List tasks
- `POST /api/tasks` - Create task
- `GET /api/tasks/:id` - Get task
- `PUT /api/tasks/:id/status` - Update task status

**Vector Search**:
- `POST /api/search/semantic` - Semantic search (using SurrealDB native vector search)
- `POST /api/search/hybrid` - Hybrid search (semantic + keyword)
- `POST /api/search/vector` - Direct vector similarity search

**Graph Analysis**:
- `GET /api/repositories/:id/graph` - Get dependency graph
- `GET /api/repositories/:id/graph/analysis` - Get graph analysis
- `GET /api/repositories/:id/graph/circular` - Find circular dependencies
- `GET /api/modules/:id/impact` - Get module impact

**Documents**:
- `POST /api/documents` - Upload document
- `GET /api/documents/:id` - Get document
- `GET /api/documents` - List documents
- `GET /api/documents/search` - Search documents

### Frontend UI Components

**Connection Management UI**:
- Connection status widget
- Connection configuration modal
- Connection test button
- Connection error alerts

**Data Management UI**:
- Papers list view with search and filters
- Paper detail view with segments
- Repository browser
- Module explorer
- Task dashboard

**Search UI**:
- Semantic search bar
- Search results with similarity scores
- Search filters panel
- Search history

**Graph Visualization UI**:
- Interactive graph canvas
- Graph controls (zoom, pan, layout)
- Module detail panel
- Graph statistics panel
- Export options

**Document Management UI**:
- Document upload area
- Document list view
- Document viewer
- Document metadata editor

### Implementation Phases

**Phase 1: Core Storage (Weeks 1-2)**
- SurrealDB connection and configuration
- Basic CRUD operations for papers, repositories, modules, tasks
- Schema setup and migration
- Connection management UI

**Phase 2: Advanced Queries (Weeks 3-4)**
- Full-text search implementation
- Complex query support
- Graph relationship queries
- Search UI components

**Phase 3: Vector Store Integration (Weeks 5-6)**
- SurrealDB native vector storage implementation
- HNSW index setup and configuration
- Semantic search using SurrealDB vector functions
- Search result ranking
- Optional: External vector DB integration (if needed for scale)

**Phase 4: Graph Analysis (Weeks 7-8)**
- Dependency graph building
- Circular dependency detection
- Impact analysis
- Graph visualization UI

**Phase 5: Real-Time Features (Weeks 9-10)**
- Live query subscriptions
- Real-time UI updates
- WebSocket integration
- Event streaming

**Phase 6: Document Storage (Weeks 11-12)**
- Document upload and storage
- Document versioning
- Document search
- Document management UI

### Performance Considerations

**SurrealDB Performance**:
- Index optimization for frequently queried fields
- Connection pooling for concurrent requests
- Query optimization and caching
- Batch operations for bulk inserts

**Hybrid Storage Performance**:
- Parallel queries to SurrealDB and vector store
- Result caching for common queries
- Lazy loading for large documents
- Pagination for large result sets

**Scalability**:
- Horizontal scaling with SurrealDB clustering
- Read replicas for query distribution
- Caching layer for hot data
- Async operations for non-blocking I/O

## Enhanced Specifications from Reference Projects

This section incorporates advanced methods, techniques, and algorithms from the reference implementations (paper2code-rs and paperbench-rs) that can significantly enhance the main Paper2Codes project.

### 1. Domain-Aware Code Generation System

**Source**: paper2code-rs

**Overview**: Automatic detection of computational domains in research papers with domain-specific code generation optimizations.

**Implementation Specifications**:

```rust
pub enum ComputationalDomain {
    NumericalComputing,
    ChipDesign,
    Bioinformatics,
    QuantumComputing,
    DigitalTwin,
    ClassicalML,
    DeepLearning,
    Transformers,
    ComputationalPhysics,
    ComputationalBiology,
    ComputationalFinance,
    SupplyChain,
    Logistics,
    General,
}

pub struct DomainDetector {
    domain_keywords: HashMap<ComputationalDomain, Vec<String>>,
    regex_patterns: HashMap<ComputationalDomain, Vec<Regex>>,
    llm_client: Option<Arc<dyn LLMClient>>,
    min_confidence: f64,
}
```

**Key Features**:
- **Hybrid Detection**: Combines rule-based keyword/regex matching with LLM-based detection for accuracy
- **Domain-Specific Preferences**: Each domain has preferred languages, frameworks, and code templates
- **Confidence Scoring**: Domain detection includes confidence metrics to handle ambiguous papers
- **Caching**: Memoization of domain detection results for performance

**Domain-Specific Code Templates**:
- Each domain provides code templates for common patterns
- Templates include proper imports, library usage, and domain conventions
- Automatic framework selection based on detected domain

**Integration Points**:
- Planning Agent: Use domain information to generate appropriate implementation plan
- Analysis Agent: Apply domain-specific analysis techniques
- Coding Agent: Use domain templates and frameworks for code generation
- Verification Agent: Apply domain-specific validation rules

### 2. Advanced Multi-LLM Strategy System

**Source**: paper2code-rs

**Overview**: Sophisticated LLM routing with task-specific preferences, adaptive strategies, and intelligent result merging.

**Implementation Specifications**:

```rust
pub enum LlmStrategy {
    OpenAiOnly,
    ClaudeOnly,
    OpenAiFirstClaudeSecond,
    ClaudeFirstOpenAiSecond,
    CompareAndMerge,
    Adaptive {
        code_detection: AdaptivePreference,
        code_improvement: AdaptivePreference,
        code_generation: AdaptivePreference,
        documentation: AdaptivePreference,
        bug_fixing: AdaptivePreference,
        performance_optimization: AdaptivePreference,
        safety_enhancement: AdaptivePreference,
        test_generation: AdaptivePreference,
        domain_detection: AdaptivePreference,
    },
}

pub struct AdaptivePreference {
    pub openai_weight: f64,
    pub claude_weight: f64,
}
```

**Key Features**:
- **Task-Specific Routing**: Different LLMs for different tasks (e.g., Claude for analysis, OpenAI for coding)
- **Compare-and-Merge**: Generate with both LLMs and intelligently merge best parts
- **Dynamic Learning**: Adaptive weights based on historical success rates
- **Fallback Mechanisms**: Automatic fallback if primary LLM fails
- **Language-Aware Selection**: Adjust LLM preference based on target programming language

**Result Merging Algorithm**:
1. Compare code outputs from multiple LLMs
2. Use LLM to analyze and merge best features
3. Prefer longer/more detailed implementations when quality is unclear
4. Combine error handling, documentation, and implementation from different sources

**Configuration Example**:
```toml
[llm_strategy]
strategy_type = "adaptive"
code_detection_preference = "prefer_claude"
code_generation_preference = "prefer_openai"
analysis_preference = "prefer_claude"
```

### 3. Error Recovery and Resilience Mechanisms

**Source**: paper2code-rs

**Overview**: Comprehensive error recovery strategies for robust operation in production environments.

**Implementation Specifications**:

```rust
pub enum RecoveryStrategy {
    RetryWithBackoff {
        max_retries: usize,
        base_delay_ms: u64,
        max_delay_ms: u64,
    },
    Fallback,
    Skip,
    Fail,
}

pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T, Paper2CodesError>;
```

**Key Features**:
- **Exponential Backoff**: Intelligent retry timing to avoid overwhelming services
- **Operation-Specific Recovery**: Different strategies for PDF extraction, LLM calls, file I/O
- **Graceful Degradation**: Continue operation with reduced functionality when components fail
- **Rate Limit Awareness**: Special handling for API rate limits with extended backoff
- **Detailed Error Context**: Rich error messages with recovery attempt information

**Specialized Recovery**:
- PDF extraction: Retry with alternative parsing methods
- LLM API calls: Exponential backoff with rate limit detection
- File operations: Retry with permissions checking
- Network operations: Automatic retry with timeout adjustments

### 4. Performance Optimization System

**Source**: paper2code-rs

**Overview**: Advanced performance optimizations including adaptive concurrency, smart rate limiting, memory profiling, and caching.

**Implementation Specifications**:

```rust
pub enum MemoryProfile {
    Low,
    Standard,
    High,
    Auto,  // Automatically detects system resources
}

pub struct AdaptiveConcurrency {
    current_limit: Arc<RwLock<usize>>,
    min_limit: usize,
    max_limit: usize,
    success_threshold: f64,
    recent_results: Arc<RwLock<Vec<bool>>>,
}

pub struct SmartRateLimiter {
    max_rpm: u32,
    tokens_per_minute: u32,
    request_history: Arc<RwLock<VecDeque<Instant>>>,
    adaptive_factor: Arc<RwLock<f64>>,
}
```

**Key Features**:

**Adaptive Concurrency Control**:
- Dynamically adjusts concurrent task limit based on success rate
- Prevents system overload while maximizing throughput
- Window-based performance tracking

**Smart Rate Limiting**:
- Token-aware rate limiting (tracks both requests and tokens)
- Adaptive rate reduction on errors
- Gradual recovery after error periods
- Jitter to avoid thundering herd problems

**Memory Management**:
- Automatic memory profile detection
- Adaptive chunk sizing for text processing
- Memory-aware parallel task limits
- Resource-based configuration

**Intelligent Caching**:
- LRU cache with metadata (access count, creation time)
- TTL-based expiration
- Compilation result caching
- Embedding cache for repeated queries

**Optimized Text Processing**:
- Paragraph-aware chunking for better context preservation
- Parallel processing using Rayon
- Adaptive batch sizing for LLM operations

### 5. GitHub Repository Integration

**Source**: paper2code-rs

**Overview**: Automatic detection and integration of GitHub repositories mentioned in papers to enhance code generation context.

**Implementation Specifications**:

```rust
pub struct GitHubClient {
    client: Client,
    token: Option<String>,
    rate_limit_remaining: u32,
}

pub struct RepositoryInfo {
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub readme: Option<String>,
    pub requirements: Option<Vec<String>>,
    // ... other fields
}
```

**Key Features**:
- **URL Detection**: Multiple regex patterns for detecting GitHub URLs in papers
- **Repository Metadata**: Fetch repository info, README, and dependencies
- **Dependency Extraction**: Parse requirements.txt, Cargo.toml, package.json, etc.
- **Context Enhancement**: Add repository context to code generation prompts
- **Rate Limit Management**: Intelligent handling of GitHub API rate limits
- **Authentication Support**: Optional token for higher rate limits

**Integration Flow**:
1. Extract GitHub URLs from paper text
2. Fetch repository metadata and README
3. Extract dependency files (requirements.txt, Cargo.toml, etc.)
4. Add context to LLM prompts for better code generation
5. Use repository structure as guidance for code organization

**Configuration**:
```toml
[github]
token = "optional_github_token"
enable = true
max_repos = 3
rate_limit_requests_per_hour = 60
```

### 6. Iterative Agent Pattern with ReAct Loop

**Source**: paperbench-rs

**Overview**: ReAct-style agent implementation with continuous improvement and tool-based execution.

**Implementation Specifications**:

```rust
pub struct IterativeAgent {
    model: Model,
    system_prompt: String,
    continue_message: String,
    tool_manager: Arc<Mutex<ToolManager>>,
    max_steps: usize,
    metrics: Arc<Mutex<AgentMetrics>>,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    async fn execute(&self, args: &str) -> Result<String>;
}
```

**Key Features**:
- **ReAct Loop**: Reason → Act (Tool) → Observe → Repeat
- **No Early Termination**: Continues working for full time allocation
- **Tool-Based Execution**: Modular tools for file reading, code execution, web browsing
- **Context Window Management**: Automatic pruning of conversation history
- **Metrics Collection**: Track API calls, token usage, tool usage statistics

**Available Tools**:
- **FileReaderTool**: Read and analyze code files
- **BashTool**: Execute shell commands in sandboxed environment
- **PythonTool**: Execute Python code with proper environment
- **WebBrowserTool**: Search web for additional context (optional)

**Agent Behavior**:
- Constantly prompts to continue working until time limit
- Uses tools to explore codebase and execute tests
- Maintains conversation context for coherent reasoning
- Collects detailed metrics for performance analysis

### 7. Hierarchical Rubric Evaluation System

**Source**: paperbench-rs

**Overview**: Sophisticated evaluation system using hierarchical rubrics with leaf-node grading and weighted aggregation.

**Implementation Specifications**:

```rust
pub enum RequirementType {
    CodeDevelopment,
    Execution,
    ResultMatch,
}

pub struct SimpleJudge {
    model: Model,
    max_files: usize,
    metrics: Arc<Mutex<JudgeMetrics>>,
}

pub struct GradingResult {
    pub score: f64,
    pub node_results: Vec<NodeGradingResult>,
    pub metadata: GradingMetadata,
}
```

**Key Features**:
- **Hierarchical Rubric**: Tree structure of requirements
- **Leaf Node Grading**: Grade only leaf nodes, aggregate upward
- **File Relevance Ranking**: Select most relevant files for each requirement
- **Requirement Type Awareness**: Different evaluation strategies for code, execution, results
- **Context Preservation**: Include ancestor requirements for better understanding
- **Detailed Explanations**: Three-part grading (Expectations, Reality, Score)

**Grading Process**:
1. Traverse rubric to find all leaf nodes
2. For each leaf node:
   - Determine requirement type (CodeDevelopment, Execution, ResultMatch)
   - Select relevant files based on type
   - Rank files by semantic relevance
   - Build context with ancestors, paper, files, execution logs
   - Grade with LLM judge
3. Aggregate scores using weighted averages up the tree
4. Generate comprehensive grading report

**File Selection Strategy**:
- **CodeDevelopment**: Source code files, README, configuration
- **Execution**: reproduce.sh, reproduce.log, source files
- **ResultMatch**: Output files (images, data), execution logs

### 8. File Ranking by Semantic Relevance

**Source**: paperbench-rs

**Overview**: Intelligent file ranking system to select most relevant files for evaluation context.

**Implementation Specifications**:

```rust
pub async fn rank_files_by_relevance(
    files: &[PathBuf],
    node: &RubricNode,
    model: &Model,
    max_files: usize,
) -> Result<Vec<PathBuf>>;
```

**Key Features**:
- **Semantic Similarity**: Use embeddings to find files relevant to requirement
- **Requirement Matching**: Score files based on how well they match rubric node
- **Context Size Optimization**: Limit files to fit within LLM context window
- **Diversity**: Ensure selected files cover different aspects when possible

**Ranking Algorithm**:
1. Generate embedding for rubric requirement
2. Generate embeddings for all candidate files (or use cached)
3. Calculate cosine similarity scores
4. Rank files by relevance score
5. Select top-k files (diversity-aware if beneficial)

### 9. Comprehensive Benchmark Metrics System

**Source**: paperbench-rs

**Overview**: Detailed metrics collection and analysis for evaluating system performance across papers and runs.

**Implementation Specifications**:

```rust
pub struct BenchmarkMetrics {
    pub overall_average_score: f64,
    pub requirement_type_scores: RequirementTypeScores,
    pub score_std_dev: f64,
    pub num_papers: usize,
    pub total_runs: usize,
    pub top_papers: Vec<TopPaperScore>,
    pub bottom_papers: Vec<TopPaperScore>,
}

pub struct AgentMetrics {
    pub total_tokens: usize,
    pub api_calls: usize,
    pub tool_calls: usize,
    pub tool_usage: HashMap<String, usize>,
}
```

**Key Features**:
- **Aggregate Statistics**: Mean, standard deviation, distribution analysis
- **Requirement Type Breakdown**: Separate scores for code, execution, results
- **Top/Bottom Identification**: Highlight best and worst performing papers
- **Agent Performance**: Track token usage, API calls, tool usage per agent
- **Time Tracking**: Measure time taken per phase, per paper, per run

**Metrics Collection Points**:
- Agent execution: tokens, API calls, tool usage, time
- Verification: issues found, test results, score breakdown
- Overall: success rate, average quality, convergence rate

### 10. Docker-Based Reproduction System

**Source**: paperbench-rs

**Overview**: Isolated execution environments using Docker for safe code reproduction and testing.

**Implementation Specifications**:

```rust
pub struct ReproductionSystem {
    docker_client: Docker,
}

pub async fn reproduce_submission(
    &self,
    submission_path: &Path,
) -> Result<ReproductionResult>;
```

**Key Features**:
- **Isolation**: Each submission runs in isolated Docker container
- **Script Execution**: Execute reproduce.sh script provided by agent
- **Output Capture**: Capture stdout, stderr, and logs
- **File Tracking**: Monitor file changes and outputs
- **Resource Limits**: CPU, memory, and time limits per container
- **Cleanup**: Automatic container cleanup after execution

**Reproduction Process**:
1. Create Docker container with base image
2. Mount submission directory
3. Execute reproduce.sh script
4. Capture all outputs and logs
5. Analyze file modifications
6. Generate reproduction report
7. Clean up container

**Configuration**:
```toml
[reproduction]
docker_image = "python:3.11"
timeout_seconds = 3600
memory_limit_mb = 4096
cpu_limit = "2.0"
```

### Integration Recommendations

**Priority 1 (High Impact, Moderate Effort)**:
1. Domain-Aware Code Generation - Significantly improves code quality
2. Advanced Multi-LLM Strategy - Better results with existing infrastructure
3. Error Recovery Mechanisms - Essential for production reliability

**Priority 2 (High Impact, High Effort)**:
4. Performance Optimization System - Improves scalability
5. GitHub Repository Integration - Enhances context for code generation
6. Iterative Agent Pattern - More capable agent behavior

**Priority 3 (Moderate Impact, Moderate Effort)**:
7. Hierarchical Rubric Evaluation - Better verification quality
8. File Ranking by Relevance - Optimizes context window usage
9. Benchmark Metrics System - Essential for evaluation and improvement

**Priority 4 (Moderate Impact, Low Effort)**:
10. Docker-Based Reproduction - Safe execution environment

### Configuration Enhancements

Add to `config.toml`:

```toml
# Domain detection
[domain_detection]
enabled = true
use_llm = true
min_confidence = 0.6
cache_results = true

# Multi-LLM strategy
[llm_strategy]
default = "adaptive"

[llm_strategy.adaptive]
code_detection = { openai_weight = 0.3, claude_weight = 0.7 }
code_generation = { openai_weight = 0.7, claude_weight = 0.3 }
analysis = { openai_weight = 0.2, claude_weight = 0.8 }
verification = { openai_weight = 0.4, claude_weight = 0.6 }

# Error recovery
[error_recovery]
max_retries = 3
base_delay_ms = 1000
max_delay_ms = 10000
enable_fallback = true

# Performance optimization
[performance]
memory_profile = "auto"  # or "low", "standard", "high"
max_concurrent_tasks = "auto"
adaptive_concurrency = true
enable_caching = true
cache_size = 1000
cache_ttl_seconds = 3600

# GitHub integration
[github]
enabled = true
token = "optional"
max_repos = 3
rate_limit_requests_per_hour = 60

# Benchmark metrics
[benchmark]
track_metrics = true
save_detailed_results = true
output_directory = "./benchmark_results"
```

### Module Structure Additions

```
src/
├── domain/
│   ├── detector.rs          # Domain detection implementation
│   ├── templates.rs         # Domain-specific code templates
│   └── mod.rs
├── llm/
│   ├── strategy.rs          # Enhanced with adaptive strategies
│   ├── merge.rs             # Result merging logic
│   └── mod.rs
├── recovery/
│   ├── retry.rs             # Retry mechanisms
│   ├── fallback.rs          # Fallback strategies
│   └── mod.rs
├── performance/
│   ├── concurrency.rs       # Adaptive concurrency control
│   ├── rate_limiter.rs      # Smart rate limiting
│   ├── cache.rs             # Caching mechanisms
│   └── mod.rs
├── github/
│   ├── client.rs            # GitHub API client
│   ├── extractor.rs         # URL extraction
│   └── mod.rs
├── agents/
│   ├── iterative.rs         # Iterative agent implementation
│   ├── tools.rs             # Agent tools
│   └── mod.rs
├── evaluation/
│   ├── judge.rs             # Hierarchical rubric judge
│   ├── file_ranking.rs      # File relevance ranking
│   ├── metrics.rs           # Benchmark metrics
│   └── mod.rs
└── reproduction/
    ├── docker.rs            # Docker-based reproduction
    └── mod.rs
```

## Future Extensions

1. **Plugin System**: Allow custom agents and verification tools
2. **Multi-language Support**: Extend beyond Python/Rust
3. **Distributed Execution**: Support for distributed agent execution
4. **Web Interface**: Optional web UI alongside CLI
5. **Advanced Symbolic Reasoning**: Enhanced SMT/CAS integration
6. **Learning from Feedback**: Improve agent prompts based on verification results
7. **Multi-user Support**: User authentication and authorization
8. **Collaboration Features**: Real-time collaborative editing
9. **Advanced Analytics**: Usage statistics and performance metrics
10. **Backup and Recovery**: Automated backup and restore capabilities

---

## Conclusion

This specification provides a comprehensive blueprint for implementing the Paper2Codes engine in Rust. The design emphasizes:

- **Type Safety**: Leveraging Rust's type system for correctness
- **Performance**: Async/await for concurrent operations
- **Modularity**: Clear separation of concerns
- **Extensibility**: Trait-based abstractions for flexibility
- **User Experience**: Rich CLI interface with Ratatui

The implementation should follow this specification closely while remaining adaptable to new requirements and improvements based on empirical results.

