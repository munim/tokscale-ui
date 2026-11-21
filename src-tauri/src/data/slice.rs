use std::collections::{BTreeSet, HashMap};

use chrono::{Duration, Local, NaiveDate};

use crate::contracts::{
    DailySourceBreakdown, DailyViewRow, DashboardQuery, HourlyViewRow, ModelViewRow,
    MonthlyViewRow, RangePreset, SessionRow, SourceWarning, TokenCounts as ContractTokenCounts,
};
use crate::data::harness;
use crate::data::schema::{DailyRow, TokenCounts, TuiData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateBounds {
    pub since: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
}

impl DateBounds {
    pub fn contains(&self, date: NaiveDate) -> bool {
        if let Some(since) = self.since {
            if date < since {
                return false;
            }
        }
        if let Some(until) = self.until {
            if date > until {
                return false;
            }
        }
        true
    }

    pub fn contains_str(&self, date_or_datetime: &str) -> bool {
        if self.since.is_none() && self.until.is_none() {
            return true;
        }
        let Some(date) = parse_date_prefix(date_or_datetime) else {
            return false;
        };
        self.contains(date)
    }
}

pub fn local_today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn parse_date_prefix(value: &str) -> Option<NaiveDate> {
    let prefix = value.trim().get(..10)?;
    NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok()
}

pub fn resolve_bounds(
    range: RangePreset,
    custom_since: Option<&str>,
    custom_until: Option<&str>,
    today: NaiveDate,
) -> Result<DateBounds, SourceWarning> {
    match range {
        RangePreset::Today => Ok(DateBounds {
            since: Some(today),
            until: Some(today),
        }),
        RangePreset::SevenDays => Ok(DateBounds {
            since: Some(today - Duration::days(6)),
            until: Some(today),
        }),
        RangePreset::ThirtyDays => Ok(DateBounds {
            since: Some(today - Duration::days(29)),
            until: Some(today),
        }),
        RangePreset::All => Ok(DateBounds {
            since: None,
            until: None,
        }),
        RangePreset::Custom => {
            let since = match custom_since.map(str::trim).filter(|s| !s.is_empty()) {
                Some(s) => match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    Ok(d) => Some(d),
                    Err(_) => {
                        return Err(SourceWarning {
                            source: "slice".to_string(),
                            message: format!("Invalid customSince date '{s}'; expected YYYY-MM-DD"),
                        });
                    }
                },
                None => None,
            };

            let until = match custom_until.map(str::trim).filter(|s| !s.is_empty()) {
                Some(u) => match NaiveDate::parse_from_str(u, "%Y-%m-%d") {
                    Ok(d) => Some(d),
                    Err(_) => {
                        return Err(SourceWarning {
                            source: "slice".to_string(),
                            message: format!("Invalid customUntil date '{u}'; expected YYYY-MM-DD"),
                        });
                    }
                },
                None => None,
            };

            if let (Some(s), Some(u)) = (since, until) {
                if s > u {
                    return Err(SourceWarning {
                        source: "slice".to_string(),
                        message: format!(
                            "Invalid custom range: since ({s}) is after until ({u})"
                        ),
                    });
                }
            }

            Ok(DateBounds { since, until })
        }
    }
}

pub fn sum_tokens(rows: &[DailyRow]) -> TokenCounts {
    let mut total = TokenCounts::default();
    for row in rows {
        total.add_assign(&row.tokens);
    }
    total
}

pub fn sum_cost(rows: &[DailyRow]) -> f64 {
    rows.iter().map(|r| r.cost).sum()
}

pub fn contract_tokens_total(t: &ContractTokenCounts) -> u64 {
    t.input
        .saturating_add(t.output)
        .saturating_add(t.cache_read)
        .saturating_add(t.cache_write)
        .saturating_add(t.reasoning)
}

pub fn client_matches(client: &str, active_clients: &[String]) -> bool {
    if active_clients.is_empty() {
        return true;
    }
    let candidate = client.trim();
    active_clients
        .iter()
        .any(|ac| ac.trim().eq_ignore_ascii_case(candidate))
}

