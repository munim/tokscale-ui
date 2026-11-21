pub mod cache;
pub mod harness;
pub mod schema;
pub mod sessions;
pub mod slice;
pub mod snapshot;
pub mod source;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, NaiveDate};
use serde::Deserialize;

use crate::contracts::{
    CliResolutionInfo, DashboardNavigateEvent, DashboardQuery, DashboardSnapshot, RangePreset,
    SessionDetail, SessionsPayload, SessionsQuery, SourceWarning, TraySnapshot,
};
use crate::data::cache::{CacheOutcome, UsageCacheOutcome};
use crate::data::schema::{
    ClientHealthCliRow, ClientsCliPayload, DailyRow, Graph, GraphCell, HourlyModel, HourlyRow,
    ModelRow, MonthlyCliRow, ProviderUsage, SourceBreakdown, SourceBreakdownModel, TokenCounts,
    UsageCache,
};
use crate::data::sessions::{
    build_projects, filter_sessions, join_sessions_and_models, merge_enriched_titles,
    parse_grouped_models, parse_report_sessions, session_join_key, GroupedModelCliRow,
    ReportSession,
};
use crate::data::slice::{local_today, parse_date_prefix};
use crate::data::snapshot::{
    build_dashboard, build_tray, CliResolutionInfoDefault, DataStore, LoadedSessionsScope,
};
use crate::data::source::{
    clients_cli_args, graph_cli_args, hourly_cli_args, models_cli_args, monthly_cli_args,
    range_cli_args, report_cli_args, CliRunner, DataOrigin, NoopCliRunner,
};

#[derive(Debug, Clone)]
pub struct HydrateReport {
    pub cache_outcome: CacheOutcome,
    pub usage_outcome: UsageCacheOutcome,
    pub fallback_required: bool,
    pub warnings: Vec<SourceWarning>,
}

pub struct DataService {
    store: RwLock<DataStore>,
    cli: Arc<dyn CliRunner>,
    tui_cache_path: PathBuf,
    usage_cache_path: PathBuf,
    disk_reads: AtomicU64,
}

impl DataService {
    pub fn new(cli: Arc<dyn CliRunner>) -> Self {
        Self::with_paths(cli, cache::tui_cache_path(), cache::usage_cache_path())
    }

    pub fn new_offline() -> Self {
        Self::new(Arc::new(NoopCliRunner))
    }

    pub fn with_paths(
        cli: Arc<dyn CliRunner>,
        tui_cache_path: PathBuf,
        usage_cache_path: PathBuf,
    ) -> Self {
        Self {
            store: RwLock::new(DataStore::default()),
            cli,
            tui_cache_path,
            usage_cache_path,
            disk_reads: AtomicU64::new(0),
        }
    }

    pub fn disk_reads(&self) -> u64 {
        self.disk_reads.load(Ordering::Relaxed)
    }

    pub fn fallback_required(&self) -> bool {
        self.store
            .read()
            .expect("DataStore lock poisoned")
            .fallback_required
    }

    pub fn origin(&self) -> DataOrigin {
        self.store.read().expect("DataStore lock poisoned").origin
    }

    pub fn has_today_data(&self) -> bool {
        self.has_date_data(local_today())
    }

    pub fn has_date_data(&self, date: NaiveDate) -> bool {
        let date_str = date.format("%Y-%m-%d").to_string();
        let guard = self.store.read().expect("DataStore lock poisoned");
        guard.cache.data.daily.iter().any(|d| d.date == date_str)
            || guard
                .cache
                .data
                .hourly
                .iter()
                .any(|h| h.datetime.starts_with(&date_str))
    }

    pub fn sessions_scope_covers(&self, query: &DashboardQuery) -> bool {
        let guard = self.store.read().expect("DataStore lock poisoned");
        let Some(ref loaded) = guard.loaded_sessions_scope else {
            return false;
        };
        match (loaded.range, query.range) {
            (RangePreset::All, _) => true,
            (
                RangePreset::ThirtyDays,
                RangePreset::ThirtyDays | RangePreset::SevenDays | RangePreset::Today,
            ) => true,
            (RangePreset::SevenDays, RangePreset::SevenDays | RangePreset::Today) => true,
            (RangePreset::Today, RangePreset::Today) => true,
            (RangePreset::Custom, RangePreset::Custom) => {
                loaded.custom_since == query.custom_since
                    && loaded.custom_until == query.custom_until
            }
            _ => false,
        }
    }

    pub fn update_usage_json(&self, raw: &serde_json::Value) -> Result<(), String> {
        let usage = parse_usage_cli_payload(raw)?;
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.usage = usage;
        Ok(())
    }

    pub fn hydrate_from_disk(&self) -> HydrateReport {
        self.disk_reads.fetch_add(1, Ordering::Relaxed);

        let cache_outcome = cache::read_tui_cache(&self.tui_cache_path);
        let usage_outcome = cache::read_usage_cache(&self.usage_cache_path);

        let mut warnings = Vec::new();
        if let Some(w) = cache_outcome.warning() {
            warnings.push(w);
        }
        if let Some(w) = usage_outcome.warning() {
            warnings.push(w);
        }

        let fallback_required = cache_outcome.fallback_required();

        {
            let mut guard = self.store.write().expect("DataStore lock poisoned");
            let cli_is_newer = match (&cache_outcome, guard.cli_synced_at) {
                (CacheOutcome::Fresh(tui_cache), Some(cli_ts)) => tui_cache.timestamp <= cli_ts,
                (_, Some(_)) => true,
                _ => false,
            };

            match &cache_outcome {
                CacheOutcome::Fresh(tui_cache) => {
                    if cli_is_newer {
                        if guard.cache.data.agents.is_empty() && !tui_cache.data.agents.is_empty() {
                            guard.cache.data.agents = tui_cache.data.agents.clone();
                        }
                        if guard.cache.enabled_clients.is_empty()
                            && !tui_cache.enabled_clients.is_empty()
                        {
                            guard.cache.enabled_clients = tui_cache.enabled_clients.clone();
                        }
                        guard.fallback_required = false;
                    } else {
                        guard.cache = tui_cache.clone();
                        guard.origin = DataOrigin::Cache;
                        guard.fallback_required = false;
                        if tui_cache.timestamp > 0 {
                            guard.last_synced_at = Some(tui_cache.timestamp);
                        }
                    }
                }
                CacheOutcome::Missing
                | CacheOutcome::Drift { .. }
                | CacheOutcome::Corrupt(_) => {
                    if !cli_is_newer {
                        guard.fallback_required = true;
                    }
                }
            }

            if let UsageCacheOutcome::Fresh(ref usage) = usage_outcome {
                if guard.usage.data.is_empty() || usage.timestamp > guard.usage.timestamp {
                    guard.usage = usage.clone();
                }
            }

            if cli_is_newer {
                let mut filtered_warnings: Vec<SourceWarning> = warnings
                    .iter()
                    .filter(|w| w.source != "cache")
                    .cloned()
                    .collect();
                for existing in &guard.warnings {
                    if existing.source != "cache" && !filtered_warnings.contains(existing) {
                        filtered_warnings.push(existing.clone());
                    }
                }
                guard.warnings = filtered_warnings;
            } else {
                guard.warnings = warnings.clone();
            }
        }

        HydrateReport {
            cache_outcome,
            usage_outcome,
            fallback_required,
            warnings,
        }
    }

