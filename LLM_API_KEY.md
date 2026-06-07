# LLM API Key Management Assessment and Fixes

This document summarizes the assessment and improvements made to LLM API key management in Paper2Codes.

## Latest Update — Dedicated API Key Management (WebUI)

API keys are now fully manageable from the Web UI with a dedicated, per-provider
panel (**Settings → LLM Configuration → Provider API Keys**). Highlights:

- **Per-provider status** — every provider shows whether a key is configured,
  a masked preview (e.g. `sk-p…wxyz`), and the key source (`config` vs
  `environment`). The previous "only the primary provider shows status" bug is
  fixed.
- **Set / Update / Remove** — each provider has its own key editor with a
  show/hide toggle. Keys provided via environment variables are surfaced but
  protected from accidental removal in the UI.
- **Live validation ("Test")** — validates a key against the provider with a
  lightweight, read-only `GET /models` request and reports validity + latency,
  before or after saving.
- **Set default provider** — promote any configured provider to the default in
  one click.
- **Hot reload** — key changes are merged into the running process via a
  runtime override store (`AppState::effective_config`) and take effect
  immediately for in-process consumers (e.g. the streaming LLM endpoint), in
  addition to being persisted to `config.toml` for worker restarts.
- **Secure-at-rest** — the persisted `config.toml` is written with `0600`
  permissions on Unix, and full keys are never returned by any API response.

### New API endpoints

| Method   | Path                                        | Description                              |
|----------|---------------------------------------------|------------------------------------------|
| `GET`    | `/api/settings/llm/providers`               | List providers + masked key status       |
| `PUT`    | `/api/settings/llm/providers/:provider/key` | Set/replace a provider key (persist+live)|
| `DELETE` | `/api/settings/llm/providers/:provider/key` | Remove a provider key                    |
| `POST`   | `/api/settings/llm/providers/:provider/test`| Validate a key against the live provider |
| `PUT`    | `/api/settings/llm/default`                 | Set the default provider                 |

The single source of truth for provider metadata (display name, base URL,
default models, key prefixes, env var) now lives in `config::KNOWN_PROVIDERS`,
removing the provider literals previously duplicated across `Config::default`,
the config loader, and the settings handlers.

---


## Assessment Summary

### Current State (Before Fixes)
- Default provider was set to "openrouter" in code
- OpenAI and Anthropic providers were disabled by default in config.example.toml
- Settings page showed OpenRouter as default
- API keys could be set via environment variables but provider creation logic needed improvement

### Issues Identified
1. ❌ Default provider was "openrouter" instead of "openai"
2. ❌ OpenAI provider disabled by default in example config
3. ❌ Settings page UI showed OpenRouter as default
4. ❌ API key loading didn't auto-enable providers when keys were provided
5. ❌ Settings page provider changes weren't properly validated

## Fixes Applied ✅

### 1. Changed Default Provider to OpenAI

**Files Modified:**
- `Paper2Codes-Core/src/config/mod.rs`
  - Changed `default_provider: "openrouter"` → `"openai"`
  - Updated default providers to include OpenAI and Anthropic enabled by default
  - OpenRouter now disabled by default (can be enabled if needed)

**Code Changes:**
```rust
// Default configuration now includes:
- OpenAI (enabled by default)
- Anthropic (enabled by default)  
- OpenRouter (disabled by default)
default_provider: "openai".to_string()
```

### 2. Updated Config Example Files

**Files Modified:**
- `Paper2Codes-Core/config.example.toml`
  - Changed `default_provider = "openai"` (was already correct)
  - Changed `[llm.providers.openai] enabled = true` (was false)
  - Changed `[llm.providers.anthropic] enabled = true` (was false)
  - API keys are now present in the example file

- `Paper2Codes-Core/env.example`
  - Changed `DEFAULT_LLM_PROVIDER=openai` (was openrouter)
  - API keys are present in the example file

### 3. Enhanced API Key Loading Logic

**Files Modified:**
- `Paper2Codes-Core/src/config/loader.rs`

**Improvements:**
- ✅ Auto-enable providers when API keys are provided via environment variables
- ✅ Auto-create provider configurations if they don't exist when API key is provided
- ✅ Proper handling for OpenAI, Anthropic, and OpenRouter keys

**Example:**
```rust
// If OPENAI_API_KEY is set, OpenAI provider is:
// 1. Created if it doesn't exist
// 2. Enabled automatically
// 3. API key is set
```

### 4. Updated Settings Page UI

**Files Modified:**
- `Paper2Codes-WebUI/components/settings/settings-page.tsx`
  - Changed dropdown to show "OpenAI (Default)" as first option
  - Reordered options: OpenAI, Anthropic, OpenRouter, xAI

### 5. Improved Settings Update Handler

**Files Modified:**
- `Paper2Codes-Core/src/api/handlers/settings.rs`

**Improvements:**
- ✅ Proper validation of provider changes
- ✅ Checks if provider exists and is enabled before allowing change
- ✅ Updates in-memory config (note: requires restart for persistence to config file)
- ✅ Validates temperature, timeout, and retry settings
- ✅ Updates model preferences

