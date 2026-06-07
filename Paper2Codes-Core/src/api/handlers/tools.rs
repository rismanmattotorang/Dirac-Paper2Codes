//! Tool metadata handler
//!
//! Exposes the catalog of available agent tools (adapted from PAL MCP) so
//! operators can inspect supported prompts, model requirements, and default
//! temperature profiles via the API/UI.

use crate::agents::{tools::ToolManager, ToolSpec};
use crate::api::state::AppState;
use crate::api::types::responses::ApiResponse;
use crate::api::utils::get_request_id;
use axum::{extract::Extension, http::HeaderMap, response::Json};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

/// API model describing a tool and its configuration metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub requires_model: bool,
    pub model_category: String,
    pub temperature_label: String,
    pub temperature_value: f32,
    pub primary_prompt: Option<PromptDetails>,
    pub supplemental_prompts: Vec<PromptDetails>,
}

/// Prompt metadata provided to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct PromptDetails {
    pub name: String,
    pub content: String,
}

fn tool_spec_to_info(spec: &ToolSpec) -> ToolInfo {
    let primary_prompt = spec.primary_prompt.map(|prompt_type| PromptDetails {
        name: prompt_type.name().to_string(),
        content: prompt_type.get_prompt().to_string(),
    });

    let supplemental_prompts = spec
        .supplemental_prompts
        .iter()
        .map(|prompt_type| PromptDetails {
            name: prompt_type.name().to_string(),
            content: prompt_type.get_prompt().to_string(),
        })
        .collect();

    ToolInfo {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        requires_model: spec.requires_model,
        model_category: spec.model_category.as_str().to_string(),
        temperature_label: spec.temperature.label().to_string(),
        temperature_value: spec.temperature_value(),
        primary_prompt,
        supplemental_prompts,
    }
}

/// List all registered tools and their configuration metadata.
pub async fn list_tools(
    Extension(_state): Extension<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ToolInfo>>>, (axum::http::StatusCode, Json<Value>)> {
    let request_id = get_request_id(&headers);

    let manager = ToolManager::new();
    let tools: Vec<ToolInfo> = manager
        .available_specs()
        .iter()
        .map(tool_spec_to_info)
        .collect();

    Ok(Json(ApiResponse::with_request_id(tools, request_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::get_tool_spec;

    #[test]
    fn converts_tool_spec_to_info() {
        let analyze_spec = get_tool_spec("analyze").expect("analyze tool spec to exist");
        let info = tool_spec_to_info(analyze_spec);
        assert_eq!(info.name, "analyze");
        assert!(info
            .primary_prompt
            .as_ref()
            .expect("primary prompt should exist")
            .content
            .contains("ROLE"));
        assert_eq!(info.temperature_label, "analytical");
        if info.name == "chat" {
            assert!(
                !info.supplemental_prompts.is_empty(),
                "Chat metadata should expose supplemental prompts"
            );
        }
    }
}
