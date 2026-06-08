use axum::{extract::Extension, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{api::state::AppState, error::Result};

/// Response for settings retrieval
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub general: GeneralSettings,
    pub security: SecuritySettings,
    pub notifications: NotificationSettings,
    pub llm: LLMSettings,
    pub database: DatabaseSettings,
    pub team: TeamSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralSettings {
    pub organization_name: String,
    pub default_domain: String,
    pub dark_mode: bool,
    pub theme: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecuritySettings {
    pub two_factor_enabled: bool,
    pub active_sessions: Vec<SessionInfo>,
    pub password_min_length: usize,
    pub jwt_expiration: u64,
    pub enable_csrf: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub device: String,
    pub ip: String,
    pub last_active: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationSettings {
    pub paper_processing_complete: bool,
    pub code_generation_errors: bool,
    pub task_queue_updates: bool,
    pub weekly_report: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMSettings {
    pub primary_provider: String,
    pub api_key_configured: bool,
    pub temperature: f32,
    pub model_preferences: ModelPreferences,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelPreferences {
    pub planning: String,
    pub analysis: String,
    pub coding: String,
    pub verification: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub connected: bool,
    pub connection_string: String,
    pub namespace: String,
    pub database: String,
    pub max_connections: usize,
    pub backup_schedule: String,
    pub last_sync: Option<String>,
    pub schema_valid: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamSettings {
    pub members: Vec<TeamMember>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: String,
}

/// Request for updating settings
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub general: Option<GeneralSettings>,
    pub security: Option<SecuritySettingsUpdate>,
    pub notifications: Option<NotificationSettings>,
    pub llm: Option<LLMSettingsUpdate>,
    pub database: Option<DatabaseSettingsUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct SecuritySettingsUpdate {
    pub password_min_length: Option<usize>,
    pub jwt_expiration: Option<u64>,
    pub enable_csrf: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LLMSettingsUpdate {
    pub primary_provider: Option<String>,
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub model_preferences: Option<ModelPreferences>,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseSettingsUpdate {
    pub connection_string: Option<String>,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub max_connections: Option<usize>,
    pub backup_schedule: Option<String>,
}

/// Get current settings
pub async fn get_settings(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<SettingsResponse>> {
    info!("Fetching current settings");

    // Use the effective config so runtime API-key/provider overrides are reflected.
    let config = state.effective_config().await;

    // Build settings response from current config
    let settings = SettingsResponse {
        general: GeneralSettings {
            organization_name: "AI Research Lab".to_string(), // TODO: Store in DB
            default_domain: "Auto-detect".to_string(),
            dark_mode: config.ui.theme == "dark",
            theme: config.ui.theme.clone(),
        },
        security: SecuritySettings {
            two_factor_enabled: false, // TODO: Implement 2FA
            active_sessions: get_active_sessions(&state).await,
            password_min_length: config.api.auth.password_min_length,
            jwt_expiration: config.api.auth.jwt_expiration,
            enable_csrf: config.api.auth.enable_csrf,
        },
        notifications: NotificationSettings {
            paper_processing_complete: true, // TODO: Store in DB
            code_generation_errors: true,
            task_queue_updates: false,
            weekly_report: true,
        },
        llm: LLMSettings {
            primary_provider: config.llm.default_provider.clone(),
            api_key_configured: config.get_api_key(&config.llm.default_provider).is_some(),
            temperature: config
                .llm
                .providers
                .get(&config.llm.default_provider)
                .map(|p| p.temperature)
                .unwrap_or(0.7),
            model_preferences: ModelPreferences {
                planning: config.agents.planning_model.clone(),
                analysis: config.agents.analysis_model.clone(),
                coding: config.agents.coding_model.clone(),
                verification: config.agents.verification_model.clone(),
            },
            timeout_seconds: config.llm.timeout_seconds,
            max_retries: config.llm.max_retries,
        },
        database: {
            let storage_guard = state.storage.read().await;
            let storage_manager = storage_guard.as_ref().cloned();
            drop(storage_guard);

            let (is_connected, schema_valid, last_sync) =
                if let Some(ref storage) = storage_manager {
                    let is_connected = storage.test_connection().await.is_ok();
                    let schema_valid = if is_connected {
                        storage.validate_schema().await.is_ok()
                    } else {
                        false
                    };
                    let last_sync = storage
                        .connection_info()
                        .await
                        .map(|info| info.connected_at.to_rfc3339());
                    (is_connected, schema_valid, last_sync)
                } else {
                    (false, false, None)
                };

            DatabaseSettings {
                connected: is_connected,
                connection_string: config.storage.connection_string.clone(),
                namespace: config.storage.namespace.clone(),
                database: config.storage.database.clone(),
                max_connections: config.storage.max_connections,
                backup_schedule: "Daily at 2:00 AM UTC".to_string(), // TODO: Make configurable
                last_sync,
                schema_valid,
            }
        },
        team: TeamSettings {
            members: get_team_members().await, // TODO: Store in DB
        },
    };

    Ok(Json(settings))
}

/// Update settings
pub async fn update_settings(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>> {
    info!("Updating settings");

    // Start from the effective config (bootstrap + current runtime overrides).
    let mut config = state.effective_config().await;
    let mut config_changed = false;

    if let Some(llm_update) = &request.llm {
        // Update default provider first (if changed) so API key update uses correct provider
        let target_provider = if let Some(ref new_provider) = llm_update.primary_provider {
            // Validate provider exists and is enabled
            if let Some(provider) = config.llm.providers.get(new_provider) {
                if provider.enabled || llm_update.api_key.is_some() {
                    // Allow changing to a provider even if disabled, if API key is being set
                    if config.llm.default_provider != *new_provider {
                        info!(
                            "Changing default LLM provider from {} to {}",
                            config.llm.default_provider, new_provider
                        );
                        config.llm.default_provider = new_provider.clone();
                        // Mirror into the runtime override store so the change is live.
                        state.llm_overrides.write().await.default_provider =
                            Some(new_provider.clone());
                        config_changed = true;
                    }
                    new_provider.clone()
                } else {
                    return Err(crate::error::Paper2CodesError::Validation(format!(
                        "Provider '{}' is not enabled. Please set an API key first.",
                        new_provider
                    )));
                }
            } else {
                return Err(crate::error::Paper2CodesError::Validation(format!(
                    "Provider '{}' not found. Available providers: {}",
                    new_provider,
                    config
                        .llm
                        .providers
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        } else {
            config.llm.default_provider.clone()
        };

        // Update API key for the target provider
        if let Some(ref api_key) = llm_update.api_key {
            if let Some(provider) = config.llm.providers.get_mut(&target_provider) {
                provider.api_key = Some(api_key.clone());
                provider.enabled = true; // Enable provider when API key is set
                // Mirror into the runtime override store so the key is live immediately.
                state
                    .llm_overrides
                    .write()
                    .await
                    .api_keys
                    .insert(target_provider.clone(), Some(api_key.clone()));
                info!("API key updated for provider: {}", target_provider);
                config_changed = true;
            } else {
                return Err(crate::error::Paper2CodesError::Validation(format!(
                    "Provider '{}' not found",
                    target_provider
                )));
            }
        }

        // Validate and update temperature
        if let Some(temp) = llm_update.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(crate::error::Paper2CodesError::Validation(
                    "Temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
            // Update temperature for the current provider
            if let Some(provider) = config.llm.providers.get_mut(&config.llm.default_provider) {
                provider.temperature = temp;
                config_changed = true;
            }
        }

        // Update timeout and retries
        if let Some(timeout) = llm_update.timeout_seconds {
            config.llm.timeout_seconds = timeout;
            config_changed = true;
        }

        if let Some(retries) = llm_update.max_retries {
            config.llm.max_retries = retries;
            config_changed = true;
        }

        // Update model preferences
        if let Some(ref model_prefs) = llm_update.model_preferences {
            config.agents.planning_model = model_prefs.planning.clone();
            config.agents.analysis_model = model_prefs.analysis.clone();
            config.agents.coding_model = model_prefs.coding.clone();
            config.agents.verification_model = model_prefs.verification.clone();
            config_changed = true;
        }
    }

    if let Some(security_update) = &request.security {
        if let Some(min_length) = security_update.password_min_length {
            if min_length < 8 {
                return Err(crate::error::Paper2CodesError::Validation(
                    "Password minimum length must be at least 8 characters".to_string(),
                ));
            }
            config.api.auth.password_min_length = min_length;
            config_changed = true;
        }
    }

    // Persist the merged configuration to disk. LLM API-key and default-provider
    // changes are also mirrored into the runtime override store above, so they
    // take effect immediately; other settings take effect on the next restart.
    if config_changed {
        let config_path = crate::api::state::config_file_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                    "Failed to create config directory: {}",
                    e
                )))
            })?;
        }

        let toml_content = toml::to_string_pretty(&config).map_err(|e| {
            crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to serialize config: {}",
                e
            )))
        })?;

        std::fs::write(&config_path, toml_content).map_err(|e| {
            crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to write config file: {}",
                e
            )))
        })?;

        // The config file can hold plaintext API keys — restrict its permissions.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))
            {
                tracing::warn!(
                    "Failed to tighten permissions on {}: {}",
                    config_path.display(),
                    e
                );
            }
        }

        info!("Configuration saved to {}", config_path.display());
    }

    // Return updated settings based on the modified config
    let settings = SettingsResponse {
        general: GeneralSettings {
            organization_name: "AI Research Lab".to_string(),
            default_domain: "Auto-detect".to_string(),
            dark_mode: config.ui.theme == "dark",
            theme: config.ui.theme.clone(),
        },
        security: SecuritySettings {
            two_factor_enabled: false,
            active_sessions: get_active_sessions(&state).await,
            password_min_length: config.api.auth.password_min_length,
            jwt_expiration: config.api.auth.jwt_expiration,
            enable_csrf: config.api.auth.enable_csrf,
        },
        notifications: NotificationSettings {
            paper_processing_complete: true,
            code_generation_errors: true,
            task_queue_updates: false,
            weekly_report: true,
        },
        llm: LLMSettings {
            primary_provider: config.llm.default_provider.clone(),
            api_key_configured: config.get_api_key(&config.llm.default_provider).is_some(),
            temperature: config
                .llm
                .providers
                .get(&config.llm.default_provider)
                .map(|p| p.temperature)
                .unwrap_or(0.7),
            model_preferences: ModelPreferences {
                planning: config.agents.planning_model.clone(),
                analysis: config.agents.analysis_model.clone(),
                coding: config.agents.coding_model.clone(),
                verification: config.agents.verification_model.clone(),
            },
            timeout_seconds: config.llm.timeout_seconds,
            max_retries: config.llm.max_retries,
        },
        database: {
            let storage_guard = state.storage.read().await;
            let storage_manager = storage_guard.as_ref().cloned();
            drop(storage_guard);

            let (is_connected, schema_valid, last_sync) =
                if let Some(ref storage) = storage_manager {
                    let is_connected = storage.test_connection().await.is_ok();
                    let schema_valid = if is_connected {
                        storage.validate_schema().await.is_ok()
                    } else {
                        false
                    };
                    let last_sync = storage
                        .connection_info()
                        .await
                        .map(|info| info.connected_at.to_rfc3339());
                    (is_connected, schema_valid, last_sync)
                } else {
                    (false, false, None)
                };

            DatabaseSettings {
                connected: is_connected,
                connection_string: config.storage.connection_string.clone(),
                namespace: config.storage.namespace.clone(),
                database: config.storage.database.clone(),
                max_connections: config.storage.max_connections,
                backup_schedule: "Daily at 2:00 AM UTC".to_string(),
                last_sync,
                schema_valid,
            }
        },
        team: TeamSettings {
            members: get_team_members().await,
        },
    };

    Ok(Json(settings))
}

/// Get active sessions from the live session store.
async fn get_active_sessions(state: &AppState) -> Vec<SessionInfo> {
    state
        .session_store
        .all_active()
        .await
        .into_iter()
        .map(|s| SessionInfo {
            id: s.id,
            device: "API session".to_string(),
            ip: "-".to_string(),
            last_active: s.last_used_at.to_rfc3339(),
            created_at: s.created_at.to_rfc3339(),
        })
        .collect()
}

/// Get team members (mock data for now)
async fn get_team_members() -> Vec<TeamMember> {
    vec![TeamMember {
        id: "user_1".to_string(),
        name: "Current User".to_string(),
        email: "user@example.com".to_string(),
        role: "Admin".to_string(),
        status: "Active".to_string(),
    }]
}

/// Revoke a session
pub async fn revoke_session(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<StatusCode> {
    info!("Revoking session: {}", session_id);
    state.session_store.revoke(&session_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Enable 2FA
pub async fn enable_2fa(
    Extension(_state): Extension<Arc<AppState>>,
) -> Result<Json<Enable2FAResponse>> {
    info!("Enabling 2FA");

    // TODO: Implement 2FA setup

    Ok(Json(Enable2FAResponse {
        qr_code: "data:image/png;base64,...".to_string(),
        secret: "ABCDEFGHIJKLMNOP".to_string(),
        backup_codes: vec!["12345678".to_string(), "87654321".to_string()],
    }))
}

#[derive(Debug, Serialize)]
pub struct Enable2FAResponse {
    pub qr_code: String,
    pub secret: String,
    pub backup_codes: Vec<String>,
}

/// Disable 2FA
pub async fn disable_2fa(Extension(_state): Extension<Arc<AppState>>) -> Result<StatusCode> {
    info!("Disabling 2FA");

    // TODO: Implement 2FA disable

    Ok(StatusCode::NO_CONTENT)
}

/// Test database connection
pub async fn test_database_connection(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<DatabaseConnectionTestResponse>> {
    info!("Testing database connection");

    let start_time = std::time::Instant::now();

    // Check if storage is initialized, if not, try to initialize it
    let storage_guard = state.storage.read().await;

    if storage_guard.is_none() {
        drop(storage_guard); // Release read lock before acquiring write lock

        // Storage is not initialized, try to initialize it
        info!("Storage not initialized, attempting to initialize...");

        if !state.config.storage.enabled {
            return Ok(Json(DatabaseConnectionTestResponse {
                success: false,
                message:
                    "Storage is disabled in configuration. Please enable it in your config file."
                        .to_string(),
                latency_ms: None,
                schema_valid: false,
            }));
        }

        // Create new storage manager and attempt connection
        let manager = crate::storage::StorageManager::new();
        let storage_config = state.config.storage.clone();

        match manager.connect(storage_config.clone()).await {
            Ok(_) => {
                info!(
                    "Successfully initialized storage connection to {}",
                    storage_config.connection_string
                );

                // Test the connection
                match manager.test_connection().await {
                    Ok(_) => {
                        // Validate schema before storing the manager
                        if let Err(e) = manager.validate_schema().await {
                            return Ok(Json(DatabaseConnectionTestResponse {
                                success: false,
                                message: format!(
                                    "Connected to SurrealDB but schema validation failed: {}",
                                    e
                                ),
                                latency_ms: None,
                                schema_valid: false,
                            }));
                        }

                        // Store the manager in AppState
                        let mut storage_write = state.storage.write().await;
                        *storage_write = Some(manager);
                        drop(storage_write);

                        let latency = start_time.elapsed().as_millis() as u64;
                        return Ok(Json(DatabaseConnectionTestResponse {
                            success: true,
                            message: format!(
                                "Successfully connected to SurrealDB (latency: {}ms)",
                                latency
                            ),
                            latency_ms: Some(latency),
                            schema_valid: true,
                        }));
                    }
                    Err(e) => {
                        return Ok(Json(DatabaseConnectionTestResponse {
                            success: false,
                            message: format!(
                                "Connected to SurrealDB but connection test failed: {}",
                                e
                            ),
                            latency_ms: None,
                            schema_valid: false,
                        }));
                    }
                }
            }
            Err(e) => {
                return Ok(Json(DatabaseConnectionTestResponse {
                    success: false,
                    message: format!("Failed to initialize storage connection to {}: {}. Please ensure SurrealDB is running and accessible.", storage_config.connection_string, e),
                    latency_ms: None,
                    schema_valid: false,
                }));
            }
        }
    }

    // Storage is already initialized, just test the connection
    drop(storage_guard);
    let storage_guard = state.storage.read().await;
    let storage = storage_guard.as_ref().unwrap();

    match storage.test_connection().await {
        Ok(_) => {
            let latency = start_time.elapsed().as_millis() as u64;
            let schema_valid = storage.validate_schema().await.is_ok();
            Ok(Json(DatabaseConnectionTestResponse {
                success: true,
                message: if schema_valid {
                    format!(
                        "Successfully connected to SurrealDB (latency: {}ms)",
                        latency
                    )
                } else {
                    format!(
                        "Connected to SurrealDB (latency: {}ms) but schema validation failed. Enable storage.auto_migrate or run the migrations.",
                        latency
                    )
                },
                latency_ms: Some(latency),
                schema_valid,
            }))
        }
        Err(e) => Ok(Json(DatabaseConnectionTestResponse {
            success: false,
            message: format!("Database connection test failed: {}", e),
            latency_ms: None,
            schema_valid: false,
        })),
    }
}

#[derive(Debug, Serialize)]
pub struct DatabaseConnectionTestResponse {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub schema_valid: bool,
}

/// Add team member
pub async fn add_team_member(
    Extension(_state): Extension<Arc<AppState>>,
    Json(request): Json<AddTeamMemberRequest>,
) -> Result<Json<TeamMember>> {
    info!("Adding team member: {}", request.email);

    // TODO: Implement team member addition

    Ok(Json(TeamMember {
        id: format!("user_{}", Uuid::new_v4()),
        name: request.name,
        email: request.email,
        role: request.role,
        status: "Invited".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct AddTeamMemberRequest {
    pub name: String,
    pub email: String,
    pub role: String,
}

/// Remove team member
pub async fn remove_team_member(
    Extension(_state): Extension<Arc<AppState>>,
    axum::extract::Path(member_id): axum::extract::Path<String>,
) -> Result<StatusCode> {
    info!("Removing team member: {}", member_id);

    // TODO: Implement team member removal

    Ok(StatusCode::NO_CONTENT)
}

/// Update team member role
pub async fn update_team_member_role(
    Extension(_state): Extension<Arc<AppState>>,
    axum::extract::Path(member_id): axum::extract::Path<String>,
    Json(request): Json<UpdateTeamMemberRoleRequest>,
) -> Result<Json<TeamMember>> {
    info!(
        "Updating team member role: {} -> {}",
        member_id, request.role
    );

    // TODO: Implement role update

    Ok(Json(TeamMember {
        id: member_id,
        name: "Team Member".to_string(),
        email: "member@example.com".to_string(),
        role: request.role,
        status: "Active".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamMemberRoleRequest {
    pub role: String,
}
