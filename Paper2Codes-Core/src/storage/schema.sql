-- Comprehensive SurrealDB Schema for Paper2Codes
-- This schema defines all tables, fields, indexes, and vector indexes

-- Papers table
DEFINE TABLE paper SCHEMAFULL;
DEFINE FIELD id ON paper TYPE string ASSERT $value != NONE;
DEFINE FIELD owner ON paper TYPE option<string>;
DEFINE FIELD title ON paper TYPE string ASSERT $value != NONE;
DEFINE FIELD abstract_text ON paper TYPE string;
DEFINE FIELD segments ON paper TYPE array<record<segment>>;
DEFINE FIELD algorithms ON paper TYPE array<object>;
DEFINE FIELD equations ON paper TYPE array<object>;
DEFINE FIELD figures ON paper TYPE array<object>;
DEFINE FIELD tables ON paper TYPE array<object>;
DEFINE FIELD references ON paper TYPE array<object>;
DEFINE FIELD metadata ON paper TYPE object;
DEFINE FIELD status ON paper TYPE string DEFAULT 'uploaded';
DEFINE FIELD created_at ON paper TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON paper TYPE datetime DEFAULT time::now();
DEFINE INDEX title_search ON paper FIELDS title SEARCH ANALYZER ascii;
DEFINE INDEX metadata_idx ON paper FIELDS metadata;
DEFINE INDEX created_at_idx ON paper FIELDS created_at;
DEFINE INDEX idx_papers_owner ON paper FIELDS owner;
DEFINE INDEX idx_papers_status ON paper FIELDS status;

-- Segments table with vector embedding support
DEFINE TABLE segment SCHEMAFULL;
DEFINE FIELD id ON segment TYPE string ASSERT $value != NONE;
DEFINE FIELD paper_id ON segment TYPE string;
DEFINE FIELD section ON segment TYPE string;
DEFINE FIELD content ON segment TYPE string ASSERT $value != NONE;
DEFINE FIELD segment_type ON segment TYPE string;
DEFINE FIELD embedding ON segment TYPE array<float>;
DEFINE FIELD line_range ON segment TYPE array<number>;
DEFINE FIELD created_at ON segment TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON segment TYPE datetime DEFAULT time::now();
DEFINE INDEX paper_id_idx ON segment FIELDS paper_id;
DEFINE INDEX content_search ON segment FIELDS content SEARCH ANALYZER ascii;
DEFINE INDEX segment_type_idx ON segment FIELDS segment_type;
-- Vector index for semantic search (HNSW with cosine distance)
-- Note: Vector index syntax may vary by SurrealDB version
-- For SurrealDB 1.5+, use: DEFINE INDEX embedding_idx ON segment FIELDS embedding MTREE DIMENSION 1536 DIST COSINE;
-- For newer versions with HNSW: DEFINE INDEX embedding_idx ON segment FIELDS embedding VECTOR HNSW DIMENSION 1536 DIST COSINE;
DEFINE INDEX embedding_idx ON segment FIELDS embedding MTREE DIMENSION 1536 DIST COSINE;

-- Repositories table
DEFINE TABLE repository SCHEMAFULL;
DEFINE FIELD id ON repository TYPE string ASSERT $value != NONE;
DEFINE FIELD owner ON repository TYPE option<string>;
DEFINE FIELD paper_id ON repository TYPE option<string>;
DEFINE FIELD root_path ON repository TYPE string ASSERT $value != NONE;
DEFINE FIELD modules ON repository TYPE array<record<module>>;
DEFINE FIELD structure ON repository TYPE object;
DEFINE FIELD metadata ON repository TYPE object;
DEFINE FIELD status ON repository TYPE string DEFAULT 'generating';
DEFINE FIELD verification_status ON repository TYPE option<string>;
DEFINE FIELD created_at ON repository TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON repository TYPE datetime DEFAULT time::now();
DEFINE INDEX root_path_idx ON repository FIELDS root_path;
DEFINE INDEX created_at_idx ON repository FIELDS created_at;
DEFINE INDEX idx_repos_owner ON repository FIELDS owner;
DEFINE INDEX idx_repos_paper ON repository FIELDS paper_id;
DEFINE INDEX idx_repos_status ON repository FIELDS status;

