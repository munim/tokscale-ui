use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub mod events {
    pub const SNAPSHOT_UPDATED: &str = "tokscale://snapshot-updated";
    pub const SYNC_PROGRESS: &str = "tokscale://sync-progress";
    pub const DASHBOARD_NAVIGATE: &str = "tokscale://dashboard-navigate";
    pub const SESSION_TITLES_UPDATED: &str = "tokscale://session-titles-updated";
    pub const PREFERENCES_UPDATED: &str = "tokscale://preferences-updated";
    pub const SETTINGS_NAVIGATE: &str = "tokscale://settings-navigate";
    pub const UPDATE_STATUS: &str = "tokscale://update-status";
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RangePreset {
    Today,
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
    All,
    Custom,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DashboardTab {
    Overview,
    Usage,
    Models,
    Daily,
    Hourly,
    Monthly,
    Sessions,
    Projects,
    Stats,
    Agents,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub reasoning: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceWarning {
    pub source: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardQuery {
    pub range: RangePreset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_until: Option<String>,
    pub group_by: String,
    pub active_clients: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionsQuery {
    pub query: DashboardQuery,
    pub enrich_titles: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    LocalOffline,
    FullManual,
    #[serde(rename = "retryClient")]
    RetryClient(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub syncing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<u64>,
    pub warnings: Vec<SourceWarning>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompanionPreferences {
    pub menubar_display: String,
    pub theme: String,
    pub launch_at_login: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_shortcut: Option<String>,
    pub notifications_enabled: bool,
    pub quota_alert_percent: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_budget_usd: Option<f64>,
    pub session_title_enrichment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_tokscale_path: Option<String>,
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_update_version: Option<String>,
}

impl Default for CompanionPreferences {
    fn default() -> Self {
        Self {
            menubar_display: "cost".to_string(),
            theme: "system".to_string(),
            launch_at_login: false,
            global_shortcut: None,
            notifications_enabled: false,
            quota_alert_percent: 85,
            daily_budget_usd: None,
            session_title_enrichment: true,
            custom_tokscale_path: None,
            auto_check_updates: true,
            skipped_update_version: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliResolutionInfo {
    pub resolved_via: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEvent {
    pub timestamp: u64,
    pub partial: bool,
    pub warnings: Vec<SourceWarning>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressEvent {
    pub syncing: bool,
    pub completed_sources: u32,
    pub total_sources: u32,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardNavigateEvent {
    pub tab: DashboardTab,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<RangePreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTitlesUpdatedEvent {
    pub updated_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HarnessSeriesItem {
    pub id: String,
    pub label: String,
    pub color: String,
    pub monogram: String,
    pub cost: f64,
    pub tokens: u64,
    pub share_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartSegment {
    pub harness_id: String,
    pub label: String,
    pub color: String,
    pub cost: f64,
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartBar {
    pub slot: String,
    pub label: String,
    pub total_cost: f64,
    pub total_tokens: u64,
    pub segments: Vec<ChartSegment>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TopModelSummary {
    pub model: String,
    pub client: String,
    pub provider: String,
    pub cost: f64,
    pub tokens: u64,
    pub share_percent: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthRow {
    pub client: String,
    pub label: String,
    pub sessions_path: String,
    pub sessions_path_exists: bool,
    pub message_count: u64,
    pub headless_supported: bool,
    pub headless_message_count: u64,
    pub status: String,
    pub credentialed: bool,
    pub api_backed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrayTotal {
    pub cost: f64,
    pub tokens: u64,
    pub sessions: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraySnapshot {
    pub range: RangePreset,
    pub total: TrayTotal,
    pub bars: Vec<ChartBar>,
    pub harnesses: Vec<HarnessSeriesItem>,
    pub top_models: Vec<TopModelSummary>,
    pub sources: Vec<SourceHealthRow>,
    pub sync_status: SyncStatus,
    pub cli_info: CliResolutionInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelViewRow {
    pub model: String,
    pub color_key: String,
    pub provider: String,
    pub client: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub session_count: u64,
    pub message_count: u64,
    #[serde(
        rename = "msPer1KTokens",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ms_per_1k_tokens: Option<f64>,
    pub group_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailySourceBreakdown {
    pub client: String,
    pub label: String,
    pub tokens: TokenCounts,
    pub cost: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyViewRow {
    pub date: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub message_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_count: Option<u64>,
    pub source_breakdown: Vec<DailySourceBreakdown>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HourlyViewRow {
    pub datetime: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub clients: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_model: Option<String>,
    pub message_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_count: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyViewRow {
    pub month: String,
    pub models: Vec<String>,
    pub tokens: TokenCounts,
    pub message_count: u64,
    pub session_count: u64,
    pub active_days: u32,
    pub cost: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphCell {
    pub date: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub intensity: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
    pub weeks: Vec<Vec<Option<GraphCell>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BestDay {
    pub date: String,
    pub cost: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    pub active_days: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_day: Option<BestDay>,
    pub avg_cost_per_active_day: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetricView {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageView {
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    pub metrics: Vec<UsageMetricView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPersonaRow {
    pub agent: String,
    pub clients: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub message_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardKpis {
    pub total_cost: f64,
    pub total_tokens: u64,
    pub sessions: u64,
    pub cost_per_million: f64,
    pub active_days: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSources {
    pub headless_roots: Vec<String>,
    pub note: String,
    pub clients: Vec<SourceHealthRow>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub query: DashboardQuery,
    pub kpis: DashboardKpis,
    pub chart: Vec<ChartBar>,
    pub harnesses: Vec<HarnessSeriesItem>,
    pub models: Vec<ModelViewRow>,
    pub daily: Vec<DailyViewRow>,
    pub hourly: Vec<HourlyViewRow>,
    pub monthly: Vec<MonthlyViewRow>,
    pub graph: GraphData,
    pub stats: StatsSummary,
    pub usage: Vec<ProviderUsageView>,
    pub agents: Vec<AgentPersonaRow>,
    pub sources: DashboardSources,
    pub sync_status: SyncStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_handoff: Option<DashboardNavigateEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub session_id: String,
    pub short_id: String,
    pub client: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub display_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub workspace_label: String,
    pub models_used: Vec<String>,
    pub message_count: u64,
    pub duration_minutes: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    pub workspace_key: Option<String>,
    pub workspace_label: String,
    pub session_count: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub models: Vec<String>,
    pub last_active: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionsPayload {
    pub sessions: Vec<SessionRow>,
    pub projects: Vec<ProjectRow>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelBreakdown {
    pub model: String,
    pub provider: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub message_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: SessionRow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complexity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_group: Option<String>,
    pub total_tokens: TokenCounts,
    pub total_cost: f64,
    pub models: Vec<SessionModelBreakdown>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SettingsSection {
    #[default]
    App,
    Sources,
    Providers,
    Pricing,
    Social,
    Cli,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsNavigateEvent {
    pub section: SettingsSection,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokscaleSettingsState {
    pub path: String,
    pub exists: bool,
    pub config_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_preferences_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_timezone: Option<String>,
    pub default_clients: Vec<String>,
    pub extra_scan_paths: BTreeMap<String, Vec<String>>,
    pub opencode_db_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_machine_label: Option<String>,
    pub include_unused_models: bool,
    pub model_aliases: BTreeMap<String, String>,
    pub native_timeout_ms: u64,
    pub light_write_cache: bool,
    pub auto_refresh_enabled: bool,
    pub auto_refresh_ms: u64,
    pub minutely_tab_enabled: bool,
    pub color_palette: String,
    pub tui_language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokscaleSettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_clients: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_scan_paths: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_db_paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_machine_label: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_unused_models: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_aliases: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_write_cache: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_refresh_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_refresh_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutely_tab_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_palette: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tui_language: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomPricingEntry {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomPricingState {
    pub path: String,
    pub exists: bool,
    pub entries: Vec<CustomPricingEntry>,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelPriceLookupResult {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_per_million: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission_safe: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAccountInfo {
    pub name: String,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusItem {
    pub id: String,
    pub label: String,
    pub connected: bool,
    pub supports_multi_account: bool,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    pub accounts: Vec<ProviderAccountInfo>,
    pub diagnostics: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProvidersStatusPayload {
    pub providers: Vec<ProviderStatusItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderActionRequest {
    pub provider: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub is_cookie: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default)]
    pub purge_cache: bool,
    #[serde(default)]
    pub all_accounts: bool,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliActionResult {
    pub ok: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutosubmitStatus {
    pub enabled: bool,
    pub interval_minutes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    pub clients: Vec<String>,
    pub today: bool,
    pub yesterday: bool,
    pub week: bool,
    pub month: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocialStatusPayload {
    pub logged_in: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub autosubmit: AutosubmitStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocialActionRequest {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default)]
    pub clients: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutosubmitConfigRequest {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(default)]
    pub clients: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePhase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Ready,
    Error,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum UpdateCapability {
    #[default]
    SelfUpdate,
    NotifyOnly,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusPayload {
    pub current_version: String,
    pub phase: UpdatePhase,
    pub capability: UpdateCapability,
    pub install_method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pub_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloaded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_query_round_trip_and_camel_case_keys() {
        let query = DashboardQuery {
            range: RangePreset::SevenDays,
            custom_since: Some("2026-11-01".to_string()),
            custom_until: Some("2026-11-21".to_string()),
            group_by: "client,model".to_string(),
            active_clients: vec!["claude".to_string(), "codex".to_string()],
            search: Some("sonnet".to_string()),
            sort: Some("cost".to_string()),
        };

        let json = serde_json::to_string(&query).expect("serialize DashboardQuery");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        assert_eq!(value["range"], "7d");
        assert_eq!(value["customSince"], "2026-11-01");
        assert_eq!(value["customUntil"], "2026-11-21");
        assert_eq!(value["groupBy"], "client,model");
        assert_eq!(value["activeClients"][0], "claude");
        assert_eq!(value["search"], "sonnet");
        assert_eq!(value["sort"], "cost");

        let decoded: DashboardQuery = serde_json::from_str(&json).expect("deserialize DashboardQuery");
        assert_eq!(decoded, query);
    }

    #[test]
    fn companion_preferences_round_trip_and_defaults() {
        let prefs = CompanionPreferences::default();
        let json = serde_json::to_string(&prefs).expect("serialize CompanionPreferences");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");

        assert_eq!(value["menubarDisplay"], "cost");
        assert_eq!(value["theme"], "system");
        assert_eq!(value["launchAtLogin"], false);
        assert_eq!(value["notificationsEnabled"], false);
        assert_eq!(value["quotaAlertPercent"], 85);
        assert_eq!(value["sessionTitleEnrichment"], true);
        assert!(value.get("globalShortcut").is_none());

        let decoded: CompanionPreferences =
            serde_json::from_str(&json).expect("deserialize CompanionPreferences");
        assert_eq!(decoded, prefs);
    }

    #[test]
    fn enums_and_camel_case_field_parity() {
        assert_eq!(serde_json::to_string(&RangePreset::Today).unwrap(), "\"today\"");
        assert_eq!(serde_json::to_string(&RangePreset::SevenDays).unwrap(), "\"7d\"");
        assert_eq!(serde_json::to_string(&RangePreset::ThirtyDays).unwrap(), "\"30d\"");
        assert_eq!(serde_json::to_string(&RangePreset::All).unwrap(), "\"all\"");
        assert_eq!(serde_json::to_string(&RangePreset::Custom).unwrap(), "\"custom\"");

        assert_eq!(serde_json::to_string(&SyncMode::LocalOffline).unwrap(), "\"local_offline\"");
        assert_eq!(serde_json::to_string(&SyncMode::FullManual).unwrap(), "\"full_manual\"");
        assert_eq!(
            serde_json::to_string(&SyncMode::RetryClient("cursor".to_string())).unwrap(),
            "{\"retryClient\":\"cursor\"}"
        );

        let tokens = TokenCounts {
            input: 100,
            output: 50,
            cache_read: 400,
            cache_write: 25,
            reasoning: 10,
        };
        let tokens_val = serde_json::to_value(tokens).unwrap();
        assert_eq!(tokens_val["cacheRead"], 400);
        assert_eq!(tokens_val["cacheWrite"], 25);

        let model_row = ModelViewRow {
            model: "claude-sonnet-4-5".to_string(),
            color_key: "claude".to_string(),
            provider: "anthropic".to_string(),
            client: "claude".to_string(),
            workspace_key: None,
            workspace_label: Some("toksacleui".to_string()),
            session_id: None,
            tokens,
            cost: 1.25,
            session_count: 3,
            message_count: 14,
            ms_per_1k_tokens: Some(210.5),
            group_key: "claude:claude-sonnet-4-5".to_string(),
        };
        let model_val = serde_json::to_value(&model_row).unwrap();
        assert_eq!(model_val["msPer1KTokens"], 210.5);
        assert_eq!(model_val["colorKey"], "claude");
        assert_eq!(model_val["sessionCount"], 3);
        assert_eq!(model_val["messageCount"], 14);
        assert_eq!(model_val["groupKey"], "claude:claude-sonnet-4-5");
        let round_trip: ModelViewRow = serde_json::from_value(model_val).unwrap();
        assert_eq!(round_trip, model_row);
    }

    #[test]
    fn snapshot_payloads_round_trip() {
        let project = ProjectRow {
            workspace_key: None,
            workspace_label: "(no workspace)".to_string(),
            session_count: 2,
            total_tokens: 1500,
            total_cost: 0.42,
            models: vec!["claude-sonnet-4-5".to_string()],
            last_active: None,
        };
        let proj_val = serde_json::to_value(&project).unwrap();
        assert!(proj_val["workspaceKey"].is_null());
        assert!(proj_val["lastActive"].is_null());

        let tray = TraySnapshot {
            range: RangePreset::Today,
            total: TrayTotal {
                cost: 48.12,
                tokens: 61_400_000,
                sessions: 12,
            },
            bars: vec![],
            harnesses: vec![],
            top_models: vec![],
            sources: vec![],
            sync_status: SyncStatus::default(),
            cli_info: CliResolutionInfo {
                resolved_via: "probe".to_string(),
                binary_path: Some("/Users/dev/.bun/bin/tokscale".to_string()),
                version: Some("4.23.0".to_string()),
                fallback: None,
            },
        };
        let tray_json = serde_json::to_string(&tray).unwrap();
        let tray_decoded: TraySnapshot = serde_json::from_str(&tray_json).unwrap();
        assert_eq!(tray_decoded, tray);
    }
}
