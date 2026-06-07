//! Test fixtures and mock data

use serde_json::Value;

/// Create a test user fixture
pub fn test_user() -> Value {
    serde_json::json!({
        "email": "test@example.com",
        "username": "testuser",
        "password": "testpassword123"
    })
}

/// Create a test paper fixture
pub fn test_paper() -> Value {
    serde_json::json!({
        "id": "test-paper-1",
        "title": "Test Paper Title",
        "abstract_text": "This is a test paper abstract for integration testing.",
        "content": "Full paper content here...",
        "metadata": {
            "authors": ["Author One", "Author Two"],
            "year": 2024,
            "venue": "Test Conference"
        }
    })
}

/// Create a test repository fixture
pub fn test_repository() -> Value {
    serde_json::json!({
        "id": "test-repo-1",
        "name": "test-repository",
        "description": "A test repository",
        "language": "python",
        "status": "generating"
    })
}

/// Create a test task fixture
pub fn test_task() -> Value {
    serde_json::json!({
        "id": "test-task-1",
        "task_type": "planning",
        "description": "Test task description",
        "status": "pending",
        "paper_id": "test-paper-1"
    })
}

/// Create multiple test papers
pub fn test_papers(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "id": format!("test-paper-{}", i),
                "title": format!("Test Paper {}", i),
                "abstract_text": format!("Abstract for paper {}", i),
                "content": format!("Content for paper {}", i),
                "metadata": {
                    "authors": [format!("Author {}", i)],
                    "year": 2024
                }
            })
        })
        .collect()
}

/// Create a test search query
pub fn test_search_query() -> Value {
    serde_json::json!({
        "query": "test query",
        "filters": {
            "year": 2024
        },
        "limit": 10
    })
}

/// Create a test batch request
pub fn test_batch_request() -> Value {
    serde_json::json!({
        "requests": [
            {
                "id": "req-1",
                "method": "GET",
                "path": "/api/papers/test-paper-1"
            },
            {
                "id": "req-2",
                "method": "GET",
                "path": "/api/repositories/test-repo-1"
            }
        ]
    })
}