    pub fn tray_snapshot(&self, range: RangePreset) -> TraySnapshot {
        self.tray_snapshot_at(range, local_today())
    }

    pub fn tray_snapshot_at(&self, range: RangePreset, today: NaiveDate) -> TraySnapshot {
        let guard = self.store.read().expect("DataStore lock poisoned");
        build_tray(range, &guard, today)
    }

    pub fn dashboard_snapshot(&self, query: &DashboardQuery) -> DashboardSnapshot {
        self.dashboard_snapshot_at(query, local_today())
    }

    pub fn dashboard_snapshot_at(
        &self,
        query: &DashboardQuery,
        today: NaiveDate,
    ) -> DashboardSnapshot {
        let guard = self.store.read().expect("DataStore lock poisoned");
        build_dashboard(query, &guard, today)
    }

    pub fn sessions_payload(&self, query: &SessionsQuery) -> SessionsPayload {
        self.sessions_payload_at(query, local_today())
    }

    pub fn sessions_payload_at(
        &self,
        query: &SessionsQuery,
        today: NaiveDate,
    ) -> SessionsPayload {
        let guard = self.store.read().expect("DataStore lock poisoned");
        let filtered = filter_sessions(
            &guard.joined_sessions,
            &guard.session_details,
            &query.query,
            today,
        );
        let projects = build_projects(&filtered, &guard.session_details);
        SessionsPayload {
            sessions: filtered,
            projects,
        }
    }

    pub fn session_detail(&self, client: &str, session_id: &str) -> Option<SessionDetail> {
        let guard = self.store.read().expect("DataStore lock poisoned");
        let key = session_join_key(client, session_id);
        guard.session_details.get(&key).cloned()
    }

    pub fn set_cli_info(&self, info: CliResolutionInfo) {
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.cli_info = CliResolutionInfoDefault(info);
    }

    pub fn set_syncing(&self, syncing: bool) {
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.syncing = syncing;
    }

    pub fn set_pending_handoff(&self, nav: Option<DashboardNavigateEvent>) {
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.pending_handoff = nav;
    }

    pub fn take_pending_handoff(&self) -> Option<DashboardNavigateEvent> {
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.pending_handoff.take()
    }

    pub fn set_sessions_data(
        &self,
        reports: Vec<ReportSession>,
        grouped_models: Vec<GroupedModelCliRow>,
    ) {
        let (rows, details) = join_sessions_and_models(&reports, &grouped_models);
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.report_sessions = reports;
        guard.grouped_models = grouped_models;
        guard.joined_sessions = rows;
        guard.session_details = details;
    }

    pub async fn refresh_cli_info(&self) -> CliResolutionInfo {
        let info = self.cli.cli_info().await;
        self.set_cli_info(info.clone());
        info
    }

    pub async fn refresh_clients(&self) -> Result<(), String> {
        let raw = self.cli.run_json(&clients_cli_args()).await?;
        let payload = parse_clients_cli_payload(&raw)?;
        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.clients_payload = Some(payload);
        Ok(())
    }

    pub async fn refresh_from_cli_fallback(&self) -> Result<(), String> {
        let models_val = self.cli.run_json(&models_cli_args(None, &[])).await?;
        let models = parse_models_cli_rows(&models_val)?;

        let graph_val = self.cli.run_json(&graph_cli_args()).await.ok();
        let parsed_graph = graph_val
            .as_ref()
            .and_then(|v| parse_graph_cli_payload(v).ok());

        let hourly_val = self.cli.run_json(&hourly_cli_args()).await.ok();
        let parsed_hourly = hourly_val
            .as_ref()
            .and_then(|v| parse_hourly_cli_rows(v).ok())
            .unwrap_or_default();

        let monthly_val = self.cli.run_json(&monthly_cli_args(&[])).await.ok();
        let monthly = monthly_val
            .as_ref()
            .and_then(|v| parse_monthly_cli_rows(v).ok());

        let clients_val = self.cli.run_json(&clients_cli_args()).await.ok();
        let clients = clients_val
            .as_ref()
            .and_then(|v| parse_clients_cli_payload(v).ok());

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1)
            .max(1);

        let mut guard = self.store.write().expect("DataStore lock poisoned");
        guard.cache.data.models = models;

        let mut intensity_by_date: HashMap<String, f64> = HashMap::new();
        for week in &guard.cache.data.graph.weeks {
            for cell in week.iter().flatten() {
                intensity_by_date.insert(cell.date.clone(), cell.intensity);
            }
        }

        let mut summary_streaks: Option<(u32, u32)> = None;
        if let Some((new_daily, new_intensities, streaks)) = parsed_graph {
            let mut daily_map: BTreeMap<String, DailyRow> = guard
                .cache
                .data
                .daily
                .drain(..)
                .map(|row| (row.date.clone(), row))
                .collect();
            for row in new_daily {
                daily_map.insert(row.date.clone(), row);
            }
            guard.cache.data.daily = daily_map.into_values().collect();
            for (date, intensity) in new_intensities {
                intensity_by_date.insert(date, intensity);
            }
            summary_streaks = streaks;
        }

        if !parsed_hourly.is_empty() {
            let mut hourly_map: BTreeMap<String, HourlyRow> = guard
                .cache
                .data
                .hourly
                .drain(..)
                .map(|row| (normalize_hour_key(&row.datetime), row))
                .collect();
            for row in parsed_hourly {
                hourly_map.insert(normalize_hour_key(&row.datetime), row);
            }
            guard.cache.data.hourly = hourly_map.into_values().collect();
        }

        let (rebuilt_graph, computed_cur_streak, computed_longest_streak) =
            rebuild_graph_and_streaks(&guard.cache.data.daily, &intensity_by_date, local_today());
        if !rebuilt_graph.weeks.is_empty() {
            guard.cache.data.graph = rebuilt_graph;
        }
        let (cur_streak, longest_streak) = match summary_streaks {
            Some((cur, longest)) if cur > 0 || longest > 0 => (cur, longest),
            _ => (computed_cur_streak, computed_longest_streak),
        };
        guard.cache.data.current_streak = cur_streak;
        guard.cache.data.longest_streak = longest_streak;

        if !guard.cache.data.models.is_empty() {
            guard.cache.data.total_cost = guard.cache.data.models.iter().map(|m| m.cost).sum();
            guard.cache.data.total_tokens = guard
                .cache
                .data
                .models
                .iter()
                .map(|m| m.tokens.total())
                .sum();
        } else if !guard.cache.data.daily.is_empty() {
            guard.cache.data.total_cost = guard.cache.data.daily.iter().map(|d| d.cost).sum();
            guard.cache.data.total_tokens = guard
                .cache
                .data
                .daily
                .iter()
                .map(|d| d.tokens.total())
                .sum();
        }

