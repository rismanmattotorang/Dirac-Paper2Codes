//! Result merging for multi-LLM outputs
use crate::llm::{LLMClient, LLMRequest, Message, MessageRole};
use std::sync::Arc;
use tracing::{debug, warn};

/// Merge code outputs from different LLMs intelligently
pub async fn merge_code_outputs(
    openai_code: &str,
    claude_code: &str,
    original_request: &LLMRequest,
    merge_client: Arc<dyn LLMClient>,
) -> crate::error::Result<String> {
    // If one output is significantly longer, prefer it
    if openai_code.len() > (claude_code.len() as f64 * 1.5) as usize {
        debug!("OpenAI code is significantly longer, using it");
        return Ok(openai_code.to_string());
    }

    if claude_code.len() > (openai_code.len() as f64 * 1.5) as usize {
        debug!("Claude code is significantly longer, using it");
        return Ok(claude_code.to_string());
    }

    // Otherwise, use LLM to merge intelligently
    let merge_prompt = format!(
        "You are an expert code reviewer. Two AI models generated code for the same task. Merge them into a single, optimal implementation.

Original Request Context:
{}

First Implementation (OpenAI):
```
{}
```

Second Implementation (Claude):
```
{}
```

Create a merged implementation that:
1. Combines the best features of both
2. Uses the better structure and organization
3. Incorporates the best error handling
4. Uses the most efficient algorithms
5. Preserves comprehensive documentation
6. Ensures completeness and executability

Return ONLY the merged code, no explanations.",
        format!("{:?}", original_request),
        openai_code,
        claude_code
    );

    let merge_request = LLMRequest {
        model: merge_client.model(),
        messages: vec![
            Message {
                role: MessageRole::System,
                content:
                    "You are an expert at merging code implementations from different AI models."
                        .to_string(),
            },
            Message {
                role: MessageRole::User,
                content: merge_prompt,
            },
        ],
        temperature: 0.3,
        max_tokens: Some(4000),
        stream: false,
    };

    match merge_client.complete(merge_request).await {
        Ok(response) => {
            debug!("Successfully merged code outputs");
            Ok(response.content)
        }
        Err(e) => {
            warn!("Failed to merge with LLM, using longer output: {}", e);
            // Fallback to longer output
            if openai_code.len() >= claude_code.len() {
                Ok(openai_code.to_string())
            } else {
                Ok(claude_code.to_string())
            }
        }
    }
}
