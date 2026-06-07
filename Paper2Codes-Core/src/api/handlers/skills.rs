//! Domain Skill management endpoints.
//!
//! Skills are listable (choose), individually fetchable, and user-savable
//! (improve). The effective set is the engine's built-in skills overlaid with
//! any user-authored skills found in the user skills directory.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::Deserialize;
use tracing::info;

use crate::api::state::AppState;
use crate::error::{Paper2CodesError, Result};
use crate::skills::{Skill, SkillRegistry};

/// Build the effective registry: built-ins overlaid with user skills.
fn effective_registry() -> SkillRegistry {
    let mut registry = SkillRegistry::with_builtins();
    if let Ok(dir) = SkillRegistry::user_skills_dir() {
        if let Err(e) = registry.load_user_dir(&dir) {
            tracing::warn!("Failed to load user skills from {}: {}", dir.display(), e);
        }
    }
    registry
}

/// `GET /api/skills` — list all available skills.
pub async fn list_skills(
    Extension(_state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<Skill>>> {
    let registry = effective_registry();
    Ok(Json(registry.list().to_vec()))
}

/// `GET /api/skills/:id` — fetch a single skill.
pub async fn get_skill(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Skill>> {
    let registry = effective_registry();
    registry
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| Paper2CodesError::Validation(format!("Skill '{}' not found", id)))
}

#[derive(Debug, Deserialize)]
pub struct UpsertSkillRequest {
    #[serde(flatten)]
    pub skill: Skill,
}

/// `PUT /api/skills/:id` — create or improve a user skill (persisted to disk).
pub async fn upsert_skill(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpsertSkillRequest>,
) -> Result<Json<Skill>> {
    let mut skill = request.skill;
    // The path id is authoritative; user skills are never marked built-in.
    skill.id = id;
    skill.builtin = false;
    skill.validate()?;

    let dir = SkillRegistry::user_skills_dir()?;
    let path = SkillRegistry::save_skill(&dir, &skill)?;
    info!("Saved user skill '{}' to {}", skill.id, path.display());

    Ok(Json(skill))
}