pub fn client_list_matches(raw_clients: &str, active_clients: &[String]) -> bool {
    if active_clients.is_empty() {
        return true;
    }
    raw_clients
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .any(|c| client_matches(c, active_clients))
}

pub fn normalize_search(search: Option<&str>) -> Option<String> {
    search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
}

pub fn contains_ci(haystack: &str, needle_lower: &str) -> bool {
    haystack.to_lowercase().contains(needle_lower)
}

pub fn slice_raw_daily<'a>(
    daily: &'a [DailyRow],
    bounds: &DateBounds,
) -> Vec<&'a DailyRow> {
    daily
        .iter()
        .filter(|row| bounds.contains_str(&row.date))
        .collect()
}

pub fn daily_for_range(
    data: &TuiData,
    query: &DashboardQuery,
    today: NaiveDate,
) -> (Vec<DailyViewRow>, Vec<SourceWarning>) {
    let bounds = match resolve_bounds(
        query.range,
        query.custom_since.as_deref(),
        query.custom_until.as_deref(),
        today,
    ) {
        Ok(b) => b,
        Err(warn) => return (Vec::new(), vec![warn]),
    };

    let needle = normalize_search(query.search.as_deref());
    let has_client_filter = !query.active_clients.is_empty();
    let mut rows = Vec::new();

    for row in &data.daily {
        if !bounds.contains_str(&row.date) {
            continue;
        }

        let mut breakdown = Vec::new();
        let mut filtered_tokens = TokenCounts::default();
        let mut filtered_cost = 0.0_f64;

        for (client, sb) in &row.source_breakdown {
            if !client_matches(client, &query.active_clients) {
                continue;
            }
            let meta = harness::resolve(client);
            filtered_tokens.add_assign(&sb.tokens);
            filtered_cost += sb.cost;
            breakdown.push(DailySourceBreakdown {
                client: client.clone(),
                label: meta.label,
                tokens: sb.tokens.into(),
                cost: sb.cost,
            });
        }

        if has_client_filter && breakdown.is_empty() {
            continue;
        }

        let (tokens, cost) = if has_client_filter {
            (filtered_tokens.into(), filtered_cost)
        } else {
            (row.tokens.into(), row.cost)
        };

        if let Some(ref q) = needle {
            let matches_date = contains_ci(&row.date, q);
            let matches_client = breakdown
                .iter()
                .any(|b| contains_ci(&b.client, q) || contains_ci(&b.label, q));
            let matches_model = row.source_breakdown.iter().any(|(c, sb)| {
                client_matches(c, &query.active_clients)
                    && sb.models.iter().any(|(m_key, m)| {
                        contains_ci(m_key, q)
                            || contains_ci(&m.display_name, q)
                            || contains_ci(&m.provider, q)
                    })
            });
            if !matches_date && !matches_client && !matches_model {
                continue;
            }
        }

        breakdown.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| a.client.cmp(&b.client))
        });

        rows.push(DailyViewRow {
            date: row.date.clone(),
            tokens,
            cost,
            message_count: row.message_count,
            turn_count: row.turn_count,
            source_breakdown: breakdown,
        });
    }

    sort_daily_rows(&mut rows, query.sort.as_deref());
    (rows, Vec::new())
}

fn sort_daily_rows(rows: &mut [DailyViewRow], sort: Option<&str>) {
    match sort.map(str::trim).unwrap_or("date") {
        "cost" => rows.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| contract_tokens_total(&b.tokens).cmp(&contract_tokens_total(&a.tokens)))
                .then_with(|| b.date.cmp(&a.date))
        }),
        "tokens" => rows.sort_by(|a, b| {
            contract_tokens_total(&b.tokens)
                .cmp(&contract_tokens_total(&a.tokens))
                .then_with(|| b.cost.total_cmp(&a.cost))
                .then_with(|| b.date.cmp(&a.date))
        }),
        _ => rows.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then_with(|| b.cost.total_cmp(&a.cost))
        }),
    }
}