        if let Some(m) = monthly {
            guard.monthly_override = Some(m);
        }
        if let Some(c) = clients {
            guard.clients_payload = Some(c);
        }
        guard.origin = DataOrigin::CliFallback;
        guard.fallback_required = false;
        guard.last_synced_at = Some(now_secs);
        guard.cli_synced_at = Some(now_secs);
        guard.loaded_sessions_scope = None;
        guard.warnings.retain(|w| w.source != "cache");
        Ok(())
    }

    pub async fn refresh_sessions(
        &self,
        range: RangePreset,
        custom_since: Option<&str>,
        custom_until: Option<&str>,
    ) -> Result<SessionsPayload, String> {
        let range_args = range_cli_args(range, custom_since, custom_until, local_today());

        let report_args = report_cli_args(false, &range_args);
        let grouped_args = models_cli_args(Some("client,session,model"), &range_args);

        let (report_res, grouped_res) = tokio::join!(
            self.cli.run_json(&report_args),
            self.cli.run_json(&grouped_args)
        );

        let reports = parse_report_sessions(&report_res?)?;
        let grouped = match grouped_res {
            Ok(v) => parse_grouped_models(&v).unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        self.set_sessions_data(reports, grouped);
        {
            let mut guard = self.store.write().expect("DataStore lock poisoned");
            guard.loaded_sessions_scope = Some(LoadedSessionsScope {
                range,
                custom_since: custom_since.map(str::to_string),
                custom_until: custom_until.map(str::to_string),
            });
        }

        let query = SessionsQuery {
            query: DashboardQuery {
                range,
                custom_since: custom_since.map(str::to_string),
                custom_until: custom_until.map(str::to_string),
                group_by: "client,session,model".to_string(),
                active_clients: Vec::new(),
                search: None,
                sort: Some("date".to_string()),
            },
            enrich_titles: false,
        };
        Ok(self.sessions_payload(&query))
    }

    pub async fn enrich_session_titles(
        &self,
        range: RangePreset,
        custom_since: Option<&str>,
        custom_until: Option<&str>,
    ) -> Result<u32, String> {
        let range_args = range_cli_args(range, custom_since, custom_until, local_today());
        let args = report_cli_args(true, &range_args);
        let raw = self.cli.run_json(&args).await?;
        let enriched = parse_report_sessions(&raw)?;

        let mut guard = self.store.write().expect("DataStore lock poisoned");
        let updated_count = merge_enriched_titles(&mut guard.report_sessions, enriched);
        if updated_count > 0 {
            let (rows, details) =
                join_sessions_and_models(&guard.report_sessions, &guard.grouped_models);
            guard.joined_sessions = rows;
            guard.session_details = details;
        }
        Ok(updated_count)
    }
}

fn parse_clients_cli_payload(value: &serde_json::Value) -> Result<ClientsCliPayload, String> {
    if value.is_array() {
        let clients: Vec<ClientHealthCliRow> = serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse clients array: {e}"))?;
        Ok(ClientsCliPayload {
            headless_roots: Vec::new(),
            note: String::new(),
            clients,
        })
    } else if value.is_object() {
        #[derive(Deserialize, Default)]
        #[serde(default, rename_all = "camelCase")]
        struct Envelope {
            #[serde(alias = "headless_roots")]
            headless_roots: Vec<String>,
            note: String,
            clients: Vec<ClientHealthCliRow>,
            data: Vec<ClientHealthCliRow>,
        }
        let env: Envelope = serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse clients payload: {e}"))?;
        let clients = if env.clients.is_empty() {
            env.data
        } else {
            env.clients
        };
        Ok(ClientsCliPayload {
            headless_roots: env.headless_roots,
            note: env.note,
            clients,
        })
    } else {
        Err("Unexpected clients --json payload shape".to_string())
    }
}

fn parse_models_cli_rows(value: &serde_json::Value) -> Result<Vec<ModelRow>, String> {
    if value.is_array() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse models array: {e}"))
    } else if let Some(inner) = value
        .get("models")
        .or_else(|| value.get("entries"))
        .or_else(|| value.get("data"))
    {
        serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse models payload: {e}"))
    } else {
        Err("Unexpected models --json payload shape".to_string())
    }
}

type ParsedGraphResult = (Vec<DailyRow>, HashMap<String, f64>, Option<(u32, u32)>);

fn parse_graph_cli_payload(value: &serde_json::Value) -> Result<ParsedGraphResult, String> {
    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct GraphCliPayload {
        contributions: Vec<GraphContribution>,
        summary: Option<GraphSummary>,
    }

    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct GraphSummary {
        current_streak: Option<u32>,
        longest_streak: Option<u32>,
    }

    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct GraphContribution {
        date: String,
        totals: GraphTotals,
        intensity: f64,
        token_breakdown: TokenCounts,
        clients: Vec<GraphClientEntry>,
    }

    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct GraphTotals {
        tokens: u64,
        cost: f64,
        messages: u64,
    }

    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct GraphClientEntry {
        client: String,
        #[serde(alias = "model")]
        model_id: String,
        #[serde(alias = "provider")]
        provider_id: String,
        tokens: TokenCounts,
        cost: f64,
        #[serde(alias = "messageCount")]
        messages: u64,
    }

    let payload: GraphCliPayload = if value.is_array() {
        GraphCliPayload {
            contributions: serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to parse graph contributions array: {e}"))?,
            summary: None,
        }
    } else if value.is_object() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse graph payload: {e}"))?
    } else {
        return Err("Unexpected graph --no-spinner --offline payload shape".to_string());
    };

    let mut daily_rows = Vec::with_capacity(payload.contributions.len());
    let mut intensity_by_date = HashMap::with_capacity(payload.contributions.len());

    for contrib in payload.contributions {
        let date = contrib.date.trim().to_string();
        if date.is_empty() {
            continue;
        }
        intensity_by_date.insert(date.clone(), contrib.intensity);

        let mut by_client: BTreeMap<String, SourceBreakdown> = BTreeMap::new();
        let mut clients_tokens = TokenCounts::default();
        let mut clients_cost = 0.0_f64;
        let mut clients_messages = 0_u64;

        for cl in contrib.clients {
            let client_key = {
                let trimmed = cl.client.trim().to_ascii_lowercase();
                if trimmed.is_empty() {
                    "unknown".to_string()
                } else {
                    trimmed
                }
            };
            clients_tokens.add_assign(&cl.tokens);
            clients_cost += cl.cost;
            clients_messages = clients_messages.saturating_add(cl.messages);

            let sb = by_client.entry(client_key.clone()).or_default();
            sb.tokens.add_assign(&cl.tokens);
            sb.cost += cl.cost;

            let model_key = {
                let trimmed = cl.model_id.trim();
                if trimmed.is_empty() {
                    "unknown".to_string()
                } else {
                    trimmed.to_string()
                }
            };
            if let Some((_, existing)) = sb.models.iter_mut().find(|(k, _)| k == &model_key) {
                existing.tokens.add_assign(&cl.tokens);
                existing.cost += cl.cost;
                existing.messages = existing.messages.saturating_add(cl.messages);
            } else {
                sb.models.push((
                    model_key.clone(),
                    SourceBreakdownModel {
                        client: client_key,
                        provider: cl.provider_id,
                        display_name: model_key.clone(),
                        color_key: model_key,
                        tokens: cl.tokens,
                        cost: cl.cost,
                        messages: cl.messages,
                        performance: None,
                    },
                ));
            }
        }

        let tokens = if contrib.token_breakdown.total() > 0 {
            contrib.token_breakdown
        } else if clients_tokens.total() > 0 {
            clients_tokens
        } else if contrib.totals.tokens > 0 {
            TokenCounts {
                input: contrib.totals.tokens,
                ..TokenCounts::default()
            }
        } else {
            TokenCounts::default()
        };

        let cost = if contrib.totals.cost > 0.0 {
            contrib.totals.cost
        } else {
            clients_cost
        };

        let message_count = if contrib.totals.messages > 0 {
            contrib.totals.messages
        } else {
            clients_messages
        };

        daily_rows.push(DailyRow {
            date,
            tokens,
            cost,
            models: serde_json::Value::Array(Vec::new()),
            source_breakdown: by_client.into_iter().collect(),
            message_count,
            turn_count: None,
        });
    }

    let streaks = payload
        .summary
        .and_then(|s| match (s.current_streak, s.longest_streak) {
            (Some(c), Some(l)) => Some((c, l)),
            _ => None,
        });

    Ok((daily_rows, intensity_by_date, streaks))
}

