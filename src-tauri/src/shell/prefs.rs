use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::contracts::CompanionPreferences;
use crate::shell::ShellError;

pub const PREFERENCES_FILENAME: &str = "preferences.json";
const TEMP_EXTENSION: &str = "json.tmp";

pub fn preferences_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PREFERENCES_FILENAME)
}

pub fn temp_file_path(target_path: &Path) -> PathBuf {
    target_path.with_extension(TEMP_EXTENSION)
}

pub fn atomic_write_file(target_path: &Path, bytes: &[u8]) -> Result<(), ShellError> {
    atomic_write_file_with(target_path, bytes, |file, data| {
        file.write_all(data)?;
        file.sync_all()?;
        Ok(())
    })
}

pub fn atomic_write_file_with<W>(
    target_path: &Path,
    bytes: &[u8],
    write_and_sync: W,
) -> Result<(), ShellError>
where
    W: FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
{
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = temp_file_path(target_path);
    let write_result = (|| -> std::io::Result<()> {
        let mut tmp_file = File::create(&tmp_path)?;
        write_and_sync(&mut tmp_file, bytes)?;
        drop(tmp_file);
        fs::rename(&tmp_path, target_path)?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(ShellError::Io(err));
    }

    Ok(())
}

pub fn load_preferences(config_dir: &Path) -> Result<CompanionPreferences, ShellError> {
    let path = preferences_file_path(config_dir);
    if !path.exists() {
        return Ok(CompanionPreferences::default());
    }

    let raw = fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let obj = value
        .as_object()
        .ok_or_else(|| ShellError::Preferences("preferences.json must be a JSON object".into()))?;

    let merged = build_merged_object(Some(obj), None)?;
    let prefs: CompanionPreferences = serde_json::from_value(Value::Object(merged))?;
    validate_preferences(&prefs)?;
    Ok(prefs)
}

pub fn patch_preferences(
    config_dir: &Path,
    patch: &Value,
) -> Result<CompanionPreferences, ShellError> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| ShellError::Preferences("preferences patch must be a JSON object".into()))?;

    let path = preferences_file_path(config_dir);
    let existing_obj = if path.exists() {
        let raw = fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&raw)?;
        Some(
            value
                .as_object()
                .ok_or_else(|| {
                    ShellError::Preferences("preferences.json must be a JSON object".into())
                })?
                .clone(),
        )
    } else {
        None
    };

    let merged = build_merged_object(existing_obj.as_ref(), Some(patch_obj))?;
    let prefs: CompanionPreferences = serde_json::from_value(Value::Object(merged.clone()))?;
    validate_preferences(&prefs)?;

    let mut bytes = serde_json::to_vec_pretty(&Value::Object(merged))?;
    bytes.push(b'\n');
    atomic_write_file(&path, &bytes)?;

    Ok(prefs)
}

fn build_merged_object(
    existing: Option<&Map<String, Value>>,
    patch: Option<&Map<String, Value>>,
) -> Result<Map<String, Value>, ShellError> {
    let defaults_value = serde_json::to_value(CompanionPreferences::default())?;
    let mut merged = defaults_value
        .as_object()
        .cloned()
        .unwrap_or_default();

    // Ensure nullable default keys exist explicitly when serialized for clarity.
    merged.entry("globalShortcut".to_string()).or_insert(Value::Null);
    merged.entry("dailyBudgetUsd".to_string()).or_insert(Value::Null);
    merged.entry("customTokscalePath".to_string()).or_insert(Value::Null);

    if let Some(existing_map) = existing {
        for (key, value) in existing_map {
            merged.insert(key.clone(), value.clone());
        }
    }

    if let Some(patch_map) = patch {
        for (key, value) in patch_map {
            merged.insert(key.clone(), value.clone());
        }
    }

    Ok(merged)
}

