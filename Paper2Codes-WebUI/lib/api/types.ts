/**
 * TypeScript types matching backend DTOs
 */

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  meta: ResponseMeta;
}

export interface ResponseMeta {
  timestamp: string;
  request_id: string;
}

export interface PaginatedResponse<T> {
  success: boolean;
  data: T[];
  pagination: PaginationMeta;
  meta: ResponseMeta;
}

export interface PaginationMeta {
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
  has_more: boolean;
  next_cursor?: string | null;
  prev_cursor?: string | null;
}

export interface ApiError {
  success: false;
  error: ErrorDetails;
  meta: ResponseMeta;
}

export interface ErrorDetails {
  code: string;
  message: string;
  details?: unknown;
}

export interface HealthResponse {
  status: string;
  version: string;
  timestamp: string;
}

export interface PromptDetails {
  name: string;
  content: string;
}

export interface ToolInfo {
  name: string;
  description: string;
  requires_model: boolean;
  model_category: string;
  temperature_label: string;
  temperature_value: number;
  primary_prompt: PromptDetails | null;
  supplemental_prompts: PromptDetails[];
}

// Request types
export interface CreatePaperRequest {
  title: string;
  abstract_text: string;
  content?: string;
}

export interface UpdatePaperRequest {
  title?: string;
  abstract_text?: string;
}

export interface PaginationQuery {
  page?: number;
  per_page?: number;
  cursor?: string | null;
  fields?: string;
}

// Domain types (from backend)
export interface Paper {
  id: string;
  title: string;
  abstract_text: string;
  segments: PaperSegment[];
  algorithms: Algorithm[];
  equations: Equation[];
  figures: Figure[];
  tables: Table[];
  references: Reference[];
  metadata: PaperMetadata;
}

export interface PaperSegment {
  id: string;
  section: string;
  content: string;
  segment_type: SegmentType;
  embedding?: number[];
  line_range: [number, number];
}

export type SegmentType =
  | 'Abstract'
  | 'Introduction'
  | 'Methodology'
  | 'Algorithm'
  | 'Experiment'
  | 'Results'
  | 'Conclusion'
  | { Other: string };

export interface Algorithm {
  id: string;
  name: string;
  pseudocode: string;
  description: string;
  line_range: [number, number];
}

export interface Equation {
  id: string;
  latex: string;
  description: string;
  line_range: [number, number];
}

export interface Figure {
  id: string;
  caption: string;
  path?: string;
}

export interface Table {
  id: string;
  caption: string;
  data: string[][];
}

export interface Reference {
  id: string;
  citation: string;
  title?: string;
  authors: string[];
  year?: number;
}

export interface PaperMetadata {
  authors: string[];
  year?: number;
  venue?: string;
  keywords: string[];
  file_path?: string;
}

export interface Repository {
  id: string;
  root_path: string;
  modules: CodeModule[];
  structure: RepositoryStructure;
  metadata: RepositoryMetadata;
}

export interface CodeModule {
  id: string;
  repository_id?: string | null;
  file_path: string;
  language: ProgrammingLanguage;
  content: string;
  ast?: string;
  dependencies: string[];
  tests: Test[];
  status?: ModuleStatus;
  created_at: string;
  updated_at: string;
}

export type ProgrammingLanguage = 'Python' | 'Rust' | { Other: string };

export type ModuleStatus =
  | 'Pending'
  | 'Analyzing'
  | 'Coding'
  | 'Verifying'
  | 'Completed'
  | { Failed: string };

export interface RepositoryStructure {
  directories: string[];
  files: string[];
}

export interface RepositoryMetadata {
  created_at: string;
  updated_at: string;
}

export interface Test {
  id: string;
  name: string;
  code: string;
  expected_output?: string;
}

export interface Task {
  id: string;
  task_type: TaskType;
  description: string;
  context: TaskContext;
  dependencies: string[];
  status: TaskStatus;
  agent_id?: string;
  created_at: string;
  updated_at: string;
}

export type TaskType =
  | 'Planning'
  | { Analysis: { module_id: string } }
  | { Coding: { module_id: string } }
  | { Verification: { module_id?: string } }
  | { Fix: { module_id: string; feedback: string } };

export interface TaskContext {
  paper_id?: string | null;
  module_id?: string | null;
  repository_id?: string | null;
  dependencies: string[];
  metadata?: unknown;
  paper_segments: string[];
  external_refs: Reference[];
  code_context?: string;
  specifications: Specification[];
}

export type TaskStatus = 'Pending' | 'InProgress' | 'Completed' | { Failed: string };

export interface Specification {
  id: string;
  description: string;
  function_signature?: string;
  equations: string[];
  constraints: string[];
}

export type ProcessingStatus =
  | 'pending'
  | 'parsing'
  | 'segmenting'
  | 'extracting'
  | 'classifying'
  | 'embedding'
  | 'completed'
  | 'failed';

export interface ProcessingStatusResponse {
  status: ProcessingStatus;
  progress: number;
  message?: string | null;
  error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface GenerationStatusResponse {
  status: GenerationStatus;
  progress: number;
  repository_id?: string | null;
  message?: string | null;
  error?: string | null;
}

export type GenerationStatus =
  | 'pending'
  | 'planning'
  | 'analyzing'
  | 'coding'
  | 'verifying'
  | 'completed'
  | 'failed';

export interface VerificationStatusResponse {
  status: VerificationStatus;
  progress: number;
  report?: VerificationReport | null;
  message?: string | null;
  error?: string | null;
  created_at: string;
  updated_at: string;
}

export type VerificationStatus =
  | 'pending'
  | 'static_analysis'
  | 'dynamic_testing'
  | 'symbolic_verification'
  | 'completed'
  | 'failed';

export interface VerificationReport {
  overall_score: number;
  static_analysis?: StaticAnalysisResult | null;
  dynamic_testing?: DynamicTestingResult | null;
  symbolic_verification?: SymbolicVerificationResult | null;
  issues: VerificationIssue[];
}

export interface StaticAnalysisResult {
  passed: boolean;
  errors: string[];
  warnings: string[];
}

export interface DynamicTestingResult {
  tests_passed: number;
  tests_failed: number;
  total_tests: number;
  failures: string[];
}

export interface SymbolicVerificationResult {
  verified_properties: number;
  failed_properties: number;
  details: string[];
}

export interface VerificationIssue {
  severity: string;
  message: string;
  module_id?: string | null;
  line_number?: number | null;
}

export interface SearchResult {
  segment: PaperSegment;
  score: number;
  paper_id: string;
  paper_title: string;
}
