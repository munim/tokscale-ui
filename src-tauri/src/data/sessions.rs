use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::contracts::{
    DashboardQuery, ProjectRow, SessionDetail, SessionModelBreakdown, SessionRow,
    TokenCounts as ContractTokenCounts,
};
use crate::data::schema::TokenCounts;
use crate::data::slice::{
    client_matches, contains_ci, contract_tokens_total, normalize_search, resolve_bounds,
};

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct ReportSession {
    #[serde(alias = "sessionId")]
    pub session_id: String,
    pub client: String,
    pub workspace: Option<String>,
    #[serde(alias = "workspaceLabel")]
    pub workspace_label: Option<String>,
    #[serde(alias = "createdAt")]
    pub created_at: Option<String>,
    #[serde(alias = "lastActive")]
    pub last_active: Option<String>,
    pub title: Option<String>,
    #[serde(alias = "taskCategory")]
    pub task_category: Option<String>,
    pub description: Option<String>,
    pub complexity: Option<String>,
    #[serde(alias = "taskGroup")]
    pub task_group: Option<String>,
    #[serde(alias = "totalInputTokens")]
    pub total_input_tokens: u64,
    #[serde(alias = "totalOutputTokens")]
    pub total_output_tokens: u64,
    #[serde(alias = "totalCacheRead")]
    pub total_cache_read: u64,
    #[serde(alias = "totalCost")]
    pub total_cost: f64,
    #[serde(alias = "modelsUsed")]
    pub models_used: Vec<String>,
    #[serde(alias = "messageCount")]
    pub message_count: u64,
    #[serde(alias = "durationMinutes")]
    pub duration_minutes: u64,
    #[serde(alias = "summarizedAt")]
    pub summarized_at: Option<String>,
    #[serde(alias = "fmVersion")]
    pub fm_version: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GroupedModelCliRow {
    pub model: String,
    pub provider: String,
    pub client: String,
    #[serde(alias = "session_id")]
    pub session_id: Option<String>,
    #[serde(alias = "workspace_key")]
    pub workspace_key: Option<String>,
    #[serde(alias = "workspace_label")]
    pub workspace_label: Option<String>,
    pub tokens: TokenCounts,
    pub cost: f64,
    #[serde(alias = "message_count", alias = "messages")]
    pub message_count: u64,
    #[serde(alias = "session_count")]
    pub session_count: u64,
}

pub fn session_join_key(client: &str, session_id: &str) -> (String, String) {
    (
        client.trim().to_ascii_lowercase(),
        session_id.trim().to_string(),
    )
}

pub fn short_session_id(session_id: &str) -> String {
    let trimmed = session_id.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 6 {
        trimmed.to_string()
    } else {
        chars[chars.len() - 6..].iter().collect()
    }
}

pub fn resolve_workspace_label(
    workspace_label: Option<&str>,
    workspace: Option<&str>,
) -> String {
    if let Some(label) = workspace_label.map(str::trim).filter(|s| !s.is_empty()) {
        return label.to_string();
    }
    if let Some(ws) = workspace.map(str::trim).filter(|s| !s.is_empty()) {
        return ws.to_string();
    }
    "(no workspace)".to_string()
}

pub fn parse_report_sessions(value: &serde_json::Value) -> Result<Vec<ReportSession>, String> {
    if value.is_array() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse report sessions array: {e}"))
    } else if let Some(inner) = value.get("sessions").or_else(|| value.get("data")) {
        serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse report sessions payload: {e}"))
    } else {
        Err("Unexpected report --json payload shape".to_string())
    }
}

pub fn parse_grouped_models(value: &serde_json::Value) -> Result<Vec<GroupedModelCliRow>, String> {
    if value.is_array() {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse grouped models array: {e}"))
    } else if let Some(inner) = value
        .get("models")
        .or_else(|| value.get("entries"))
        .or_else(|| value.get("data"))
    {
        serde_json::from_value(inner.clone())
            .map_err(|e| format!("Failed to parse grouped models payload: {e}"))
    } else {
        Err("Unexpected models --json payload shape".to_string())
    }
}

pub fn build_session_row(report: &ReportSession) -> SessionRow {
    let short_id = short_session_id(&report.session_id);
    let workspace_label = resolve_workspace_label(
        report.workspace_label.as_deref(),
        report.workspace.as_deref(),
    );
    let fallback_label = format!("{workspace_label} · #{short_id}");
    let title = report
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let display_label = title.clone().unwrap_or(fallback_label);

    SessionRow {
        session_id: report.session_id.clone(),
        short_id,
        client: report.client.clone(),
        title,
        display_label,
        workspace: report.workspace.clone(),
        workspace_label,
        models_used: report.models_used.clone(),
        message_count: report.message_count,
        duration_minutes: report.duration_minutes,
        input_tokens: report.total_input_tokens,
        output_tokens: report.total_output_tokens,
        cache_read_tokens: report.total_cache_read,
        total_cost: report.total_cost,
        created_at: report.created_at.clone(),
        last_active: report.last_active.clone(),
    }
}