fn parse_hourly_cli_rows(value: &serde_json::Value) -> Result<Vec<HourlyRow>, String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        One(String),
        Many(Vec<String>),
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum HourlyModelItem {
        Name(String),
        Obj {
            #[serde(default)]
            model: String,
            #[serde(default)]
            provider: String,
            #[serde(default, alias = "displayName")]
            display_name: String,
            #[serde(default)]
            tokens: TokenCounts,
            #[serde(default)]
            cost: f64,
        },
    }

    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct HourlyCliEntry {
        #[serde(alias = "datetime")]
        hour: String,
        clients: Option<StringOrVec>,
        models: Vec<HourlyModelItem>,
        input: u64,
        output: u64,
        #[serde(alias = "cache_read")]
        cache_read: u64,
        #[serde(alias = "cache_write")]
        cache_write: u64,
        reasoning: u64,
        tokens: Option<TokenCounts>,
        #[serde(alias = "message_count")]
        message_count: u64,
        #[serde(alias = "turn_count")]
        turn_count: Option<u64>,
        cost: f64,
    }

    let raw_entries: Vec<HourlyCliEntry> = if value.is_array() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse hourly array: {e}"))?
    } else if let Some(inner) = value
        .get("entries")
        .or_else(|| value.get("hourly"))
        .or_else(|| value.get("data"))
    {
        serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse hourly payload: {e}"))?
    } else {
        return Err("Unexpected hourly --json payload shape".to_string());
    };

    let mut rows = Vec::with_capacity(raw_entries.len());
    for entry in raw_entries {
        let datetime = normalize_hour_key(&entry.hour);
        if datetime.is_empty() {
            continue;
        }
        let tokens = match entry.tokens {
            Some(t) if t.total() > 0 => t,
            _ => TokenCounts {
                input: entry.input,
                output: entry.output,
                cache_read: entry.cache_read,
                cache_write: entry.cache_write,
                reasoning: entry.reasoning,
            },
        };
        let clients = match entry.clients {
            Some(StringOrVec::One(s)) => s
                .split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect(),
            Some(StringOrVec::Many(v)) => v
                .into_iter()
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect(),
            None => Vec::new(),
        };

        let model_count = entry.models.len();
        let mut models = Vec::with_capacity(model_count);
        for (idx, item) in entry.models.into_iter().enumerate() {
            match item {
                HourlyModelItem::Name(name) => {
                    let trimmed = name.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let (m_tokens, m_cost) = if model_count == 1 && idx == 0 {
                        (tokens, entry.cost)
                    } else {
                        (TokenCounts::default(), 0.0)
                    };
                    models.push((
                        trimmed.clone(),
                        HourlyModel {
                            provider: String::new(),
                            display_name: trimmed.clone(),
                            color_key: trimmed,
                            tokens: m_tokens,
                            cost: m_cost,
                        },
                    ));
                }
                HourlyModelItem::Obj {
                    model,
                    provider,
                    display_name,
                    tokens: m_tokens,
                    cost: m_cost,
                } => {
                    let key = if !model.trim().is_empty() {
                        model.trim().to_string()
                    } else {
                        display_name.trim().to_string()
                    };
                    if key.is_empty() {
                        continue;
                    }
                    let disp = if display_name.trim().is_empty() {
                        key.clone()
                    } else {
                        display_name
                    };
                    models.push((
                        key.clone(),
                        HourlyModel {
                            provider,
                            display_name: disp,
                            color_key: key,
                            tokens: m_tokens,
                            cost: m_cost,
                        },
                    ));
                }
            }
        }

        rows.push(HourlyRow {
            datetime,
            tokens,
            cost: entry.cost,
            clients,
            models,
            message_count: entry.message_count,
            turn_count: entry.turn_count,
        });
    }

    Ok(rows)
}

fn parse_usage_cli_payload(value: &serde_json::Value) -> Result<UsageCache, String> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1)
        .max(1);

    if value.is_array() {
        let data: Vec<ProviderUsage> = serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse usage array: {e}"))?;
        Ok(UsageCache {
            timestamp: now_secs,
            data,
        })
    } else if let Some(inner) = value
        .get("data")
        .or_else(|| value.get("providers"))
        .or_else(|| value.get("usage"))
    {
        let data: Vec<ProviderUsage> = serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse usage payload: {e}"))?;
        let timestamp = value
            .get("timestamp")
            .and_then(|v| v.as_u64())
            .filter(|&t| t > 0)
            .unwrap_or(now_secs);
        Ok(UsageCache { timestamp, data })
    } else {
        Err("Unexpected usage --json payload shape".to_string())
    }
}

fn normalize_hour_key(raw: &str) -> String {
    let normalized = raw.trim().replace('T', " ");
    if normalized.len() >= 13 && normalized.as_bytes().get(13) == Some(&b':') {
        return format!("{}:00", &normalized[..13]);
    }
    if normalized.len() >= 16 {
        normalized[..16].to_string()
    } else {
        normalized
    }
}

