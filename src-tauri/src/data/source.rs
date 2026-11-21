use chrono::NaiveDate;

use crate::contracts::{CliResolutionInfo, RangePreset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataOrigin {
    #[default]
    Cache,
    CliFallback,
    Hybrid,
}

#[async_trait::async_trait]
pub trait CliRunner: Send + Sync {
    /// Execute a `tokscale` subcommand with the provided arguments (e.g. `["report", "--json"]`)
    /// and return its parsed JSON stdout.
    async fn run_json(&self, args: &[String]) -> Result<serde_json::Value, String>;

    /// Return current CLI binary resolution metadata.
    async fn cli_info(&self) -> CliResolutionInfo {
        CliResolutionInfo {
            resolved_via: "unavailable".to_string(),
            binary_path: None,
            version: None,
            fallback: None,
        }
    }
}

/// Default offline runner used in tests or before a live CLI runner is wired.
#[derive(Debug, Default)]
pub struct NoopCliRunner;

#[async_trait::async_trait]
impl CliRunner for NoopCliRunner {
    async fn run_json(&self, args: &[String]) -> Result<serde_json::Value, String> {
        Err(format!(
            "CLI runner unavailable for command: {}",
            args.join(" ")
        ))
    }
}

pub fn range_cli_args(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
    today: NaiveDate,
) -> Vec<String> {
    match range {
        RangePreset::Today => {
            let d = today.format("%Y-%m-%d").to_string();
            vec!["--since".to_string(), d.clone(), "--until".to_string(), d]
        }
        RangePreset::SevenDays => {
            let since = (today - chrono::Duration::days(6))
                .format("%Y-%m-%d")
                .to_string();
            let until = today.format("%Y-%m-%d").to_string();
            vec!["--since".to_string(), since, "--until".to_string(), until]
        }
        RangePreset::ThirtyDays => {
            let since = (today - chrono::Duration::days(29))
                .format("%Y-%m-%d")
                .to_string();
            let until = today.format("%Y-%m-%d").to_string();
            vec!["--since".to_string(), since, "--until".to_string(), until]
        }
        RangePreset::All => Vec::new(),
        RangePreset::Custom => {
            let mut out = Vec::new();
            if let Some(since) = custom_since.map(str::trim).filter(|s| !s.is_empty()) {
                out.push("--since".to_string());
                out.push(since.to_string());
            }
            if let Some(until) = custom_until.map(str::trim).filter(|s| !s.is_empty()) {
                out.push("--until".to_string());
                out.push(until.to_string());
            }
            out
        }
    }
}

pub fn models_cli_args(group_by: Option<&str>, range_args: &[String]) -> Vec<String> {
    let mut args = vec!["models".to_string(), "--json".to_string()];
    if let Some(group) = group_by.map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--group-by".to_string());
        args.push(group.to_string());
    }
    args.extend(range_args.iter().cloned());
    args
}

pub fn monthly_cli_args(range_args: &[String]) -> Vec<String> {
    let mut args = vec!["monthly".to_string(), "--json".to_string()];
    args.extend(range_args.iter().cloned());
    args
}

pub fn clients_cli_args() -> Vec<String> {
    vec!["clients".to_string(), "--json".to_string()]
}

/// Build `report` CLI arguments. Never emits `--summarizer`.
pub fn report_cli_args(summarize: bool, range_args: &[String]) -> Vec<String> {
    let mut args = vec!["report".to_string()];
    if !summarize {
        args.push("--no-summarize".to_string());
    }
    args.push("--json".to_string());
    args.extend(range_args.iter().cloned());
    args
}