pub fn join_sessions_and_models(
    reports: &[ReportSession],
    grouped_models: &[GroupedModelCliRow],
) -> (Vec<SessionRow>, HashMap<(String, String), SessionDetail>) {
    let mut models_by_key: HashMap<(String, String), Vec<SessionModelBreakdown>> = HashMap::new();

    for gm in grouped_models {
        let Some(sid) = gm.session_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let key = session_join_key(&gm.client, sid);
        models_by_key
            .entry(key)
            .or_default()
            .push(SessionModelBreakdown {
                model: gm.model.clone(),
                provider: gm.provider.clone(),
                tokens: gm.tokens.into(),
                cost: gm.cost,
                message_count: gm.message_count,
            });
    }

    let mut rows = Vec::with_capacity(reports.len());
    let mut details = HashMap::with_capacity(reports.len());

    for report in reports {
        let key = session_join_key(&report.client, &report.session_id);
        let mut row = build_session_row(report);

        let mut model_rows = models_by_key.remove(&key).unwrap_or_default();
        model_rows.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| a.model.cmp(&b.model))
        });

        let total_tokens = if model_rows.is_empty() {
            ContractTokenCounts {
                input: report.total_input_tokens,
                output: report.total_output_tokens,
                cache_read: report.total_cache_read,
                cache_write: 0,
                reasoning: 0,
            }
        } else {
            let mut sum = TokenCounts::default();
            for m in &model_rows {
                let mt: TokenCounts = m.tokens.into();
                sum.add_assign(&mt);
            }
            if row.models_used.is_empty() {
                let mut seen = BTreeSet::new();
                for m in &model_rows {
                    if !m.model.trim().is_empty() {
                        seen.insert(m.model.clone());
                    }
                }
                row.models_used = seen.into_iter().collect();
            }
            sum.into()
        };

        let detail = SessionDetail {
            session: row.clone(),
            task_category: report.task_category.clone(),
            description: report.description.clone(),
            complexity: report.complexity.clone(),
            task_group: report.task_group.clone(),
            total_tokens,
            total_cost: report.total_cost,
            models: model_rows,
        };

        rows.push(row);
        details.insert(key, detail);
    }

    (rows, details)
}

pub fn merge_enriched_titles(
    existing: &mut [ReportSession],
    enriched: Vec<ReportSession>,
) -> u32 {
    let mut by_key: HashMap<(String, String), ReportSession> =
        HashMap::with_capacity(enriched.len());
    for item in enriched {
        by_key.insert(session_join_key(&item.client, &item.session_id), item);
    }

    let mut updated_count = 0_u32;
    for slot in existing.iter_mut() {
        let key = session_join_key(&slot.client, &slot.session_id);
        let Some(fresh) = by_key.get(&key) else {
            continue;
        };

        let new_title = fresh
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        if let Some(ref title) = new_title {
            if slot.title.as_deref() != Some(title.as_str()) {
                slot.title = Some(title.clone());
                updated_count = updated_count.saturating_add(1);
            }
        }
        if fresh.task_category.is_some() {
            slot.task_category = fresh.task_category.clone();
        }
        if fresh.description.is_some() {
            slot.description = fresh.description.clone();
        }
        if fresh.complexity.is_some() {
            slot.complexity = fresh.complexity.clone();
        }
        if fresh.task_group.is_some() {
            slot.task_group = fresh.task_group.clone();
        }
        if fresh.summarized_at.is_some() {
            slot.summarized_at = fresh.summarized_at.clone();
        }
        if fresh.fm_version.is_some() {
            slot.fm_version = fresh.fm_version.clone();
        }
    }

    updated_count
}