fn rebuild_graph_and_streaks(
    daily: &[DailyRow],
    intensity_by_date: &HashMap<String, f64>,
    today: NaiveDate,
) -> (Graph, u32, u32) {
    let mut daily_by_date: HashMap<&str, &DailyRow> = HashMap::with_capacity(daily.len());
    let mut max_date: Option<NaiveDate> = None;
    let mut active_dates: Vec<NaiveDate> = Vec::new();
    let mut max_cost = 0.0_f64;

    for row in daily {
        daily_by_date.insert(row.date.as_str(), row);
        max_cost = max_cost.max(row.cost);
        if let Some(date) = parse_date_prefix(&row.date) {
            max_date = Some(max_date.map_or(date, |prev| prev.max(date)));
            if row.cost > 0.0 || row.tokens.total() > 0 {
                active_dates.push(date);
            }
        }
    }

    let Some(end_date) = max_date else {
        return (Graph { weeks: Vec::new() }, 0, 0);
    };

    let end_sunday =
        end_date - chrono::Duration::days(i64::from(end_date.weekday().num_days_from_sunday()));
    let start_sunday = end_sunday - chrono::Duration::weeks(51);

    let mut weeks = Vec::with_capacity(52);
    for week_idx in 0..52_i64 {
        let mut week = Vec::with_capacity(7);
        for day_idx in 0..7_i64 {
            let cell_date = start_sunday + chrono::Duration::days(week_idx * 7 + day_idx);
            if cell_date > end_date {
                week.push(None);
                continue;
            }
            let date_str = cell_date.format("%Y-%m-%d").to_string();
            if let Some(row) = daily_by_date.get(date_str.as_str()) {
                let intensity = intensity_by_date
                    .get(&date_str)
                    .copied()
                    .unwrap_or_else(|| {
                        if row.cost <= 0.0 && row.tokens.total() == 0 {
                            0.0
                        } else if max_cost > 0.0 {
                            ((row.cost / max_cost) * 4.0).ceil().clamp(1.0, 4.0)
                        } else {
                            1.0
                        }
                    });
                week.push(Some(GraphCell {
                    date: date_str,
                    tokens: row.tokens,
                    cost: row.cost,
                    intensity,
                }));
            } else {
                week.push(Some(GraphCell {
                    date: date_str,
                    tokens: TokenCounts::default(),
                    cost: 0.0,
                    intensity: 0.0,
                }));
            }
        }
        weeks.push(week);
    }

    active_dates.sort_unstable();
    active_dates.dedup();
    let (current_streak, longest_streak) = if active_dates.is_empty() {
        (0, 0)
    } else {
        let mut longest = 1_u32;
        let mut run = 1_u32;
        for pair in active_dates.windows(2) {
            if pair[1] - pair[0] == chrono::Duration::days(1) {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 1;
            }
        }

        let mut current = 0_u32;
        if let Some(&last) = active_dates.last() {
            if last == today || last == today - chrono::Duration::days(1) {
                current = 1;
                for pair in active_dates.windows(2).rev() {
                    if pair[1] - pair[0] == chrono::Duration::days(1) {
                        current += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        (current, longest)
    };

    (Graph { weeks }, current_streak, longest_streak)
}

fn parse_monthly_cli_rows(value: &serde_json::Value) -> Result<Vec<MonthlyCliRow>, String> {
    if value.is_array() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse monthly array: {e}"))
    } else if let Some(inner) = value
        .get("monthly")
        .or_else(|| value.get("months"))
        .or_else(|| value.get("data"))
    {
        serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse monthly payload: {e}"))
    } else {
        Err("Unexpected monthly --json payload shape".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::Instant;
    use tempfile::TempDir;

    fn fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("tui-data-cache.v4.23.0.json")
    }

    fn fixed_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 21).expect("valid fixed date")
    }

    fn base_query(range: RangePreset) -> DashboardQuery {
        DashboardQuery {
            range,
            custom_since: None,
            custom_until: None,
            group_by: "model".to_string(),
            active_clients: Vec::new(),
            search: None,
            sort: Some("cost".to_string()),
        }
    }

    #[test]
    fn parses_v4_23_0_fixture_and_measures_cold_parse_speed() {
        let bytes = std::fs::read(fixture_path()).expect("read v4.23.0 fixture");
        let start = Instant::now();
        let outcome = cache::parse_tui_cache_bytes(&bytes);
        let elapsed = start.elapsed();

        let tui = match outcome {
            CacheOutcome::Fresh(c) => c,
            other => panic!("expected Fresh cache, got {other:?}"),
        };

        assert_eq!(tui.schema_version, schema::EXPECTED_SCHEMA_VERSION);
        assert_eq!(tui.schema_version, 12);
        assert_eq!(tui.parser_generation, 3431528046474800600_u64);
        assert_eq!(tui.data.models.len(), 3);
        assert_eq!(tui.data.daily.len(), 4);
        assert_eq!(tui.data.hourly.len(), 2);
        assert_eq!(tui.data.agents.len(), 2);
        assert!(
            elapsed.as_millis() < 50,
            "cold parse took unexpectedly long: {elapsed:?}"
        );
        eprintln!(
            "tui-data-cache.v4.23.0.json cold parse took {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    #[test]
    fn schema_drift_returns_drift_outcome_and_warning_without_panic() {
        let raw = std::fs::read_to_string(fixture_path()).expect("read fixture");
        let higher_version = schema::EXPECTED_SCHEMA_VERSION + 1;
        let mutated = raw.replacen(
            &format!("\"schemaVersion\": {}", schema::EXPECTED_SCHEMA_VERSION),
            &format!("\"schemaVersion\": {higher_version}"),
            1,
        );

        let tmp = TempDir::new().expect("tempdir");
        let cache_file = tmp.path().join("tui-data-cache.json");
        std::fs::write(&cache_file, mutated).expect("write drifted cache");

        let outcome = cache::read_tui_cache(&cache_file);
        match outcome {
            CacheOutcome::Drift { found, expected } => {
                assert_eq!(found, higher_version);
                assert_eq!(expected, schema::EXPECTED_SCHEMA_VERSION);
            }
            other => panic!("expected Drift, got {other:?}"),
        }
        assert!(outcome.fallback_required());
        let warning = outcome.warning().expect("drift warning");
        assert!(warning.message.contains("drift"));
    }

    #[test]
    fn missing_hourly_and_agents_keys_still_parse_via_serde_defaults() {
        let minimal = serde_json::json!({
            "schemaVersion": schema::EXPECTED_SCHEMA_VERSION,
            "parserGeneration": 3431528046474800600_u64,
            "timestamp": 1795195069_u64,
            "data": {
                "models": [
                    {
                        "model": "claude-sonnet-4-5",
                        "client": "claude",
                        "tokens": { "input": 100, "output": 50, "cacheRead": 200 },
                        "cost": 0.42
                    }
                ],
                "daily": [
                    {
                        "date": "2026-11-21",
                        "tokens": { "input": 100, "output": 50, "cacheRead": 200 },
                        "cost": 0.42
                    }
                ]
            }
        });

        let bytes = serde_json::to_vec(&minimal).expect("serialize minimal");
        match cache::parse_tui_cache_bytes(&bytes) {
            CacheOutcome::Fresh(c) => {
                assert!(c.data.hourly.is_empty());
                assert!(c.data.agents.is_empty());
                assert_eq!(c.data.models.len(), 1);
                assert_eq!(c.data.daily.len(), 1);
            }
            other => panic!("expected Fresh minimal cache, got {other:?}"),
        }
    }

    #[test]
    fn subscription_usage_cache_accepts_both_snake_case_and_camel_case() {
        let mixed = serde_json::json!({
            "timestamp": 1795195069_u64,
            "data": [
                {
                    "provider": "Copilot",
                    "plan": "Individual",
                    "metrics": [
                        {
                            "label": "Chat",
                            "remaining_label": "200/200 left",
                            "remaining_percent": 100.0,
                            "used_percent": 0.0,
                            "resets_at": "2026-12-01"
                        },
                        {
                            "label": "Completions",
                            "remainingLabel": "95% left",
                            "remainingPercent": 95.0,
                            "usedPercent": 5.0,
                            "resetsAt": "2026-12-01"
                        }
                    ]
                }
            ]
        });
        let bytes = serde_json::to_vec(&mixed).expect("serialize usage");
        match cache::parse_usage_cache_bytes(&bytes) {
            UsageCacheOutcome::Fresh(u) => {
                assert_eq!(u.data.len(), 1);
                let metrics = &u.data[0].metrics;
                assert_eq!(metrics[0].remaining_label.as_deref(), Some("200/200 left"));
                assert_eq!(metrics[0].remaining_percent, Some(100.0));
                assert_eq!(metrics[0].used_percent, Some(0.0));
                assert_eq!(metrics[0].resets_at.as_deref(), Some("2026-12-01"));

                assert_eq!(metrics[1].remaining_label.as_deref(), Some("95% left"));
                assert_eq!(metrics[1].remaining_percent, Some(95.0));
                assert_eq!(metrics[1].used_percent, Some(5.0));
                assert_eq!(metrics[1].resets_at.as_deref(), Some("2026-12-01"));
            }
            other => panic!("expected Fresh usage cache, got {other:?}"),
        }
    }

    #[test]
    fn range_boundaries_today_7d_30d_are_inclusive_with_injected_today() {
        let bytes = std::fs::read(fixture_path()).expect("read fixture");
        let CacheOutcome::Fresh(tui) = cache::parse_tui_cache_bytes(&bytes) else {
            panic!("fixture should parse");
        };
        let today = fixed_today(); // 2026-11-21
        // Fixture daily dates: 2026-10-10, 2026-11-01, 2026-11-15, 2026-11-21
        // 7d inclusive: 2026-11-15 ..= 2026-11-21 -> includes 2026-11-15 and 2026-11-21 (2 rows)
        // 30d inclusive: 2026-10-23 ..= 2026-11-21 -> includes 2026-11-01, 2026-11-15, 2026-11-21 (3 rows)

        let (today_rows, _) = slice::daily_for_range(&tui.data, &base_query(RangePreset::Today), today);
        assert_eq!(today_rows.len(), 1);
        assert_eq!(today_rows[0].date, "2026-11-21");

        let (seven_rows, _) =
            slice::daily_for_range(&tui.data, &base_query(RangePreset::SevenDays), today);
        assert_eq!(seven_rows.len(), 2);
        assert_eq!(seven_rows[0].date, "2026-11-21");
        assert_eq!(seven_rows[1].date, "2026-11-15");

        let (thirty_rows, _) =
            slice::daily_for_range(&tui.data, &base_query(RangePreset::ThirtyDays), today);
        assert_eq!(thirty_rows.len(), 3);

        let (all_rows, _) = slice::daily_for_range(&tui.data, &base_query(RangePreset::All), today);
        assert_eq!(all_rows.len(), 4);
    }

    #[test]
    fn custom_range_with_since_after_until_yields_warning_and_empty_rows() {
        let bytes = std::fs::read(fixture_path()).expect("read fixture");
        let CacheOutcome::Fresh(tui) = cache::parse_tui_cache_bytes(&bytes) else {
            panic!("fixture should parse");
        };

        let mut query = base_query(RangePreset::Custom);
        query.custom_since = Some("2026-11-25".to_string());
        query.custom_until = Some("2026-11-10".to_string());

        let (daily, warnings) = slice::daily_for_range(&tui.data, &query, fixed_today());
        assert!(daily.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("after until"));

        let (models, model_warnings) = slice::models_for_range(&tui.data, &query, fixed_today());
        assert!(models.is_empty());
        assert_eq!(model_warnings.len(), 1);
    }

    #[test]
    fn search_and_client_filters_match_case_insensitively() {
        let bytes = std::fs::read(fixture_path()).expect("read fixture");
        let CacheOutcome::Fresh(tui) = cache::parse_tui_cache_bytes(&bytes) else {
            panic!("fixture should parse");
        };

        let mut query = base_query(RangePreset::SevenDays);
        query.active_clients = vec!["CLAUDE".to_string()];
        query.search = Some("SONNET".to_string());

        let (models, warnings) = slice::models_for_range(&tui.data, &query, fixed_today());
        assert!(warnings.is_empty());
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model, "claude-sonnet-4-5");
        assert_eq!(models[0].client, "claude");
        assert!((models[0].cost - 8.45).abs() < 1e-9);
    }

    #[test]
    fn harness_fnv1a_vector_and_curated_metadata() {
        let hash = harness::fnv1a_64("futureclient");
        assert_eq!(hash, 0xccc170496ab24de5_u64);
        assert_eq!((hash % 360) as u16, 77);

        let future = harness::resolve("futureclient");
        assert_eq!(future.id, "futureclient");
        assert_eq!(future.color, "oklch(62% 0.12 77)");
        assert_eq!(future.monogram, "FU");
        assert!(!future.curated);

        // Deterministic across repeated calls
        assert_eq!(harness::resolve("futureclient"), future);

        let claude = harness::resolve("claude");
        assert_eq!(claude.label, "Claude Code");
        assert_eq!(claude.color, "oklch(68% 0.15 50)");
        assert_eq!(claude.monogram, "CC");
        assert!(claude.curated);

        let codex = harness::resolve("codex");
        assert_eq!(codex.label, "Codex");
        assert_eq!(codex.color, "oklch(60% 0.13 250)");
        assert_eq!(codex.monogram, "CO");
        assert!(codex.curated);

        let opencode = harness::resolve("opencode");
        assert_eq!(opencode.label, "OpenCode");
        assert_eq!(opencode.color, "oklch(60% 0.12 305)");
        assert_eq!(opencode.monogram, "OP");
        assert!(opencode.curated);

        let gemini = harness::resolve("gemini");
        assert_eq!(gemini.label, "Gemini CLI");
        assert_eq!(gemini.color, "oklch(78% 0.13 85)");
        assert_eq!(gemini.monogram, "GE");
        assert!(gemini.curated);
    }

    #[test]
    fn session_and_model_join_attaches_cache_write_and_reasoning_by_client_and_session_id() {
        let reports = vec![
            ReportSession {
                session_id: "sess_abc12345".to_string(),
                client: "claude".to_string(),
                workspace: Some("/Users/dev/work/toksacleui".to_string()),
                workspace_label: Some("toksacleui".to_string()),
                created_at: Some("2026-11-21T09:00:00Z".to_string()),
                last_active: Some("2026-11-21T10:15:00Z".to_string()),
                title: None,
                total_input_tokens: 1000,
                total_output_tokens: 250,
                total_cache_read: 4000,
                total_cost: 1.50,
                models_used: vec!["claude-sonnet-4-5".to_string()],
                message_count: 10,
                duration_minutes: 15,
                ..ReportSession::default()
            },
            ReportSession {
                session_id: "sess_abc12345".to_string(),
                client: "codex".to_string(),
                workspace: None,
                workspace_label: None,
                created_at: Some("2026-11-21T11:00:00Z".to_string()),
                last_active: Some("2026-11-21T11:30:00Z".to_string()),
                title: Some("Add cache reader".to_string()),
                total_input_tokens: 500,
                total_output_tokens: 100,
                total_cache_read: 800,
                total_cost: 0.75,
                models_used: vec!["gpt-5.3-codex".to_string()],
                message_count: 4,
                duration_minutes: 8,
                ..ReportSession::default()
            },
        ];

        let grouped = vec![
            GroupedModelCliRow {
                model: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                client: "claude".to_string(),
                session_id: Some("sess_abc12345".to_string()),
                tokens: schema::TokenCounts {
                    input: 1000,
                    output: 250,
                    cache_read: 4000,
                    cache_write: 320,
                    reasoning: 90,
                },
                cost: 1.50,
                message_count: 10,
                ..GroupedModelCliRow::default()
            },
            GroupedModelCliRow {
                model: "gpt-5.3-codex".to_string(),
                provider: "openai".to_string(),
                client: "codex".to_string(),
                session_id: Some("sess_abc12345".to_string()),
                tokens: schema::TokenCounts {
                    input: 500,
                    output: 100,
                    cache_read: 800,
                    cache_write: 0,
                    reasoning: 410,
                },
                cost: 0.75,
                message_count: 4,
                ..GroupedModelCliRow::default()
            },
        ];

        let (rows, details) = join_sessions_and_models(&reports, &grouped);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].short_id, "c12345");
        assert_eq!(rows[0].display_label, "toksacleui · #c12345");
        assert_eq!(rows[1].workspace_label, "(no workspace)");
        assert_eq!(rows[1].display_label, "Add cache reader");

        let claude_detail = details
            .get(&session_join_key("claude", "sess_abc12345"))
            .expect("claude detail");
        assert_eq!(claude_detail.total_tokens.cache_write, 320);
        assert_eq!(claude_detail.total_tokens.reasoning, 90);
        assert_eq!(claude_detail.models[0].tokens.cache_write, 320);

        let codex_detail = details
            .get(&session_join_key("codex", "sess_abc12345"))
            .expect("codex detail");
        assert_eq!(codex_detail.total_tokens.cache_write, 0);
        assert_eq!(codex_detail.total_tokens.reasoning, 410);

        let projects = build_projects(&rows, &details);
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].workspace_label, "toksacleui");
        assert_eq!(projects[0].session_count, 1);
        assert_eq!(projects[0].total_tokens, 1000 + 250 + 4000 + 320 + 90);
        assert_eq!(projects[1].workspace_label, "(no workspace)");
    }

    struct MockCli {
        calls: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl CliRunner for MockCli {
        async fn run_json(&self, args: &[String]) -> Result<serde_json::Value, String> {
            self.calls.lock().unwrap().push(args.to_vec());
            if args.first().map(String::as_str) == Some("report") {
                let is_enriched = !args.iter().any(|a| a == "--no-summarize");
                let title = if is_enriched {
                    Some("Enriched session title")
                } else {
                    None
                };
                Ok(serde_json::json!([
                    {
                        "session_id": "sess_100001",
                        "client": "opencode",
                        "workspace": "/Users/dev/work/toksacleui",
                        "workspace_label": "toksacleui",
                        "created_at": "2026-11-21T08:00:00Z",
                        "last_active": "2026-11-21T09:00:00Z",
                        "title": title,
                        "total_input_tokens": 200,
                        "total_output_tokens": 50,
                        "total_cache_read": 300,
                        "total_cost": 0.45,
                        "models_used": ["claude-sonnet-4-5"],
                        "message_count": 3,
                        "duration_minutes": 5
                    }
                ]))
            } else if args.first().map(String::as_str) == Some("models") {
                Ok(serde_json::json!([
                    {
                        "model": "claude-sonnet-4-5",
                        "provider": "anthropic",
                        "client": "opencode",
                        "sessionId": "sess_100001",
                        "tokens": {
                            "input": 200,
                            "output": 50,
                            "cacheRead": 300,
                            "cacheWrite": 40,
                            "reasoning": 15
                        },
                        "cost": 0.45,
                        "messageCount": 3
                    }
                ]))
            } else {
                Ok(serde_json::json!([]))
            }
        }
    }

    #[tokio::test]
    async fn data_service_hydrates_slices_with_zero_extra_disk_reads_and_enriches_titles() {
        let tmp = TempDir::new().expect("tempdir");
        let tui_path = tmp.path().join("tui-data-cache.json");
        let usage_path = tmp.path().join("subscription-usage-cache.json");
        std::fs::copy(fixture_path(), &tui_path).expect("copy fixture");

        let cli = Arc::new(MockCli {
            calls: Mutex::new(Vec::new()),
        });
        let svc = DataService::with_paths(cli.clone(), tui_path, usage_path);

        let report = svc.hydrate_from_disk();
        assert!(!report.fallback_required);
        assert_eq!(svc.disk_reads(), 1);

        // Multiple range slices use in-memory cache with 0 additional disk reads.
        let tray_7d = svc.tray_snapshot_at(RangePreset::SevenDays, fixed_today());
        assert_eq!(tray_7d.bars.len(), 7);
        assert!((tray_7d.total.cost - 25.75).abs() < 1e-6);

        let dash_all = svc.dashboard_snapshot_at(&base_query(RangePreset::All), fixed_today());
        assert_eq!(dash_all.kpis.active_days, 4);
        assert_eq!(dash_all.stats.current_streak, 2);
        assert_eq!(dash_all.stats.longest_streak, 7);
        assert_eq!(svc.disk_reads(), 1);

        let payload = svc
            .refresh_sessions(RangePreset::SevenDays, None, None)
            .await
            .expect("refresh sessions");
        assert_eq!(payload.sessions.len(), 1);
        assert_eq!(payload.sessions[0].display_label, "toksacleui · #100001");

        let updated = svc
            .enrich_session_titles(RangePreset::SevenDays, None, None)
            .await
            .expect("enrich titles");
        assert_eq!(updated, 1);

        let detail = svc
            .session_detail("opencode", "sess_100001")
            .expect("detail after enrichment");
        assert_eq!(
            detail.session.title.as_deref(),
            Some("Enriched session title")
        );
        assert_eq!(detail.total_tokens.cache_write, 40);
        assert_eq!(detail.total_tokens.reasoning, 15);

        let recorded_calls = cli.calls.lock().unwrap().clone();
        assert!(recorded_calls
            .iter()
            .all(|args| !args.iter().any(|a| a == "--summarizer")));
    }

    #[test]
    fn harness_bucketing_keeps_top_5_and_rolls_remainder_into_other() {
        let items: Vec<crate::contracts::HarnessSeriesItem> = (1..=7)
            .map(|i| {
                let id = format!("client{i}");
                let meta = harness::resolve(&id);
                crate::contracts::HarnessSeriesItem {
                    id: meta.id,
                    label: meta.label,
                    color: meta.color,
                    monogram: meta.monogram,
                    cost: f64::from(i * 10),
                    tokens: (i as u64) * 1000,
                    share_percent: 10.0,
                    sessions: Some(i as u64),
                }
            })
            .collect();

        let bucketed = harness::bucket_harnesses(items, harness::MAX_STACKED_HARNESSES);
        assert_eq!(bucketed.len(), 6);
        assert_eq!(bucketed[0].id, "client7");
        assert_eq!(bucketed[4].id, "client3");
        let other = &bucketed[5];
        assert_eq!(other.id, "other");
        assert_eq!(other.label, "Other");
        assert_eq!(other.color, "var(--c-other)");
        assert!((other.cost - 30.0).abs() < 1e-9);
        assert_eq!(other.tokens, 3000);
        assert_eq!(other.sessions, Some(3));
    }

    #[test]
    fn graph_intensity_and_hourly_today_bars_and_monthly_rollup() {
        let bytes = std::fs::read(fixture_path()).expect("read fixture");
        let CacheOutcome::Fresh(tui) = cache::parse_tui_cache_bytes(&bytes) else {
            panic!("fixture should parse");
        };

        let mut store = DataStore {
            cache: tui,
            ..DataStore::default()
        };
        store.origin = DataOrigin::Cache;

        let dash_today = build_dashboard(&base_query(RangePreset::Today), &store, fixed_today());
        assert_eq!(dash_today.chart.len(), 1);
        assert_eq!(dash_today.chart[0].slot, "09:00");
        assert!((dash_today.kpis.total_cost - 13.60).abs() < 1e-9);

        let dash_all = build_dashboard(&base_query(RangePreset::All), &store, fixed_today());
        assert_eq!(dash_all.monthly.len(), 2);
        assert_eq!(dash_all.monthly[0].month, "2026-11");
        assert_eq!(dash_all.monthly[1].month, "2026-10");

        // Verify intensity conversion from normalized 0.0..=1.0 to 0..=4 levels
        let week0_cell = dash_all.graph.weeks[0][0].as_ref().expect("2026-10-10 cell");
        assert_eq!(week0_cell.intensity, 1);
        let today_cell = dash_all.graph.weeks[2][6].as_ref().expect("2026-11-21 cell");
        assert_eq!(today_cell.intensity, 4);
        assert_eq!(today_cell.tokens.cache_read, 1_290_000);
    }

    #[tokio::test]
    async fn cli_fallback_recovers_when_cache_is_missing_or_drifted() {
        struct FallbackCli;

        #[async_trait::async_trait]
        impl CliRunner for FallbackCli {
            async fn run_json(&self, args: &[String]) -> Result<serde_json::Value, String> {
                match args.first().map(String::as_str) {
                    Some("models") => Ok(serde_json::json!([
                        {
                            "model": "claude-sonnet-4-5",
                            "provider": "anthropic",
                            "client": "claude",
                            "tokens": { "input": 100, "output": 20, "cacheRead": 300 },
                            "cost": 1.25,
                            "sessionCount": 2
                        }
                    ])),
                    Some("daily") => Err("error: unrecognized subcommand 'daily'".to_string()),
                    Some("graph") => Ok(serde_json::json!({
                        "meta": {},
                        "summary": {},
                        "years": [2026],
                        "contributions": [
                            {
                                "date": "2026-11-21",
                                "totals": { "tokens": 420, "cost": 1.25, "messages": 5 },
                                "intensity": 4,
                                "tokenBreakdown": { "input": 100, "output": 20, "cacheRead": 300, "cacheWrite": 0, "reasoning": 0 },
                                "clients": [
                                    {
                                        "client": "claude",
                                        "modelId": "claude-sonnet-4-5",
                                        "providerId": "anthropic",
                                        "tokens": { "input": 100, "output": 20, "cacheRead": 300, "cacheWrite": 0, "reasoning": 0 },
                                        "cost": 1.25,
                                        "messages": 5
                                    }
                                ]
                            }
                        ]
                    })),
                    Some("hourly") => Ok(serde_json::json!({
                        "entries": [
                            {
                                "hour": "2026-11-21 10:00",
                                "clients": ["claude"],
                                "models": ["claude-sonnet-4-5"],
                                "input": 100,
                                "output": 20,
                                "cacheRead": 300,
                                "cacheWrite": 0,
                                "messageCount": 5,
                                "turnCount": 2,
                                "cost": 1.25
                            }
                        ],
                        "totalCost": 1.25,
                        "processingTimeMs": 12
                    })),
                    Some("monthly") => Ok(serde_json::json!([
                        {
                            "month": "2026-11",
                            "models": ["claude-sonnet-4-5"],
                            "tokens": { "input": 100, "output": 20, "cacheRead": 300 },
                            "messageCount": 5,
                            "sessionCount": 2,
                            "activeDays": 1,
                            "cost": 1.25
                        }
                    ])),
                    Some("clients") => Ok(serde_json::json!([
                        {
                            "client": "claude",
                            "sessionsPath": "/Users/dev/.claude/projects",
                            "sessionsPathExists": true,
                            "messageCount": 5,
                            "status": "ready"
                        }
                    ])),
                    _ => Ok(serde_json::json!([])),
                }
            }
        }

        let tmp = TempDir::new().expect("tempdir");
        let svc = DataService::with_paths(
            Arc::new(FallbackCli),
            tmp.path().join("missing-tui.json"),
            tmp.path().join("missing-usage.json"),
        );

        let report = svc.hydrate_from_disk();
        assert!(report.fallback_required);
        assert!(svc.fallback_required());

        svc.refresh_from_cli_fallback()
            .await
            .expect("cli fallback");
        assert!(!svc.fallback_required());
        assert_eq!(svc.origin(), DataOrigin::CliFallback);

        let dash = svc.dashboard_snapshot_at(&base_query(RangePreset::All), fixed_today());
        assert!((dash.kpis.total_cost - 1.25).abs() < 1e-9);
        assert_eq!(dash.models.len(), 1);
        assert_eq!(dash.sources.clients.len(), 1);
        assert_eq!(dash.sources.clients[0].label, "Claude Code");

        let today_tray = svc.tray_snapshot_at(RangePreset::Today, fixed_today());
        assert!((today_tray.total.cost - 1.25).abs() < 1e-9);
        assert_eq!(today_tray.bars.len(), 1);
        assert_eq!(today_tray.bars[0].slot, "10:00");
        assert_eq!(today_tray.top_models.len(), 1);
    }

    #[test]
    fn update_usage_json_accepts_array_and_object_shapes() {
        let svc = DataService::new_offline();

        let array_payload = serde_json::json!([
            {
                "provider": "Copilot",
                "plan": "Individual",
                "metrics": [{ "label": "Chat", "remainingPercent": 80.0, "usedPercent": 20.0 }]
            }
        ]);
        svc.update_usage_json(&array_payload)
            .expect("array usage shape");
        let dash = svc.dashboard_snapshot_at(&base_query(RangePreset::Today), fixed_today());
        assert_eq!(dash.usage.len(), 1);
        assert_eq!(dash.usage[0].provider, "Copilot");

        let object_payload = serde_json::json!({
            "timestamp": 1795200000_u64,
            "data": [
                {
                    "provider": "Codex",
                    "plan": "Pro",
                    "metrics": [{ "label": "Weekly", "remainingPercent": 55.0, "usedPercent": 45.0 }]
                }
            ]
        });
        svc.update_usage_json(&object_payload)
            .expect("object usage shape");
        let dash2 = svc.dashboard_snapshot_at(&base_query(RangePreset::Today), fixed_today());
        assert_eq!(dash2.usage.len(), 1);
        assert_eq!(dash2.usage[0].provider, "Codex");
    }

    #[tokio::test]
    async fn sessions_scope_covers_narrower_ranges_and_refetches_wider_ranges() {
        let tmp = TempDir::new().expect("tempdir");
        let cli = Arc::new(MockCli {
            calls: Mutex::new(Vec::new()),
        });
        let svc = DataService::with_paths(
            cli,
            tmp.path().join("missing-tui.json"),
            tmp.path().join("missing-usage.json"),
        );

        assert!(!svc.sessions_scope_covers(&base_query(RangePreset::Today)));

        svc.refresh_sessions(RangePreset::SevenDays, None, None)
            .await
            .expect("refresh 7d");
        assert!(svc.sessions_scope_covers(&base_query(RangePreset::Today)));
        assert!(svc.sessions_scope_covers(&base_query(RangePreset::SevenDays)));
        assert!(!svc.sessions_scope_covers(&base_query(RangePreset::ThirtyDays)));
        assert!(!svc.sessions_scope_covers(&base_query(RangePreset::All)));

        svc.refresh_sessions(RangePreset::All, None, None)
            .await
            .expect("refresh all");
        assert!(svc.sessions_scope_covers(&base_query(RangePreset::ThirtyDays)));
        assert!(svc.sessions_scope_covers(&base_query(RangePreset::All)));
    }
}