pub fn models_for_range(
    data: &TuiData,
    query: &DashboardQuery,
    today: NaiveDate,
) -> (Vec<ModelViewRow>, Vec<SourceWarning>) {
    let bounds = match resolve_bounds(
        query.range,
        query.custom_since.as_deref(),
        query.custom_until.as_deref(),
        today,
    ) {
        Ok(b) => b,
        Err(warn) => return (Vec::new(), vec![warn]),
    };

    let needle = normalize_search(query.search.as_deref());
    let group_by_client = query
        .group_by
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case("client"));

    let mut out = if query.range == RangePreset::All && !group_by_client {
        let mut rows = Vec::new();
        for row in &data.models {
            if !client_list_matches(&row.client, &query.active_clients) {
                continue;
            }
            let view = ModelViewRow {
                model: row.model.clone(),
                color_key: if row.color_key.is_empty() {
                    row.model.clone()
                } else {
                    row.color_key.clone()
                },
                provider: row.provider.clone(),
                client: row.client.clone(),
                workspace_key: row.workspace_key.clone(),
                workspace_label: row.workspace_label.clone(),
                session_id: None,
                tokens: row.tokens.into(),
                cost: row.cost,
                session_count: row.session_count,
                message_count: 0,
                ms_per_1k_tokens: row.ms_per_1k_tokens(),
                group_key: if row.group_key.is_empty() {
                    row.model.clone()
                } else {
                    row.group_key.clone()
                },
            };
            if matches_model_search(&view, needle.as_deref()) {
                rows.push(view);
            }
        }
        rows
    } else {
        struct ModelAgg {
            model: String,
            color_key: String,
            providers: BTreeSet<String>,
            clients: BTreeSet<String>,
            tokens: TokenCounts,
            cost: f64,
            message_count: u64,
            weighted_duration_ms: f64,
            timed_tokens: u64,
        }

        let mut agg: HashMap<String, ModelAgg> = HashMap::new();

        for day in &data.daily {
            if !bounds.contains_str(&day.date) {
                continue;
            }
            for (client_key, sb) in &day.source_breakdown {
                if !client_matches(client_key, &query.active_clients) {
                    continue;
                }
                for (model_key, entry) in &sb.models {
                    let model_name = if entry.display_name.is_empty() {
                        model_key.clone()
                    } else {
                        entry.display_name.clone()
                    };
                    let effective_client = if entry.client.trim().is_empty() {
                        client_key.clone()
                    } else {
                        entry.client.clone()
                    };
                    let group_key = if group_by_client {
                        format!("{}:{}", effective_client.to_ascii_lowercase(), model_name)
                    } else {
                        model_name.clone()
                    };

                    let slot = agg.entry(group_key).or_insert_with(|| ModelAgg {
                        model: model_name.clone(),
                        color_key: if entry.color_key.is_empty() {
                            model_name.clone()
                        } else {
                            entry.color_key.clone()
                        },
                        providers: BTreeSet::new(),
                        clients: BTreeSet::new(),
                        tokens: TokenCounts::default(),
                        cost: 0.0,
                        message_count: 0,
                        weighted_duration_ms: 0.0,
                        timed_tokens: 0,
                    });

                    if !entry.provider.trim().is_empty() {
                        slot.providers.insert(entry.provider.trim().to_string());
                    }
                    if !effective_client.trim().is_empty() {
                        slot.clients.insert(effective_client.trim().to_string());
                    }
                    slot.tokens.add_assign(&entry.tokens);
                    slot.cost += entry.cost;
                    slot.message_count = slot.message_count.saturating_add(entry.messages);
                    if let Some(ms) = entry.ms_per_1k_tokens() {
                        let t = entry.tokens.total();
                        if ms.is_finite() && t > 0 {
                            slot.weighted_duration_ms += ms * (t as f64 / 1000.0);
                            slot.timed_tokens = slot.timed_tokens.saturating_add(t);
                        }
                    }
                }
            }
        }

        // Fallback to `data.models` if `RangePreset::All` with `group_by_client` had no daily breakdown.
        if agg.is_empty() && query.range == RangePreset::All && !data.models.is_empty() {
            for row in &data.models {
                if !client_list_matches(&row.client, &query.active_clients) {
                    continue;
                }
                let group_key = if row.group_key.is_empty() {
                    format!("{}:{}", row.client, row.model)
                } else {
                    row.group_key.clone()
                };
                let slot = agg.entry(group_key).or_insert_with(|| ModelAgg {
                    model: row.model.clone(),
                    color_key: row.color_key.clone(),
                    providers: BTreeSet::new(),
                    clients: BTreeSet::new(),
                    tokens: TokenCounts::default(),
                    cost: 0.0,
                    message_count: 0,
                    weighted_duration_ms: 0.0,
                    timed_tokens: 0,
                });
                if !row.provider.trim().is_empty() {
                    slot.providers.insert(row.provider.trim().to_string());
                }
                for c in row.client.split(',').map(str::trim).filter(|c| !c.is_empty()) {
                    slot.clients.insert(c.to_string());
                }
                slot.tokens.add_assign(&row.tokens);
                slot.cost += row.cost;
            }
        }

        let mut rows = Vec::with_capacity(agg.len());
        for (group_key, item) in agg {
            let ms_per_1k_tokens = if item.timed_tokens > 0 {
                Some(item.weighted_duration_ms / (item.timed_tokens as f64 / 1000.0))
            } else {
                None
            };
            let view = ModelViewRow {
                model: item.model,
                color_key: item.color_key,
                provider: item.providers.into_iter().collect::<Vec<_>>().join(", "),
                client: item.clients.into_iter().collect::<Vec<_>>().join(", "),
                workspace_key: None,
                workspace_label: None,
                session_id: None,
                tokens: item.tokens.into(),
                cost: item.cost,
                session_count: 0,
                message_count: item.message_count,
                ms_per_1k_tokens,
                group_key,
            };
            if matches_model_search(&view, needle.as_deref()) {
                rows.push(view);
            }
        }
        rows
    };

    sort_model_rows(&mut out, query.sort.as_deref());
    (out, Vec::new())
}

