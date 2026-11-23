use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::cli::commands as cli_commands;
use crate::cli::error::CliError;
use crate::cli::locate::ProbeEnv;
use crate::cli::providers;
use crate::cli::CliService;
use crate::contracts::{
    AutosubmitConfigRequest, AutosubmitStatus, CliActionResult, CustomPricingEntry,
    CustomPricingState, ModelPriceLookupResult, ProviderAccountInfo, ProviderActionRequest,
    ProviderStatusItem, ProvidersStatusPayload, SocialActionRequest, SocialStatusPayload,
    TokscaleSettingsPatch, TokscaleSettingsState,
};
use crate::shell::prefs::atomic_write_file;

const SETTINGS_FILENAME: &str = "settings.json";
const CUSTOM_PRICING_FILENAME: &str = "custom-pricing.json";

pub fn tokscale_config_dir(env: &ProbeEnv) -> Result<PathBuf, String> {
    env.tokscale_config_dir()
        .ok_or_else(|| "Unable to resolve tokscale config directory".to_string())
}

fn resolve_write_target(path: &Path) -> PathBuf {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            if let Ok(canon) = fs::canonicalize(path) {
                return canon;
            }
        }
    }
    path.to_path_buf()
}

pub fn read_tokscale_settings(
    env: &ProbeEnv,
    app_prefs_path: Option<&Path>,
) -> Result<TokscaleSettingsState, String> {
    let config_dir = tokscale_config_dir(env)?;
    let path = config_dir.join(SETTINGS_FILENAME);
    let exists = path.is_file();

    let root_obj: Map<String, Value> = if exists {
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let parsed: Value = serde_json::from_str(&raw)
            .map_err(|err| format!("Invalid JSON in {}: {err}", path.display()))?;
        parsed
            .as_object()
            .cloned()
            .ok_or_else(|| format!("{} must be a JSON object", path.display()))?
    } else {
        Map::new()
    };

    Ok(extract_settings_state(
        &config_dir,
        &path,
        exists,
        app_prefs_path,
        &root_obj,
    ))
}

pub fn patch_tokscale_settings(
    env: &ProbeEnv,
    app_prefs_path: Option<&Path>,
    patch: &TokscaleSettingsPatch,
) -> Result<TokscaleSettingsState, String> {
    let config_dir = tokscale_config_dir(env)?;
    let path = config_dir.join(SETTINGS_FILENAME);
    let write_target = resolve_write_target(&path);

    let mut root_obj: Map<String, Value> = if path.is_file() {
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let parsed: Value = serde_json::from_str(&raw)
            .map_err(|err| format!("Invalid JSON in {}: {err}", path.display()))?;
        parsed
            .as_object()
            .cloned()
            .ok_or_else(|| format!("{} must be a JSON object", path.display()))?
    } else {
        Map::new()
    };

    if let Some(ref clients) = patch.default_clients {
        let cleaned: Vec<Value> = clients
            .iter()
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .map(|c| Value::String(c.to_string()))
            .collect();
        root_obj.insert("defaultClients".to_string(), Value::Array(cleaned));
    }

    if let Some(include_unused) = patch.include_unused_models {
        root_obj.insert(
            "includeUnusedModels".to_string(),
            Value::Bool(include_unused),
        );
    }

    if let Some(ref aliases) = patch.model_aliases {
        let mut map = Map::new();
        for (k, v) in aliases {
            let kt = k.trim();
            let vt = v.trim();
            if !kt.is_empty() && !vt.is_empty() {
                map.insert(kt.to_string(), Value::String(vt.to_string()));
            }
        }
        root_obj.insert("modelAliases".to_string(), Value::Object(map));
    }

    if let Some(timeout_ms) = patch.native_timeout_ms {
        if !(5_000..=3_600_000).contains(&timeout_ms) {
            return Err("nativeTimeoutMs must be between 5,000 and 3,600,000 ms".to_string());
        }
        root_obj.insert("nativeTimeoutMs".to_string(), json!(timeout_ms));
    }

    if let Some(write_cache) = patch.light_write_cache {
        let light = ensure_child_object(&mut root_obj, "light");
        light.insert("writeCache".to_string(), Value::Bool(write_cache));
    }

    if let Some(enabled) = patch.auto_refresh_enabled {
        root_obj.insert("autoRefreshEnabled".to_string(), Value::Bool(enabled));
    }

    if let Some(refresh_ms) = patch.auto_refresh_ms {
        if !(30_000..=3_600_000).contains(&refresh_ms) {
            return Err("autoRefreshMs must be between 30,000 and 3,600,000 ms".to_string());
        }
        root_obj.insert("autoRefreshMs".to_string(), json!(refresh_ms));
    }

    if let Some(minutely) = patch.minutely_tab_enabled {
        root_obj.insert("minutelyTabEnabled".to_string(), Value::Bool(minutely));
    }

    if let Some(ref palette) = patch.color_palette {
        let trimmed = palette.trim();
        if trimmed.is_empty() {
            return Err("colorPalette cannot be empty".to_string());
        }
        root_obj.insert("colorPalette".to_string(), Value::String(trimmed.to_string()));
    }

    if let Some(ref lang) = patch.tui_language {
        let trimmed = lang.trim();
        if trimmed.is_empty() {
            return Err("tuiLanguage cannot be empty".to_string());
        }
        root_obj.insert("tuiLanguage".to_string(), Value::String(trimmed.to_string()));
    }

    if patch.extra_scan_paths.is_some()
        || patch.opencode_db_paths.is_some()
        || patch.local_machine_label.is_some()
    {
        let scanner = ensure_child_object(&mut root_obj, "scanner");

        if let Some(ref extra_paths) = patch.extra_scan_paths {
            let mut extra_obj = Map::new();
            for (client, dirs) in extra_paths {
                let ct = client.trim();
                if ct.is_empty() {
                    continue;
                }
                let cleaned: Vec<Value> = dirs
                    .iter()
                    .map(|d| d.trim())
                    .filter(|d| !d.is_empty())
                    .map(|d| Value::String(d.to_string()))
                    .collect();
                if !cleaned.is_empty() {
                    extra_obj.insert(ct.to_string(), Value::Array(cleaned));
                }
            }
            scanner.insert("extraScanPaths".to_string(), Value::Object(extra_obj));
        }

        if let Some(ref db_paths) = patch.opencode_db_paths {
            let cleaned: Vec<Value> = db_paths
                .iter()
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .map(|p| Value::String(p.to_string()))
                .collect();
            scanner.insert("opencodeDbPaths".to_string(), Value::Array(cleaned));
        }

        if let Some(ref maybe_label) = patch.local_machine_label {
            match maybe_label.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(label) => {
                    scanner.insert(
                        "localMachineLabel".to_string(),
                        Value::String(label.to_string()),
                    );
                }
                None => {
                    scanner.remove("localMachineLabel");
                }
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&Value::Object(root_obj.clone()))
        .map_err(|err| format!("Failed to serialize settings.json: {err}"))?;
    bytes.push(b'\n');
    atomic_write_file(&write_target, &bytes).map_err(|err| err.to_string())?;

    Ok(extract_settings_state(
        &config_dir,
        &path,
        true,
        app_prefs_path,
        &root_obj,
    ))
}

fn ensure_child_object<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    entry.as_object_mut().expect("child is object")
}