**Validation Rules:**
- Provider must exist in configuration
- Provider must be enabled
- Temperature must be 0.0-2.0
- Password min length must be >= 8

## API Key Management

### Supported Methods

1. **Config File** (`~/.config/paper2codes/config.toml`)
   ```toml
   [llm]
   default_provider = "openai"
   openai_api_key = "sk-..."
   anthropic_api_key = "sk-ant-..."
   ```

2. **Environment Variables**
   ```bash
   export OPENAI_API_KEY="sk-..."
   export ANTHROPIC_API_KEY="sk-ant-..."
   export DEFAULT_LLM_PROVIDER="openai"
   ```

3. **Settings Page** (Web UI)
   - Navigate to Settings → LLM Configuration
   - Change "Primary LLM Provider" dropdown
   - Modify temperature, timeout, and model preferences
   - Changes are validated and applied

### Priority Order

1. Environment variables (highest priority)
2. Config file values
3. Default values

### Provider Status

| Provider | Default Status | Auto-Enable with Key? |
|----------|---------------|----------------------|
| OpenAI   | ✅ Enabled     | ✅ Yes                |
| Anthropic| ✅ Enabled     | ✅ Yes                |
| OpenRouter| ❌ Disabled   | ✅ Yes                |
| xAI      | ❌ Disabled    | ⚠️ Not implemented    |

## Usage Examples

### Setting API Keys via Environment Variables

```bash
# Set OpenAI API key (default provider)
export OPENAI_API_KEY="sk-proj-..."

# Set Anthropic API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Change default provider
export DEFAULT_LLM_PROVIDER="anthropic"
```

### Setting API Keys in Config File

```toml
[llm]
default_provider = "openai"
openai_api_key = "sk-proj-..."
anthropic_api_key = "sk-ant-..."

[llm.providers.openai]
enabled = true
base_url = "https://api.openai.com/v1"

[llm.providers.anthropic]
enabled = true
base_url = "https://api.anthropic.com/v1"
```

### Changing Provider via Settings Page

1. Navigate to Settings in the WebUI
2. Click on "LLM Configuration" tab
3. Select new provider from "Primary LLM Provider" dropdown
4. Click "Save Changes"
5. ⚠️ **Note**: Changes require backend restart to persist to config file

## Current Configuration

### Default Provider: OpenAI ✅

The default LLM provider is now **OpenAI**. This can be changed:
- In config file: `default_provider = "anthropic"`
- Via environment: `DEFAULT_LLM_PROVIDER=anthropic`
- Via Settings page: Change dropdown selection

### Default Models

- **Planning**: `gpt-4-turbo-preview` (OpenAI)
- **Analysis**: `claude-3-opus-20240229` (Anthropic)
- **Coding**: `gpt-4-turbo-preview` (OpenAI)
- **Verification**: `claude-3-opus-20240229` (Anthropic)

These can be customized in:
- Config file: `[agents]` section
- Settings page: Model Preferences section

## Settings Page Features

### LLM Configuration Tab

✅ **Primary Provider Selection**
- Dropdown with: OpenAI (Default), Anthropic, OpenRouter, xAI
- Validates provider exists and is enabled
- Shows API key status (Configured/Not Configured)

✅ **Temperature Control**
- Slider (0.0 - 2.0)
- Applies to selected provider

✅ **Model Preferences**
- Planning, Analysis, Coding, Verification models
- Can select different models for each task

✅ **Advanced Settings**
- Timeout (seconds)
- Max retries

### Resolved Limitations

✅ **Persistence & hot reload**: Provider and key changes are persisted to
`config.toml` *and* applied to the running process via the runtime override
store, so in-process consumers pick them up without a restart.

✅ **API Key Management via UI**: API keys can now be set, tested, and removed
per provider from the dedicated Provider API Keys panel. Keys are stored
server-side, the config file is written with `0600` permissions, and responses
only ever expose a masked preview — never the full secret.

## Testing

To verify the configuration:

1. **Check default provider:**
   ```bash
   cd Paper2Codes-Core
   cargo run -- config | grep default_provider
   # Should show: default_provider = "openai"
   ```

2. **Verify API keys loaded:**
   ```bash
   echo $OPENAI_API_KEY  # Should show your key
   echo $ANTHROPIC_API_KEY  # Should show your key
   ```

3. **Test Settings page:**
   - Open WebUI → Settings → LLM Configuration
   - Verify "OpenAI (Default)" is selected
   - Change to Anthropic and save
   - Verify change is reflected

## Summary

✅ **Default Provider**: Changed from OpenRouter to OpenAI
✅ **Provider Defaults**: OpenAI and Anthropic enabled by default
✅ **API Key Loading**: Auto-enable providers when keys provided
✅ **Settings Page**: Updated to show OpenAI as default
✅ **Validation**: Added proper validation for provider changes
✅ **Documentation**: Example files updated with correct defaults

**Note**: All changes are backward compatible. Existing configs with OpenRouter will continue to work, but new installations will default to OpenAI.

