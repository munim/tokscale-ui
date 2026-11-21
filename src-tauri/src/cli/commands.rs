use std::time::Duration;

use crate::contracts::RangePreset;

/// Timeout for `report`, `models`, `monthly`, and `clients` commands.
pub const TIMEOUT_DATA: Duration = Duration::from_secs(120);

/// Timeout for `usage` and `--version` commands.
pub const TIMEOUT_SHORT: Duration = Duration::from_secs(30);

/// Timeout for provider `sync` and global install/upgrade commands.
pub const TIMEOUT_LONG: Duration = Duration::from_secs(600);

/// Build CLI range flags for a `RangePreset` and optional custom YYYY-MM-DD bounds.
pub fn range_flags(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    match range {
        RangePreset::Today => vec!["--today".to_string()],
        RangePreset::SevenDays => vec!["--week".to_string()],
        RangePreset::ThirtyDays => vec!["--month".to_string()],
        RangePreset::All => Vec::new(),
        RangePreset::Custom => {
            let mut flags = Vec::new();
            if let Some(since) = custom_since.map(str::trim).filter(|s| !s.is_empty()) {
                flags.push("--since".to_string());
                flags.push(since.to_string());
            }
            if let Some(until) = custom_until.map(str::trim).filter(|s| !s.is_empty()) {
                flags.push("--until".to_string());
                flags.push(until.to_string());
            }
            flags
        }
    }
}

/// Offline model grouping: `models --json --group-by <groupBy> --no-spinner --offline [range]`.
pub fn models_offline(
    group_by: &str,
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        "models".to_string(),
        "--json".to_string(),
        "--group-by".to_string(),
        group_by.to_string(),
        "--no-spinner".to_string(),
        "--offline".to_string(),
    ];
    argv.extend(range_flags(range, custom_since, custom_until));
    argv
}

/// Sessions (fast, without title summarization): `report --no-summarize --json [range]`.
///
/// Note: `report` does not accept `--no-spinner`.
pub fn report_fast(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        "report".to_string(),
        "--no-summarize".to_string(),
        "--json".to_string(),
    ];
    argv.extend(range_flags(range, custom_since, custom_until));
    debug_assert!(!argv.iter().any(|a| a == "--no-spinner"));
    debug_assert!(!argv
        .iter()
        .any(|a| a == "--summarizer" || a.starts_with("--summarizer=")));
    argv
}

/// Sessions with free local title enrichment: `report --json [range]`.
///
/// Uses the default free local summarizer only; never passes `--summarizer` or `--no-spinner`.
pub fn report_with_titles(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    let mut argv = vec!["report".to_string(), "--json".to_string()];
    argv.extend(range_flags(range, custom_since, custom_until));
    debug_assert!(!argv.iter().any(|a| a == "--no-spinner"));
    debug_assert!(!argv
        .iter()
        .any(|a| a == "--summarizer" || a.starts_with("--summarizer=")));
    argv
}

/// Session-to-model join: `models --json --group-by client,session,model --no-spinner --offline [range]`.
pub fn session_model_join(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    models_offline("client,session,model", range, custom_since, custom_until)
}

/// Monthly rollup: `monthly --json --no-spinner --offline [range]`.
pub fn monthly_offline(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        "monthly".to_string(),
        "--json".to_string(),
        "--no-spinner".to_string(),
        "--offline".to_string(),
    ];
    argv.extend(range_flags(range, custom_since, custom_until));
    argv
}

/// Clients list: `clients --json`.
pub fn clients() -> Vec<String> {
    vec!["clients".to_string(), "--json".to_string()]
}

/// Subscription quotas: `usage --json`.
pub fn usage() -> Vec<String> {
    vec!["usage".to_string(), "--json".to_string()]
}

/// Provider sync: `<provider> sync`.
pub fn provider_sync(provider: &str) -> Vec<String> {
    vec![provider.to_string(), "sync".to_string()]
}

/// CLI version check: `--version`.
pub fn version() -> Vec<String> {
    vec!["--version".to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_presets() -> [RangePreset; 5] {
        [
            RangePreset::Today,
            RangePreset::SevenDays,
            RangePreset::ThirtyDays,
            RangePreset::All,
            RangePreset::Custom,
        ]
    }

    #[test]
    fn report_argv_never_contains_no_spinner_or_summarizer() {
        for preset in all_presets() {
            let fast = report_fast(preset, Some("2026-11-01"), Some("2026-11-21"));
            assert!(!fast.iter().any(|arg| arg == "--no-spinner"));
            assert!(!fast
                .iter()
                .any(|arg| arg == "--summarizer" || arg.starts_with("--summarizer=")));
            assert_eq!(fast[0], "report");
            assert!(fast.contains(&"--no-summarize".to_string()));
            assert!(fast.contains(&"--json".to_string()));

            let enriched = report_with_titles(preset, Some("2026-11-01"), Some("2026-11-21"));
            assert!(!enriched.iter().any(|arg| arg == "--no-spinner"));
            assert!(!enriched
                .iter()
                .any(|arg| arg == "--summarizer" || arg.starts_with("--summarizer=")));
            assert!(!enriched.contains(&"--no-summarize".to_string()));
            assert_eq!(enriched[0], "report");
            assert!(enriched.contains(&"--json".to_string()));
        }
    }

    #[test]
    fn exact_command_shapes_and_range_flags() {
        assert_eq!(
            models_offline("client,model", RangePreset::Today, None, None),
            vec![
                "models",
                "--json",
                "--group-by",
                "client,model",
                "--no-spinner",
                "--offline",
                "--today",
            ]
        );
        assert_eq!(
            session_model_join(RangePreset::SevenDays, None, None),
            vec![
                "models",
                "--json",
                "--group-by",
                "client,session,model",
                "--no-spinner",
                "--offline",
                "--week",
            ]
        );
        assert_eq!(
            monthly_offline(RangePreset::ThirtyDays, None, None),
            vec![
                "monthly",
                "--json",
                "--no-spinner",
                "--offline",
                "--month",
            ]
        );
        assert_eq!(
            monthly_offline(RangePreset::All, None, None),
            vec!["monthly", "--json", "--no-spinner", "--offline"]
        );
        assert_eq!(
            report_fast(RangePreset::Custom, Some("2026-11-01"), Some("2026-11-21")),
            vec![
                "report",
                "--no-summarize",
                "--json",
                "--since",
                "2026-11-01",
                "--until",
                "2026-11-21",
            ]
        );
        assert_eq!(clients(), vec!["clients", "--json"]);
        assert_eq!(usage(), vec!["usage", "--json"]);
        assert_eq!(provider_sync("cursor"), vec!["cursor", "sync"]);
        assert_eq!(version(), vec!["--version"]);
    }
}