fn extract_settings_state(
    config_dir: &Path,
    path: &Path,
    exists: bool,
    app_prefs_path: Option<&Path>,
    obj: &Map<String, Value>,
) -> TokscaleSettingsState {
    let scanner = obj.get("scanner").and_then(Value::as_object);
    let light = obj.get("light").and_then(Value::as_object);

    let bucket_timezone = scanner
        .and_then(|s| s.get("bucketTimezone"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    let local_machine_label = scanner
        .and_then(|s| s.get("localMachineLabel"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    let opencode_db_paths = scanner
        .and_then(|s| s.get("opencodeDbPaths"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut extra_scan_paths = BTreeMap::new();
    if let Some(map) = scanner
        .and_then(|s| s.get("extraScanPaths"))
        .and_then(Value::as_object)
    {
        for (client, val) in map {
            if let Some(arr) = val.as_array() {
                let paths: Vec<String> = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect();
                if !paths.is_empty() {
                    extra_scan_paths.insert(client.clone(), paths);
                }
            }
        }
    }

    let default_clients = obj
        .get("defaultClients")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut model_aliases = BTreeMap::new();
    if let Some(map) = obj.get("modelAliases").and_then(Value::as_object) {
        for (k, v) in map {
            if let Some(target) = v.as_str() {
                model_aliases.insert(k.clone(), target.to_string());
            }
        }
    }

    TokscaleSettingsState {
        path: path.to_string_lossy().into_owned(),
        exists,
        config_dir: config_dir.to_string_lossy().into_owned(),
        app_preferences_path: app_prefs_path.map(|p| p.to_string_lossy().into_owned()),
        bucket_timezone,
        default_clients,
        extra_scan_paths,
        opencode_db_paths,
        local_machine_label,
        include_unused_models: obj
            .get("includeUnusedModels")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        model_aliases,
        native_timeout_ms: obj
            .get("nativeTimeoutMs")
            .and_then(Value::as_u64)
            .unwrap_or(300_000),
        light_write_cache: light
            .and_then(|l| l.get("writeCache"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        auto_refresh_enabled: obj
            .get("autoRefreshEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        auto_refresh_ms: obj
            .get("autoRefreshMs")
            .and_then(Value::as_u64)
            .unwrap_or(60_000),
        minutely_tab_enabled: obj
            .get("minutelyTabEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        color_palette: obj
            .get("colorPalette")
            .and_then(Value::as_str)
            .unwrap_or("blue")
            .to_string(),
        tui_language: obj
            .get("tuiLanguage")
            .and_then(Value::as_str)
            .unwrap_or("en")
            .to_string(),
    }
}

pub async fn recover_timezone(service: &CliService) -> CliActionResult {
    run_cli_action(
        service,
        vec![
            "config".to_string(),
            "set".to_string(),
            "timezone".to_string(),
            "auto".to_string(),
        ],
        cli_commands::TIMEOUT_SHORT,
        "Timezone auto-detected",
    )
    .await
}

pub fn read_custom_pricing(env: &ProbeEnv) -> Result<CustomPricingState, String> {
    let config_dir = tokscale_config_dir(env)?;
    let path = config_dir.join(CUSTOM_PRICING_FILENAME);
    let exists = path.is_file();

    if !exists {
        return Ok(CustomPricingState {
            path: path.to_string_lossy().into_owned(),
            exists: false,
            entries: Vec::new(),
            valid: true,
            validation_message: None,
        });
    }

    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(val) => val,
        Err(err) => {
            return Ok(CustomPricingState {
                path: path.to_string_lossy().into_owned(),
                exists: true,
                entries: Vec::new(),
                valid: false,
                validation_message: Some(format!("Invalid JSON: {err}")),
            });
        }
    };

    let entries = parse_custom_pricing_entries(&parsed);
    Ok(CustomPricingState {
        path: path.to_string_lossy().into_owned(),
        exists: true,
        entries,
        valid: true,
        validation_message: None,
    })
}

fn parse_custom_pricing_entries(root: &Value) -> Vec<CustomPricingEntry> {
    let mut entries = Vec::new();

    if let Some(models) = root.get("models").and_then(Value::as_object) {
        for (model_id, pricing_val) in models {
            if let Some(pricing_obj) = pricing_val.as_object() {
                entries.push(CustomPricingEntry {
                    scope: "model".to_string(),
                    provider_id: None,
                    model_id: model_id.clone(),
                    input_per_million: extract_per_million(
                        pricing_obj,
                        "input_cost_per_million_tokens",
                        "input_cost_per_token",
                    ),
                    output_per_million: extract_per_million(
                        pricing_obj,
                        "output_cost_per_million_tokens",
                        "output_cost_per_token",
                    ),
                    cache_read_per_million: extract_per_million(
                        pricing_obj,
                        "cache_read_input_token_cost_per_million_tokens",
                        "cache_read_input_token_cost",
                    ),
                    cache_write_per_million: extract_per_million(
                        pricing_obj,
                        "cache_creation_input_token_cost_per_million_tokens",
                        "cache_creation_input_token_cost",
                    ),
                    notes: pricing_obj
                        .get("notes")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    source: pricing_obj
                        .get("source")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                });
            }
        }
    }

    if let Some(overrides) = root.get("provider_overrides").and_then(Value::as_array) {
        for item in overrides {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let Some(provider_id) = obj.get("provider_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(model_id) = obj.get("model_id").and_then(Value::as_str) else {
                continue;
            };
            let pricing_obj = obj.get("pricing").and_then(Value::as_object);
            entries.push(CustomPricingEntry {
                scope: "provider".to_string(),
                provider_id: Some(provider_id.to_string()),
                model_id: model_id.to_string(),
                input_per_million: pricing_obj.and_then(|p| {
                    extract_per_million(p, "input_cost_per_million_tokens", "input_cost_per_token")
                }),
                output_per_million: pricing_obj.and_then(|p| {
                    extract_per_million(p, "output_cost_per_million_tokens", "output_cost_per_token")
                }),
                cache_read_per_million: pricing_obj.and_then(|p| {
                    extract_per_million(
                        p,
                        "cache_read_input_token_cost_per_million_tokens",
                        "cache_read_input_token_cost",
                    )
                }),
                cache_write_per_million: pricing_obj.and_then(|p| {
                    extract_per_million(
                        p,
                        "cache_creation_input_token_cost_per_million_tokens",
                        "cache_creation_input_token_cost",
                    )
                }),
                notes: obj
                    .get("notes")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                source: obj
                    .get("source")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            });
        }
    }

    entries
}

fn extract_per_million(obj: &Map<String, Value>, per_m_key: &str, per_tok_key: &str) -> Option<f64> {
    if let Some(v) = obj.get(per_m_key).and_then(Value::as_f64) {
        return Some(v);
    }
    obj.get(per_tok_key)
        .and_then(Value::as_f64)
        .map(|v| (v * 1_000_000.0 * 10000.0).round() / 10000.0)
}

pub async fn save_custom_pricing(
    env: &ProbeEnv,
    service: &CliService,
    entries: &[CustomPricingEntry],
) -> Result<CustomPricingState, String> {
    let config_dir = tokscale_config_dir(env)?;
    let path = config_dir.join(CUSTOM_PRICING_FILENAME);
    let write_target = resolve_write_target(&path);

    let mut root_obj: Map<String, Value> = if path.is_file() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|val| val.as_object().cloned())
            .unwrap_or_default()
    } else {
        Map::new()
    };

    let mut models_map = Map::new();
    let mut provider_overrides = Vec::new();

    for entry in entries {
        let model_id = entry.model_id.trim();
        if model_id.is_empty() {
            return Err("Model ID cannot be empty".to_string());
        }

        let pricing = build_pricing_object(entry)?;

        if entry.scope == "provider" {
            let provider_id = entry
                .provider_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!("Provider ID is required for provider-scoped override '{model_id}'")
                })?;
            let mut item = Map::new();
            item.insert(
                "provider_id".to_string(),
                Value::String(provider_id.to_string()),
            );
            item.insert("model_id".to_string(), Value::String(model_id.to_string()));
            item.insert("pricing".to_string(), Value::Object(pricing));
            if let Some(notes) = entry.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                item.insert("notes".to_string(), Value::String(notes.to_string()));
            }
            if let Some(source) = entry.source.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                item.insert("source".to_string(), Value::String(source.to_string()));
            }
            provider_overrides.push(Value::Object(item));
        } else {
            models_map.insert(model_id.to_string(), Value::Object(pricing));
        }
    }

    root_obj.insert("models".to_string(), Value::Object(models_map));
    if provider_overrides.is_empty() {
        root_obj.remove("provider_overrides");
    } else {
        root_obj.insert(
            "provider_overrides".to_string(),
            Value::Array(provider_overrides),
        );
    }

    let mut bytes = serde_json::to_vec_pretty(&Value::Object(root_obj))
        .map_err(|err| format!("Failed to serialize custom-pricing.json: {err}"))?;
    bytes.push(b'\n');
    atomic_write_file(&write_target, &bytes).map_err(|err| err.to_string())?;

    let mut state = read_custom_pricing(env)?;
    match service
        .run_raw(
            vec![
                "pricing".to_string(),
                "validate-overrides".to_string(),
                "--strict".to_string(),
                "--no-spinner".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(_) => {
            state.valid = true;
            state.validation_message = Some("Validated by tokscale CLI".to_string());
        }
        Err(CliError::NotFound) => {
            state.valid = true;
            state.validation_message = Some("Saved (CLI unavailable for strict validation)".to_string());
        }
        Err(err) => {
            state.valid = false;
            state.validation_message = Some(short_cli_error(&err));
        }
    }

    Ok(state)
}

fn build_pricing_object(entry: &CustomPricingEntry) -> Result<Map<String, Value>, String> {
    let mut obj = Map::new();
    let pairs = [
        ("input_cost_per_million_tokens", entry.input_per_million),
        ("output_cost_per_million_tokens", entry.output_per_million),
        (
            "cache_read_input_token_cost_per_million_tokens",
            entry.cache_read_per_million,
        ),
        (
            "cache_creation_input_token_cost_per_million_tokens",
            entry.cache_write_per_million,
        ),
    ];

    for (key, maybe_val) in pairs {
        if let Some(val) = maybe_val {
            if !val.is_finite() || val < 0.0 {
                return Err(format!(
                    "Invalid rate for {}: must be a non-negative finite number",
                    entry.model_id
                ));
            }
            obj.insert(key.to_string(), json!(val));
        }
    }

    if obj.is_empty() {
        obj.insert("input_cost_per_million_tokens".to_string(), json!(0.0));
        obj.insert("output_cost_per_million_tokens".to_string(), json!(0.0));
    }

    Ok(obj)
}

pub async fn lookup_model_pricing(
    service: &CliService,
    model_id: &str,
) -> Result<ModelPriceLookupResult, String> {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return Err("Enter a model ID to look up".to_string());
    }

    let argv = vec![
        "pricing".to_string(),
        trimmed.to_string(),
        "--json".to_string(),
        "--offline".to_string(),
        "--no-spinner".to_string(),
    ];

    let val: Value = service
        .run_json(argv, cli_commands::TIMEOUT_SHORT)
        .await
        .map_err(|err| short_cli_error(&err))?;

    let pricing = val.get("pricing").and_then(Value::as_object);
    let to_per_m = |key: &str| {
        pricing
            .and_then(|p| p.get(key))
            .and_then(Value::as_f64)
            .map(|v| (v * 1_000_000.0 * 10000.0).round() / 10000.0)
    };

    Ok(ModelPriceLookupResult {
        model_id: val
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or(trimmed)
            .to_string(),
        matched_key: val
            .get("matchedKey")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source: val
            .get("source")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        scope: val
            .get("scope")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        input_per_million: to_per_m("inputCostPerToken"),
        output_per_million: to_per_m("outputCostPerToken"),
        cache_read_per_million: to_per_m("cacheReadInputTokenCost"),
        cache_write_per_million: to_per_m("cacheCreationInputTokenCost"),
        submission_safe: val
            .get("resolution")
            .and_then(|r| r.get("submissionSafe"))
            .and_then(Value::as_bool),
    })
}

pub async fn refresh_pricing_catalog(service: &CliService) -> CliActionResult {
    run_cli_action(
        service,
        vec![
            "pricing".to_string(),
            "refresh".to_string(),
            "--no-spinner".to_string(),
        ],
        cli_commands::TIMEOUT_DATA,
        "Pricing catalogs refreshed",
    )
    .await
}

pub async fn get_providers_status(service: &CliService) -> ProvidersStatusPayload {
    let mut list = Vec::new();
    list.push(query_cursor_status(service).await);
    list.push(query_codex_status(service).await);
    list.push(query_trae_status(service).await);
    list.push(query_warp_status(service).await);
    list.push(query_capy_status(service).await);
    list.push(query_antigravity_status(service).await);
    list.push(ProviderStatusItem {
        id: "amp".to_string(),
        label: "Amp".to_string(),
        connected: false,
        supports_multi_account: false,
        summary: "Exports threads via local amp CLI".to_string(),
        last_synced_at: None,
        accounts: Vec::new(),
        diagnostics: Vec::new(),
        variants: Vec::new(),
    });
    list.push(ProviderStatusItem {
        id: "hindsight".to_string(),
        label: "Hindsight".to_string(),
        connected: false,
        supports_multi_account: false,
        summary: "Syncs LLM request logs from Hindsight API".to_string(),
        last_synced_at: None,
        accounts: Vec::new(),
        diagnostics: Vec::new(),
        variants: Vec::new(),
    });

    ProvidersStatusPayload { providers: list }
}

async fn query_cursor_status(service: &CliService) -> ProviderStatusItem {
    let fallback_cred = providers::is_provider_credentialed("cursor", service.env());
    let mut accounts = Vec::new();
    let mut diagnostics = Vec::new();

    match service
        .run_json::<Value>(
            vec![
                "cursor".to_string(),
                "accounts".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            accounts = parse_account_list(&val);
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    let connected = !accounts.is_empty() || fallback_cred;
    let summary = if !accounts.is_empty() {
        let active = accounts
            .iter()
            .find(|a| a.active)
            .map(|a| a.name.as_str())
            .unwrap_or(&accounts[0].name);
        format!("{} saved · active: {active}", accounts.len())
    } else if connected {
        "Connected".to_string()
    } else {
        "Not connected".to_string()
    };

    ProviderStatusItem {
        id: "cursor".to_string(),
        label: "Cursor".to_string(),
        connected,
        supports_multi_account: true,
        summary,
        last_synced_at: None,
        accounts,
        diagnostics,
        variants: Vec::new(),
    }
}

async fn query_codex_status(service: &CliService) -> ProviderStatusItem {
    let mut accounts = Vec::new();
    let mut diagnostics = Vec::new();

    match service
        .run_json::<Value>(
            vec![
                "codex".to_string(),
                "accounts".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            accounts = parse_account_list(&val);
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    let connected = !accounts.is_empty();
    let summary = if connected {
        let active = accounts
            .iter()
            .find(|a| a.active)
            .map(|a| a.name.as_str())
            .unwrap_or(&accounts[0].name);
        format!("{} saved · active: {active}", accounts.len())
    } else {
        "No saved Codex accounts".to_string()
    };

    ProviderStatusItem {
        id: "codex".to_string(),
        label: "Codex".to_string(),
        connected,
        supports_multi_account: true,
        summary,
        last_synced_at: None,
        accounts,
        diagnostics,
        variants: Vec::new(),
    }
}

async fn query_trae_status(service: &CliService) -> ProviderStatusItem {
    let mut variants = Vec::new();
    let mut diagnostics = Vec::new();
    let mut connected = providers::is_provider_credentialed("trae", service.env());

    match service
        .run_json::<Value>(
            vec![
                "trae".to_string(),
                "status".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            if let Some(obj) = val.as_object() {
                for (k, v) in obj {
                    if v.as_bool() == Some(true) {
                        variants.push(k.clone());
                    }
                }
                connected = !variants.is_empty();
            }
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    let summary = if !variants.is_empty() {
        format!("Connected ({})", variants.join(", "))
    } else if connected {
        "Connected".to_string()
    } else {
        "Not connected".to_string()
    };

    ProviderStatusItem {
        id: "trae".to_string(),
        label: "Trae IDE".to_string(),
        connected,
        supports_multi_account: false,
        summary,
        last_synced_at: None,
        accounts: Vec::new(),
        diagnostics,
        variants,
    }
}

async fn query_warp_status(service: &CliService) -> ProviderStatusItem {
    let mut connected = providers::is_provider_credentialed("warp", service.env());
    let mut diagnostics = Vec::new();
    let mut summary = "Not connected".to_string();

    match service
        .run_json::<Value>(
            vec![
                "warp".to_string(),
                "status".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            connected = val
                .get("hasCredentials")
                .and_then(Value::as_bool)
                .unwrap_or(connected);
            let has_cache = val.get("hasCache").and_then(Value::as_bool).unwrap_or(false);
            let workspaces = val
                .get("workspaceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if let Some(arr) = val.get("diagnostics").and_then(Value::as_array) {
                diagnostics = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect();
            }
            summary = if connected && has_cache {
                format!("Connected · {workspaces} workspace(s) cached")
            } else if connected {
                "Connected · cache not synced yet".to_string()
            } else {
                "Not connected".to_string()
            };
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    ProviderStatusItem {
        id: "warp".to_string(),
        label: "Warp / Oz".to_string(),
        connected,
        supports_multi_account: false,
        summary,
        last_synced_at: None,
        accounts: Vec::new(),
        diagnostics,
        variants: Vec::new(),
    }
}

async fn query_capy_status(service: &CliService) -> ProviderStatusItem {
    let mut connected = providers::is_provider_credentialed("capy", service.env());
    let mut diagnostics = Vec::new();
    let mut last_synced_at = None;
    let mut summary = "Not connected".to_string();

    match service
        .run_json::<Value>(
            vec![
                "capy".to_string(),
                "status".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            connected = val
                .get("hasCredentials")
                .and_then(Value::as_bool)
                .unwrap_or(connected);
            let rows = val.get("rows").and_then(Value::as_u64).unwrap_or(0);
            last_synced_at = val
                .get("syncedAt")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if let Some(arr) = val.get("diagnostics").and_then(Value::as_array) {
                diagnostics = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect();
            }
            summary = if connected {
                format!("Connected · {rows} usage row(s)")
            } else {
                "Not connected".to_string()
            };
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    ProviderStatusItem {
        id: "capy".to_string(),
        label: "Capy".to_string(),
        connected,
        supports_multi_account: false,
        summary,
        last_synced_at,
        accounts: Vec::new(),
        diagnostics,
        variants: Vec::new(),
    }
}

async fn query_antigravity_status(service: &CliService) -> ProviderStatusItem {
    let mut connected = false;
    let mut diagnostics = Vec::new();
    let mut last_synced_at = None;
    let mut summary = "No running language servers detected".to_string();

    match service
        .run_json::<Value>(
            vec![
                "antigravity".to_string(),
                "status".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => {
            let connections = val
                .get("detectedConnections")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let cached_sessions = val
                .get("cachedSessions")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            last_synced_at = val
                .get("lastSyncedAt")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            connected = connections > 0 || cached_sessions > 0;
            summary = if connections > 0 {
                format!("{connections} server(s) detected · {cached_sessions} cached session(s)")
            } else if cached_sessions > 0 {
                format!("{cached_sessions} cached session(s)")
            } else {
                "No running language servers detected".to_string()
            };
        }
        Err(err) => {
            diagnostics.push(short_cli_error(&err));
        }
    }

    ProviderStatusItem {
        id: "antigravity".to_string(),
        label: "Antigravity".to_string(),
        connected,
        supports_multi_account: false,
        summary,
        last_synced_at,
        accounts: Vec::new(),
        diagnostics,
        variants: Vec::new(),
    }
}

fn parse_account_list(val: &Value) -> Vec<ProviderAccountInfo> {
    let Some(arr) = val.get("accounts").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let name = obj
                .get("name")
                .or_else(|| obj.get("label"))
                .or_else(|| obj.get("id"))
                .and_then(Value::as_str)?
                .to_string();
            let active = obj
                .get("active")
                .or_else(|| obj.get("isActive"))
                .or_else(|| obj.get("is_active"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let email = obj
                .get("email")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let plan = obj
                .get("plan")
                .or_else(|| obj.get("planType"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            Some(ProviderAccountInfo {
                name,
                active,
                email,
                plan,
            })
        })
        .collect()
}

pub fn build_provider_action_argv(req: &ProviderActionRequest) -> Result<Vec<String>, String> {
    let provider = req.provider.trim();
    let action = req.action.trim();
    let account_name = req
        .account_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match (provider, action) {
        ("cursor", "login") => {
            let mut argv = vec!["cursor".into(), "login".into()];
            if let Some(name) = account_name {
                argv.push("--name".into());
                argv.push(name.into());
            }
            Ok(argv)
        }
        ("cursor", "sync") => Ok(vec!["cursor".into(), "sync".into()]),
        ("cursor", "switch") => {
            let name = account_name.ok_or("Account name is required to switch Cursor account")?;
            Ok(vec!["cursor".into(), "switch".into(), name.into()])
        }
        ("cursor", "logout") => {
            let mut argv = vec!["cursor".into(), "logout".into()];
            if req.all_accounts {
                argv.push("--all".into());
            } else if let Some(name) = account_name {
                argv.push("--name".into());
                argv.push(name.into());
            }
            if req.purge_cache {
                argv.push("--purge-cache".into());
            }
            Ok(argv)
        }
        ("codex", "import") => {
            let mut argv = vec!["codex".into(), "import".into()];
            if let Some(name) = account_name {
                argv.push("--name".into());
                argv.push(name.into());
            }
            Ok(argv)
        }
        ("codex", "switch") => {
            let name = account_name.ok_or("Account name is required to switch Codex account")?;
            Ok(vec!["codex".into(), "switch".into(), name.into()])
        }
        ("codex", "remove") => {
            let name = account_name.ok_or("Account name is required to remove Codex account")?;
            Ok(vec!["codex".into(), "remove".into(), name.into()])
        }
        ("codex", "status") => {
            let mut argv = vec!["codex".into(), "status".into()];
            if let Some(name) = account_name {
                argv.push("--name".into());
                argv.push(name.into());
            }
            Ok(argv)
        }
        ("trae", "login") => {
            let mut argv = vec!["trae".into(), "login".into()];
            if let Some(variant) = req.variant.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                argv.push("--variant".into());
                argv.push(variant.into());
            }
            Ok(argv)
        }
        ("trae", "sync") => Ok(vec!["trae".into(), "sync".into()]),
        ("trae", "logout") => {
            let mut argv = vec!["trae".into(), "logout".into()];
            if let Some(variant) = req.variant.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                argv.push("--variant".into());
                argv.push(variant.into());
            }
            Ok(argv)
        }
        ("warp", "login") => {
            let token = req
                .token
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("Warp bearer token or cookie header is required")?;
            let mut argv = vec![
                "warp".into(),
                "login".into(),
                "--token".into(),
                token.into(),
            ];
            if req.is_cookie {
                argv.push("--cookie".into());
            }
            Ok(argv)
        }
        ("warp", "sync") => Ok(vec!["warp".into(), "sync".into()]),
        ("warp", "logout") => {
            let mut argv = vec!["warp".into(), "logout".into()];
            if req.purge_cache {
                argv.push("--purge-cache".into());
            }
            Ok(argv)
        }
        ("capy", "login") => {
            let key = req
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("Capy API key is required")?;
            let mut argv = vec!["capy".into(), "login".into(), "--key".into(), key.into()];
            for user in &req.users {
                let u = user.trim();
                if !u.is_empty() {
                    argv.push("--user".into());
                    argv.push(u.into());
                }
            }
            Ok(argv)
        }
        ("capy", "sync") => Ok(vec!["capy".into(), "sync".into()]),
        ("capy", "logout") => Ok(vec!["capy".into(), "logout".into()]),
        ("antigravity", "sync") => Ok(vec!["antigravity".into(), "sync".into()]),
        ("antigravity", "purge_cache") => Ok(vec!["antigravity".into(), "purge-cache".into()]),
        ("amp", "sync") => Ok(vec!["amp".into(), "sync".into(), "--json".into()]),
        ("hindsight", "sync") => {
            let mut argv = vec!["hindsight".into(), "sync".into(), "--json".into()];
            if let Some(url) = req.api_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                argv.push("--api".into());
                argv.push(url.into());
            }
            if let Some(tenant) = req.tenant.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                argv.push("--tenant".into());
                argv.push(tenant.into());
            }
            if let Some(token) = req.token.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                argv.push("--token".into());
                argv.push(token.into());
            }
            Ok(argv)
        }
        _ => Err(format!(
            "Unsupported action '{action}' for provider '{provider}'"
        )),
    }
}

pub async fn run_provider_action(
    service: &CliService,
    req: &ProviderActionRequest,
) -> CliActionResult {
    let argv = match build_provider_action_argv(req) {
        Ok(argv) => argv,
        Err(msg) => {
            return CliActionResult {
                ok: false,
                message: msg,
                output: None,
            };
        }
    };

    let timeout = if req.action == "sync" {
        cli_commands::TIMEOUT_LONG
    } else {
        cli_commands::TIMEOUT_DATA
    };

    let fallback_msg = format!("{} {} completed", req.provider, req.action);
    run_cli_action(service, argv, timeout, &fallback_msg).await
}

pub async fn get_social_status(service: &CliService) -> SocialStatusPayload {
    let (logged_in, username, detail) = match service
        .run_raw(vec!["whoami".to_string()], cli_commands::TIMEOUT_SHORT)
        .await
    {
        Ok(out) => parse_whoami_output(&out),
        Err(err) => (false, None, Some(short_cli_error(&err))),
    };

    let autosubmit = match service
        .run_json::<Value>(
            vec![
                "autosubmit".to_string(),
                "status".to_string(),
                "--json".to_string(),
            ],
            cli_commands::TIMEOUT_SHORT,
        )
        .await
    {
        Ok(val) => parse_autosubmit_status(&val),
        Err(_) => AutosubmitStatus {
            interval_minutes: 1440,
            ..AutosubmitStatus::default()
        },
    };

    SocialStatusPayload {
        logged_in,
        username,
        detail,
        autosubmit,
    }
}

fn parse_whoami_output(raw: &str) -> (bool, Option<String>, Option<String>) {
    let cleaned: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let joined = cleaned.join(" ");
    let lower = joined.to_ascii_lowercase();
    if lower.contains("not logged in") || cleaned.is_empty() {
        return (false, None, Some("Not logged in".to_string()));
    }

    for line in &cleaned {
        if let Some(rest) = line
            .strip_prefix("Logged in as")
            .or_else(|| line.strip_prefix("Username:"))
            .or_else(|| line.strip_prefix("User:"))
        {
            let candidate = rest
                .trim()
                .trim_start_matches(':')
                .trim()
                .trim_start_matches('@')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if !candidate.is_empty() {
                return (true, Some(candidate.to_string()), Some(joined));
            }
        }
    }

    (true, Some(cleaned[0].to_string()), Some(joined))
}

fn parse_autosubmit_status(val: &Value) -> AutosubmitStatus {
    AutosubmitStatus {
        enabled: val.get("enabled").and_then(Value::as_bool).unwrap_or(false),
        interval_minutes: val
            .get("intervalMinutes")
            .and_then(Value::as_u64)
            .unwrap_or(1440),
        scheduler: val
            .get("scheduler")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        clients: val
            .get("clients")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        today: val.get("today").and_then(Value::as_bool).unwrap_or(false),
        yesterday: val
            .get("yesterday")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        week: val.get("week").and_then(Value::as_bool).unwrap_or(false),
        month: val.get("month").and_then(Value::as_bool).unwrap_or(false),
        since: val
            .get("since")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        until: val
            .get("until")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        year: val
            .get("year")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        last_run_at_ms: val.get("lastRunAtMs").and_then(Value::as_u64),
        last_error: val
            .get("lastError")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    }
}

pub fn build_social_action_argv(req: &SocialActionRequest) -> Result<Vec<String>, String> {
    match req.action.as_str() {
        "login_browser" => Ok(vec!["login".into()]),
        "login_token" => {
            let token = req
                .token
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("Tokscale API token is required")?;
            Ok(vec!["login".into(), "--token".into(), token.into()])
        }
        "logout" => Ok(vec!["logout".into()]),
        "submit_dry_run" | "submit" => {
            let mut argv = vec!["submit".into()];
            if req.action == "submit_dry_run" {
                argv.push("--dry-run".into());
            }
            append_filter_flags(&mut argv, &req.clients, req.range.as_deref());
            Ok(argv)
        }
        "autosubmit_run" => Ok(vec!["autosubmit".into(), "run".into()]),
        other => Err(format!("Unsupported social action '{other}'")),
    }
}

pub async fn run_social_action(
    service: &CliService,
    req: &SocialActionRequest,
) -> CliActionResult {
    let argv = match build_social_action_argv(req) {
        Ok(argv) => argv,
        Err(msg) => {
            return CliActionResult {
                ok: false,
                message: msg,
                output: None,
            };
        }
    };

    let timeout = match req.action.as_str() {
        "login_browser" | "submit" | "autosubmit_run" => cli_commands::TIMEOUT_LONG,
        _ => cli_commands::TIMEOUT_DATA,
    };

    run_cli_action(service, argv, timeout, "Action completed").await
}

pub fn build_autosubmit_config_argv(req: &AutosubmitConfigRequest) -> Vec<String> {
    if !req.enabled {
        return vec!["autosubmit".into(), "disable".into()];
    }
    let mut argv = vec!["autosubmit".into(), "enable".into()];
    if let Some(interval) = req.interval.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        argv.push("--interval".into());
        argv.push(interval.into());
    }
    append_filter_flags(&mut argv, &req.clients, req.range.as_deref());
    argv
}

pub async fn configure_autosubmit(
    service: &CliService,
    req: &AutosubmitConfigRequest,
) -> CliActionResult {
    let argv = build_autosubmit_config_argv(req);
    let fallback = if req.enabled {
        "Autosubmit enabled"
    } else {
        "Autosubmit disabled"
    };
    run_cli_action(service, argv, cli_commands::TIMEOUT_SHORT, fallback).await
}

fn append_filter_flags(argv: &mut Vec<String>, clients: &[String], range: Option<&str>) {
    let cleaned_clients: Vec<&str> = clients
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if !cleaned_clients.is_empty() {
        argv.push("-c".into());
        argv.push(cleaned_clients.join(","));
    }

    match range.map(str::trim) {
        Some("today") => argv.push("--today".into()),
        Some("yesterday") => argv.push("--yesterday".into()),
        Some("week") | Some("7d") => argv.push("--week".into()),
        Some("month") | Some("30d") => argv.push("--month".into()),
        _ => {}
    }
}

async fn run_cli_action(
    service: &CliService,
    argv: Vec<String>,
    timeout: std::time::Duration,
    fallback_ok_message: &str,
) -> CliActionResult {
    match service.run_raw(argv, timeout).await {
        Ok(stdout) => {
            let trimmed = stdout.trim();
            let first_line = trimmed
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or(fallback_ok_message);
            CliActionResult {
                ok: true,
                message: first_line.to_string(),
                output: if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                },
            }
        }
        Err(err) => CliActionResult {
            ok: false,
            message: short_cli_error(&err),
            output: None,
        },
    }
}

fn short_cli_error(err: &CliError) -> String {
    match err {
        CliError::NonZeroExit { stderr, .. } => stderr
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| err.to_string()),
        other => other.to_string(),
    }
}

pub fn reveal_path(target: &Path) -> Result<(), String> {
    let reveal_target = if target.exists() {
        target.to_path_buf()
    } else if let Some(parent) = target.parent().filter(|p| p.exists()) {
        parent.to_path_buf()
    } else {
        return Err(format!("Path does not exist: {}", target.display()));
    };

    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        if reveal_target.is_file() {
            cmd.arg("-R");
        }
        cmd.arg(&reveal_target)
            .status()
            .map_err(|err| format!("Failed to reveal path: {err}"))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        if reveal_target.is_file() {
            cmd.arg(format!("/select,{}", reveal_target.display()));
        } else {
            cmd.arg(&reveal_target);
        }
        cmd.status()
            .map_err(|err| format!("Failed to reveal path: {err}"))?;
        Ok(())
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let open_dir = if reveal_target.is_file() {
            reveal_target.parent().unwrap_or(&reveal_target)
        } else {
            &reveal_target
        };
        std::process::Command::new("xdg-open")
            .arg(open_dir)
            .status()
            .map_err(|err| format!("Failed to open path: {err}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;

    use super::*;
    #[cfg(unix)]
    use crate::cli::locate::write_test_script;
    use crate::contracts::CompanionPreferences;

    #[test]
    fn patch_tokscale_settings_preserves_unknown_keys_and_updates_nested_fields() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let cfg_dir = home.join(".config/tokscale");
        fs::create_dir_all(&cfg_dir).unwrap();

        let initial = json!({
            "colorPalette": "dusk",
            "autosubmit": { "enabled": true, "intervalMinutes": 120 },
            "futureUnknown": { "keepMe": 42 },
            "scanner": {
                "bucketTimezone": "Europe/Berlin",
                "customFutureScannerKey": "preserve"
            }
        });
        fs::write(
            cfg_dir.join("settings.json"),
            serde_json::to_vec_pretty(&initial).unwrap(),
        )
        .unwrap();

        let env = ProbeEnv::isolated(home);
        let mut extra = BTreeMap::new();
        extra.insert("codex".to_string(), vec!["/work/project/.codex".to_string()]);
        let mut aliases = BTreeMap::new();
        aliases.insert("big-pickle".to_string(), "claude-opus-4-6".to_string());

        let updated = patch_tokscale_settings(
            &env,
            None,
            &TokscaleSettingsPatch {
                default_clients: Some(vec!["claude".to_string(), "opencode".to_string()]),
                extra_scan_paths: Some(extra.clone()),
                opencode_db_paths: Some(vec!["/tmp/opencode.db".to_string()]),
                local_machine_label: Some(Some("macbook-pro".to_string())),
                include_unused_models: Some(true),
                model_aliases: Some(aliases.clone()),
                native_timeout_ms: Some(450_000),
                light_write_cache: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.bucket_timezone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(updated.default_clients, vec!["claude", "opencode"]);
        assert_eq!(updated.extra_scan_paths, extra);
        assert_eq!(updated.opencode_db_paths, vec!["/tmp/opencode.db"]);
        assert_eq!(updated.local_machine_label.as_deref(), Some("macbook-pro"));
        assert!(updated.include_unused_models);
        assert_eq!(updated.model_aliases, aliases);
        assert_eq!(updated.native_timeout_ms, 450_000);
        assert!(updated.light_write_cache);

        let raw: Value =
            serde_json::from_str(&fs::read_to_string(cfg_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(raw["futureUnknown"]["keepMe"], json!(42));
        assert_eq!(raw["autosubmit"]["intervalMinutes"], json!(120));
        assert_eq!(raw["scanner"]["bucketTimezone"], json!("Europe/Berlin"));
        assert_eq!(
            raw["scanner"]["customFutureScannerKey"],
            json!("preserve")
        );
    }

    #[test]
    fn provider_and_autosubmit_argv_builders_handle_flags_safely() {
        let warp = build_provider_action_argv(&ProviderActionRequest {
            provider: "warp".into(),
            action: "login".into(),
            token: Some("tok_secret".into()),
            is_cookie: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(warp, vec!["warp", "login", "--token", "tok_secret", "--cookie"]);

        let capy = build_provider_action_argv(&ProviderActionRequest {
            provider: "capy".into(),
            action: "login".into(),
            api_key: Some("capy_key".into()),
            users: vec!["alice@example.com".into()],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            capy,
            vec!["capy", "login", "--key", "capy_key", "--user", "alice@example.com"]
        );

        let auto_on = build_autosubmit_config_argv(&AutosubmitConfigRequest {
            enabled: true,
            interval: Some("6h".into()),
            clients: vec!["claude".into(), "codex".into()],
            range: Some("week".into()),
        });
        assert_eq!(
            auto_on,
            vec!["autosubmit", "enable", "--interval", "6h", "-c", "claude,codex", "--week"]
        );

        let auto_off = build_autosubmit_config_argv(&AutosubmitConfigRequest {
            enabled: false,
            ..Default::default()
        });
        assert_eq!(auto_off, vec!["autosubmit", "disable"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn custom_pricing_round_trip_and_strict_validation() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let script = bin_dir.join("tokscale");
        write_test_script(&script, "#!/bin/sh\necho 'ok'\n");

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env.clone());

        let entries = vec![
            CustomPricingEntry {
                scope: "model".into(),
                provider_id: None,
                model_id: "local/qwen-coder".into(),
                input_per_million: Some(0.0),
                output_per_million: Some(0.0),
                cache_read_per_million: Some(0.0),
                cache_write_per_million: Some(0.0),
                notes: None,
                source: None,
            },
            CustomPricingEntry {
                scope: "provider".into(),
                provider_id: Some("fireworks".into()),
                model_id: "kimi-k2".into(),
                input_per_million: Some(2.0),
                output_per_million: Some(8.0),
                cache_read_per_million: Some(0.3),
                cache_write_per_million: None,
                notes: Some("custom fireworks rate".into()),
                source: None,
            },
        ];

        let saved = save_custom_pricing(&env, &service, &entries).await.unwrap();
        assert!(saved.exists);
        assert!(saved.valid);
        assert_eq!(saved.entries.len(), 2);
        assert_eq!(saved.entries[0].model_id, "local/qwen-coder");
        assert_eq!(saved.entries[1].provider_id.as_deref(), Some("fireworks"));
        assert_eq!(saved.entries[1].output_per_million, Some(8.0));
    }
}