fn matches_model_search(row: &ModelViewRow, needle_lower: Option<&str>) -> bool {
    let Some(q) = needle_lower else {
        return true;
    };
    contains_ci(&row.model, q)
        || contains_ci(&row.provider, q)
        || contains_ci(&row.client, q)
        || row
            .workspace_label
            .as_deref()
            .is_some_and(|w| contains_ci(w, q))
        || row
            .session_id
            .as_deref()
            .is_some_and(|s| contains_ci(s, q))
}

fn sort_model_rows(rows: &mut [ModelViewRow], sort: Option<&str>) {
    match sort.map(str::trim).unwrap_or("cost") {
        "tokens" => rows.sort_by(|a, b| {
            contract_tokens_total(&b.tokens)
                .cmp(&contract_tokens_total(&a.tokens))
                .then_with(|| b.cost.total_cmp(&a.cost))
                .then_with(|| a.model.cmp(&b.model))
                .then_with(|| a.client.cmp(&b.client))
        }),
        _ => rows.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| contract_tokens_total(&b.tokens).cmp(&contract_tokens_total(&a.tokens)))
                .then_with(|| a.model.cmp(&b.model))
                .then_with(|| a.client.cmp(&b.client))
        }),
    }
}

pub fn hourly_for_range(
    data: &TuiData,
    query: &DashboardQuery,
    today: NaiveDate,
) -> (Vec<HourlyViewRow>, Vec<SourceWarning>) {
    let bounds = match resolve_bounds(
        query.range,
        query.custom_since.as_deref(),
        query.custom_until.as_deref(),
        today,
    ) {
        Ok(b) => b,
        Err(warn) => return (Vec::new(), vec![warn]),
    };

    let needle = normalize_search(query.search.as_deref());
    let mut rows = Vec::new();

    for row in &data.hourly {
        if !bounds.contains_str(&row.datetime) {
            continue;
        }
        if !query.active_clients.is_empty()
            && !row
                .clients
                .iter()
                .any(|c| client_matches(c, &query.active_clients))
        {
            continue;
        }

        let top_model = row
            .models
            .iter()
            .max_by(|(ak, av), (bk, bv)| {
                av.cost
                    .total_cmp(&bv.cost)
                    .then_with(|| av.tokens.total().cmp(&bv.tokens.total()))
                    .then_with(|| bk.cmp(ak))
            })
            .map(|(k, v)| {
                if v.display_name.is_empty() {
                    k.clone()
                } else {
                    v.display_name.clone()
                }
            });

        if let Some(ref q) = needle {
            let match_dt = contains_ci(&row.datetime, q);
            let match_client = row.clients.iter().any(|c| contains_ci(c, q));
            let match_model = row.models.iter().any(|(k, m)| {
                contains_ci(k, q)
                    || contains_ci(&m.display_name, q)
                    || contains_ci(&m.provider, q)
            });
            if !match_dt && !match_client && !match_model {
                continue;
            }
        }

        rows.push(HourlyViewRow {
            datetime: row.datetime.clone(),
            tokens: row.tokens.into(),
            cost: row.cost,
            clients: row.clients.clone(),
            top_model,
            message_count: row.message_count,
            turn_count: row.turn_count,
        });
    }

    match query.sort.as_deref().map(str::trim).unwrap_or("date") {
        "cost" => rows.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| contract_tokens_total(&b.tokens).cmp(&contract_tokens_total(&a.tokens)))
                .then_with(|| b.datetime.cmp(&a.datetime))
        }),
        "tokens" => rows.sort_by(|a, b| {
            contract_tokens_total(&b.tokens)
                .cmp(&contract_tokens_total(&a.tokens))
                .then_with(|| b.cost.total_cmp(&a.cost))
                .then_with(|| b.datetime.cmp(&a.datetime))
        }),
        _ => rows.sort_by(|a, b| b.datetime.cmp(&a.datetime)),
    }

    (rows, Vec::new())
}