pub fn filter_sessions(
    sessions: &[SessionRow],
    details: &HashMap<(String, String), SessionDetail>,
    query: &DashboardQuery,
    today: NaiveDate,
) -> Vec<SessionRow> {
    let Ok(bounds) = resolve_bounds(
        query.range,
        query.custom_since.as_deref(),
        query.custom_until.as_deref(),
        today,
    ) else {
        return Vec::new();
    };

    let needle = normalize_search(query.search.as_deref());
    let mut out = Vec::new();

    for s in sessions {
        if bounds.since.is_some() || bounds.until.is_some() {
            let date_str = s
                .last_active
                .as_deref()
                .or(s.created_at.as_deref())
                .unwrap_or("");
            if !bounds.contains_str(date_str) {
                continue;
            }
        }

        if !client_matches(&s.client, &query.active_clients) {
            continue;
        }

        if let Some(ref q) = needle {
            let key = session_join_key(&s.client, &s.session_id);
            let detail = details.get(&key);
            let matches = contains_ci(&s.session_id, q)
                || contains_ci(&s.short_id, q)
                || contains_ci(&s.client, q)
                || contains_ci(&s.workspace_label, q)
                || contains_ci(&s.display_label, q)
                || s.title.as_deref().is_some_and(|t| contains_ci(t, q))
                || s.models_used.iter().any(|m| contains_ci(m, q))
                || detail
                    .and_then(|d| d.description.as_deref())
                    .is_some_and(|desc| contains_ci(desc, q))
                || detail
                    .and_then(|d| d.task_category.as_deref())
                    .is_some_and(|cat| contains_ci(cat, q));
            if !matches {
                continue;
            }
        }

        out.push(s.clone());
    }

    match query.sort.as_deref().map(str::trim).unwrap_or("date") {
        "cost" => out.sort_by(|a, b| {
            b.total_cost
                .total_cmp(&a.total_cost)
                .then_with(|| b.last_active.cmp(&a.last_active))
                .then_with(|| a.session_id.cmp(&b.session_id))
        }),
        "tokens" => out.sort_by(|a, b| {
            session_total_tokens(b, details)
                .cmp(&session_total_tokens(a, details))
                .then_with(|| b.total_cost.total_cmp(&a.total_cost))
                .then_with(|| a.session_id.cmp(&b.session_id))
        }),
        _ => out.sort_by(|a, b| {
            b.last_active
                .cmp(&a.last_active)
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| b.total_cost.total_cmp(&a.total_cost))
        }),
    }

    out
}

fn session_total_tokens(
    s: &SessionRow,
    details: &HashMap<(String, String), SessionDetail>,
) -> u64 {
    let key = session_join_key(&s.client, &s.session_id);
    if let Some(detail) = details.get(&key) {
        contract_tokens_total(&detail.total_tokens)
    } else {
        s.input_tokens
            .saturating_add(s.output_tokens)
            .saturating_add(s.cache_read_tokens)
    }
}

pub fn build_projects(
    sessions: &[SessionRow],
    details: &HashMap<(String, String), SessionDetail>,
) -> Vec<ProjectRow> {
    struct ProjectAgg {
        workspace_key: Option<String>,
        workspace_label: String,
        session_count: u64,
        total_tokens: u64,
        total_cost: f64,
        models: BTreeSet<String>,
        last_active: Option<String>,
    }

    let mut groups: HashMap<String, ProjectAgg> = HashMap::new();

    for s in sessions {
        let label = resolve_workspace_label(
            Some(s.workspace_label.as_str()),
            s.workspace.as_deref(),
        );

        let slot = groups.entry(label.clone()).or_insert_with(|| ProjectAgg {
            workspace_key: s
                .workspace
                .as_deref()
                .map(str::trim)
                .filter(|w| !w.is_empty())
                .map(str::to_string),
            workspace_label: label,
            session_count: 0,
            total_tokens: 0,
            total_cost: 0.0,
            models: BTreeSet::new(),
            last_active: None,
        });

        if slot.workspace_key.is_none() {
            slot.workspace_key = s
                .workspace
                .as_deref()
                .map(str::trim)
                .filter(|w| !w.is_empty())
                .map(str::to_string);
        }

        slot.session_count = slot.session_count.saturating_add(1);
        slot.total_tokens = slot
            .total_tokens
            .saturating_add(session_total_tokens(s, details));
        slot.total_cost += s.total_cost;

        for m in &s.models_used {
            let trimmed = m.trim();
            if !trimmed.is_empty() {
                slot.models.insert(trimmed.to_string());
            }
        }

        if let Some(ref ts) = s.last_active {
            let should_replace = slot
                .last_active
                .as_deref()
                .is_none_or(|prev| ts.as_str() > prev);
            if should_replace {
                slot.last_active = Some(ts.clone());
            }
        }
    }

    let mut out: Vec<ProjectRow> = groups
        .into_values()
        .map(|g| ProjectRow {
            workspace_key: g.workspace_key,
            workspace_label: g.workspace_label,
            session_count: g.session_count,
            total_tokens: g.total_tokens,
            total_cost: g.total_cost,
            models: g.models.into_iter().collect(),
            last_active: g.last_active,
        })
        .collect();

    out.sort_by(|a, b| {
        b.total_cost
            .total_cmp(&a.total_cost)
            .then_with(|| b.total_tokens.cmp(&a.total_tokens))
            .then_with(|| a.workspace_label.cmp(&b.workspace_label))
    });

    out
}