-- Modules table
DEFINE TABLE module SCHEMAFULL;
DEFINE FIELD id ON module TYPE string ASSERT $value != NONE;
DEFINE FIELD repository_id ON module TYPE string;
DEFINE FIELD file_path ON module TYPE string ASSERT $value != NONE;
DEFINE FIELD language ON module TYPE string;
DEFINE FIELD content ON module TYPE string;
DEFINE FIELD ast ON module TYPE option<object>;
DEFINE FIELD dependencies ON module TYPE array<string>;
DEFINE FIELD tests ON module TYPE array<object>;
DEFINE FIELD status ON module TYPE string;
DEFINE FIELD created_at ON module TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON module TYPE datetime DEFAULT time::now();
DEFINE INDEX repository_id_idx ON module FIELDS repository_id;
DEFINE INDEX file_path_idx ON module FIELDS file_path;
DEFINE INDEX language_idx ON module FIELDS language;
DEFINE INDEX status_idx ON module FIELDS status;
DEFINE INDEX content_search ON module FIELDS content SEARCH ANALYZER ascii;

-- Tasks table
DEFINE TABLE task SCHEMAFULL;
DEFINE FIELD id ON task TYPE string ASSERT $value != NONE;
DEFINE FIELD owner ON task TYPE option<string>;
DEFINE FIELD task_type ON task TYPE any ASSERT $value != NONE;
DEFINE FIELD description ON task TYPE string;
DEFINE FIELD context ON task TYPE object;
DEFINE FIELD dependencies ON task TYPE array<string>;
DEFINE FIELD status ON task TYPE any ASSERT $value != NONE;
DEFINE FIELD progress ON task TYPE float DEFAULT 0.0;
DEFINE FIELD error ON task TYPE option<string>;
DEFINE FIELD result ON task TYPE option<object>;
DEFINE FIELD paper_id ON task TYPE option<string>;
DEFINE FIELD module_id ON task TYPE option<string>;
DEFINE FIELD agent_id ON task TYPE option<string>;
DEFINE FIELD created_at ON task TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON task TYPE datetime DEFAULT time::now();
DEFINE FIELD completed_at ON task TYPE option<datetime>;
DEFINE INDEX status_idx ON task FIELDS status;
DEFINE INDEX created_at_idx ON task FIELDS created_at;
DEFINE INDEX paper_id_idx ON task FIELDS paper_id;
DEFINE INDEX module_id_idx ON task FIELDS module_id;
DEFINE INDEX agent_id_idx ON task FIELDS agent_id;
DEFINE INDEX idx_tasks_owner ON task FIELDS owner;
DEFINE INDEX idx_tasks_status ON task FIELDS status;

-- Dependencies table (Graph edges)
DEFINE TABLE dependency SCHEMAFULL;
DEFINE FIELD from ON dependency TYPE string ASSERT $value != NONE;
DEFINE FIELD to ON dependency TYPE string ASSERT $value != NONE;
DEFINE FIELD dependency_type ON dependency TYPE string;
DEFINE FIELD created_at ON dependency TYPE datetime DEFAULT time::now();
DEFINE INDEX from_idx ON dependency FIELDS from;
DEFINE INDEX to_idx ON dependency FIELDS to;
DEFINE INDEX from_to_idx ON dependency FIELDS from, to UNIQUE;

-- Documents table
DEFINE TABLE document SCHEMAFULL;
DEFINE FIELD id ON document TYPE string ASSERT $value != NONE;
DEFINE FIELD name ON document TYPE string ASSERT $value != NONE;
DEFINE FIELD content ON document TYPE string;
DEFINE FIELD content_type ON document TYPE string;
DEFINE FIELD metadata ON document TYPE object;
DEFINE FIELD created_at ON document TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON document TYPE datetime DEFAULT time::now();
DEFINE INDEX name_search ON document FIELDS name SEARCH ANALYZER ascii;
DEFINE INDEX content_type_idx ON document FIELDS content_type;
DEFINE INDEX created_at_idx ON document FIELDS created_at;

-- Users table (for authentication - Phase 2)
DEFINE TABLE user SCHEMAFULL;
DEFINE FIELD id ON user TYPE string ASSERT $value != NONE;
DEFINE FIELD email ON user TYPE string ASSERT string::is::email($value);
DEFINE FIELD username ON user TYPE string;
DEFINE FIELD password_hash ON user TYPE string;
DEFINE FIELD roles ON user TYPE array<string> DEFAULT ['user'];
DEFINE FIELD created_at ON user TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON user TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_user_email ON user FIELDS email UNIQUE;
DEFINE INDEX idx_user_username ON user FIELDS username UNIQUE;

-- Sessions table (for JWT refresh tokens - Phase 2)
DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD id ON session TYPE string ASSERT $value != NONE;
DEFINE FIELD user_id ON session TYPE string;
DEFINE FIELD refresh_token ON session TYPE string;
DEFINE FIELD expires_at ON session TYPE datetime;
DEFINE FIELD created_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD last_used_at ON session TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_session_user ON session FIELDS user_id;
DEFINE INDEX idx_session_refresh_token ON session FIELDS refresh_token UNIQUE;
DEFINE INDEX idx_session_expires ON session FIELDS expires_at;