pub fn monthly_for_range(
    daily_views: &[DailyViewRow],
    raw_daily: &[DailyRow],
    active_clients: &[String],
    sessions: &[SessionRow],
) -> Vec<MonthlyViewRow> {
    struct MonthAgg {
        models: BTreeSet<String>,
        tokens: TokenCounts,
        message_count: u64,
        session_count: u64,
        active_days: u32,
        cost: f64,
    }

    let mut raw_by_date: HashMap<&str, &DailyRow> = HashMap::with_capacity(raw_daily.len());
    for row in raw_daily {
        raw_by_date.insert(row.date.as_str(), row);
    }

    let mut agg: HashMap<String, MonthAgg> = HashMap::new();

    for day in daily_views {
        let Some(month) = day.date.get(..7) else {
            continue;
        };
        let slot = agg.entry(month.to_string()).or_insert_with(|| MonthAgg {
            models: BTreeSet::new(),
            tokens: TokenCounts::default(),
            message_count: 0,
            session_count: 0,
            active_days: 0,
            cost: 0.0,
        });

        let day_tokens: TokenCounts = day.tokens.into();
        slot.tokens.add_assign(&day_tokens);
        slot.cost += day.cost;
        slot.message_count = slot.message_count.saturating_add(day.message_count);
        if day.cost > 0.0 || contract_tokens_total(&day.tokens) > 0 {
            slot.active_days += 1;
        }

        if let Some(raw) = raw_by_date.get(day.date.as_str()) {
            for (client, sb) in &raw.source_breakdown {
                if !client_matches(client, active_clients) {
                    continue;
                }
                for (model_key, entry) in &sb.models {
                    let name = if entry.display_name.is_empty() {
                        model_key.as_str()
                    } else {
                        entry.display_name.as_str()
                    };
                    if !name.is_empty() {
                        slot.models.insert(name.to_string());
                    }
                }
            }
        }
    }

    for session in sessions {
        let month_opt = session
            .last_active
            .as_deref()
            .or(session.created_at.as_deref())
            .and_then(|s| s.get(..7));
        if let Some(month) = month_opt {
            if let Some(slot) = agg.get_mut(month) {
                slot.session_count = slot.session_count.saturating_add(1);
            }
        }
    }

    let mut out: Vec<MonthlyViewRow> = agg
        .into_iter()
        .map(|(month, item)| MonthlyViewRow {
            month,
            models: item.models.into_iter().collect(),
            tokens: item.tokens.into(),
            message_count: item.message_count,
            session_count: item.session_count,
            active_days: item.active_days,
            cost: item.cost,
        })
        .collect();

    out.sort_by(|a, b| b.month.cmp(&a.month));
    out
}