fn validate_preferences(prefs: &CompanionPreferences) -> Result<(), ShellError> {
    match prefs.menubar_display.as_str() {
        "cost" | "tokens" | "icon" => {}
        other => {
            return Err(ShellError::Preferences(format!(
                "invalid menubarDisplay '{other}': expected 'cost', 'tokens', or 'icon'"
            )));
        }
    }

    match prefs.theme.as_str() {
        "light" | "dark" | "system" => {}
        other => {
            return Err(ShellError::Preferences(format!(
                "invalid theme '{other}': expected 'light', 'dark', or 'system'"
            )));
        }
    }

    if prefs.quota_alert_percent > 100 {
        return Err(ShellError::Preferences(format!(
            "invalid quotaAlertPercent {}: must be between 0 and 100",
            prefs.quota_alert_percent
        )));
    }

    if let Some(budget) = prefs.daily_budget_usd {
        if !budget.is_finite() || budget < 0.0 {
            return Err(ShellError::Preferences(format!(
                "invalid dailyBudgetUsd {budget}: must be a non-negative finite number"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Error;
    use tempfile::TempDir;

    #[test]
    fn load_returns_defaults_when_missing() {
        let dir = TempDir::new().unwrap();
        let prefs = load_preferences(dir.path()).unwrap();
        assert_eq!(prefs, CompanionPreferences::default());
        assert!(!preferences_file_path(dir.path()).exists());
    }

    #[test]
    fn atomic_write_creates_file_and_leaves_no_temp_file() {
        let dir = TempDir::new().unwrap();
        let target = preferences_file_path(dir.path());
        let payload = br#"{"menubarDisplay":"tokens"}"#;

        atomic_write_file(&target, payload).unwrap();

        assert_eq!(fs::read(&target).unwrap(), payload);
        assert!(!temp_file_path(&target).exists());
    }

    #[test]
    fn atomic_write_preserves_original_on_simulated_mid_write_failure() {
        let dir = TempDir::new().unwrap();
        let target = preferences_file_path(dir.path());
        let original = br#"{"menubarDisplay":"cost","theme":"dark"}"#;
        fs::write(&target, original).unwrap();

        let err = atomic_write_file_with(&target, br#"{"menubarDisplay":"icon"}"#, |file, data| {
            // Simulate partial write followed by an I/O failure before fsync/rename.
            file.write_all(&data[..8])?;
            Err(Error::other("simulated mid-write disk error"))
        })
        .unwrap_err();

        assert!(matches!(err, ShellError::Io(_)));
        assert_eq!(fs::read(&target).unwrap(), original);
        assert!(!temp_file_path(&target).exists());
    }

    #[test]
    fn patch_preserves_unknown_keys_and_updates_known_fields() {
        let dir = TempDir::new().unwrap();
        let target = preferences_file_path(dir.path());
        let initial = json!({
            "menubarDisplay": "cost",
            "theme": "system",
            "experimentalCustomFlag": true,
            "nestedUnknown": { "keep": 42, "tag": "alpha" }
        });
        fs::write(&target, serde_json::to_vec_pretty(&initial).unwrap()).unwrap();

        let updated = patch_preferences(
            dir.path(),
            &json!({
                "menubarDisplay": "tokens",
                "notificationsEnabled": true,
                "dailyBudgetUsd": 25.5
            }),
        )
        .unwrap();

        assert_eq!(updated.menubar_display, "tokens");
        assert!(updated.notifications_enabled);
        assert_eq!(updated.daily_budget_usd, Some(25.5));

        let raw: Value = serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(raw["experimentalCustomFlag"], json!(true));
        assert_eq!(raw["nestedUnknown"], json!({ "keep": 42, "tag": "alpha" }));
        assert_eq!(raw["menubarDisplay"], json!("tokens"));
        assert_eq!(raw["theme"], json!("system"));
    }

    #[test]
    fn patch_rejects_invalid_values_without_overwriting_disk() {
        let dir = TempDir::new().unwrap();
        let target = preferences_file_path(dir.path());
        let initial = br#"{"menubarDisplay":"cost","theme":"light"}"#;
        fs::write(&target, initial).unwrap();

        let err = patch_preferences(
            dir.path(),
            &json!({ "menubarDisplay": "invalid-mode" }),
        )
        .unwrap_err();

        assert!(matches!(err, ShellError::Preferences(_)));
        assert_eq!(fs::read(&target).unwrap(), initial);

        let non_obj_err = patch_preferences(dir.path(), &json!("not-an-object")).unwrap_err();
        assert!(matches!(non_obj_err, ShellError::Preferences(_)));
        assert_eq!(fs::read(&target).unwrap(), initial);
    }

    #[test]
    fn patch_can_set_and_clear_nullable_fields() {
        let dir = TempDir::new().unwrap();

        let set = patch_preferences(
            dir.path(),
            &json!({
                "globalShortcut": "CmdOrCtrl+Shift+T",
                "dailyBudgetUsd": 15.0
            }),
        )
        .unwrap();
        assert_eq!(set.global_shortcut.as_deref(), Some("CmdOrCtrl+Shift+T"));
        assert_eq!(set.daily_budget_usd, Some(15.0));

        let cleared = patch_preferences(
            dir.path(),
            &json!({
                "globalShortcut": null,
                "dailyBudgetUsd": null
            }),
        )
        .unwrap();
        assert_eq!(cleared.global_shortcut, None);
        assert_eq!(cleared.daily_budget_usd, None);
    }
}
