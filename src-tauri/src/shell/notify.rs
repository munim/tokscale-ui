use std::collections::HashMap;

use chrono::{Local, NaiveDate};
use tauri::{AppHandle, Wry};

use crate::contracts::{CompanionPreferences, ProviderUsageView};
use crate::shell::tray::format_usd;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertKind {
    Quota {
        provider: String,
        metric_label: String,
    },
    DailyBudget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingAlert {
    pub kind: AlertKind,
    pub dedupe_key: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Default, Clone)]
pub struct NotificationTracker {
    sent_on_date: HashMap<String, NaiveDate>,
}

impl NotificationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(
        &mut self,
        prefs: &CompanionPreferences,
        today: NaiveDate,
        today_cost: f64,
        usage: &[ProviderUsageView],
    ) -> Vec<PendingAlert> {
        if !prefs.notifications_enabled {
            return Vec::new();
        }

        // Discard entries from previous days to keep tracker bounded.
        self.sent_on_date.retain(|_, date| *date == today);

        let mut alerts = Vec::new();

        if let Some(budget) = prefs.daily_budget_usd {
            if budget.is_finite() && budget >= 0.0 && today_cost.is_finite() && today_cost >= budget
            {
                let key = "daily_budget".to_string();
                if self.sent_on_date.get(&key) != Some(&today) {
                    self.sent_on_date.insert(key.clone(), today);
                    alerts.push(PendingAlert {
                        kind: AlertKind::DailyBudget,
                        dedupe_key: key,
                        title: "Tokscale Daily Budget Reached".to_string(),
                        body: format!(
                            "Today's spend ({}) reached your {} daily budget.",
                            format_usd(today_cost),
                            format_usd(budget)
                        ),
                    });
                }
            }
        }

        let threshold = f64::from(prefs.quota_alert_percent);
        for provider in usage {
            for metric in &provider.metrics {
                let Some(used_percent) = metric.used_percent else {
                    continue;
                };
                if !used_percent.is_finite() || used_percent < threshold {
                    continue;
                }

                let key = format!(
                    "quota:{}:{}",
                    provider.provider.trim().to_lowercase(),
                    metric.label.trim().to_lowercase()
                );

                if self.sent_on_date.get(&key) != Some(&today) {
                    self.sent_on_date.insert(key.clone(), today);
                    alerts.push(PendingAlert {
                        kind: AlertKind::Quota {
                            provider: provider.provider.clone(),
                            metric_label: metric.label.clone(),
                        },
                        dedupe_key: key,
                        title: format!("Tokscale Quota Alert: {}", provider.provider),
                        body: format!(
                            "{} is at {:.0}% used (threshold {}%).",
                            metric.label, used_percent, prefs.quota_alert_percent
                        ),
                    });
                }
            }
        }

        alerts
    }
}

pub fn current_local_date() -> NaiveDate {
    Local::now().date_naive()
}

pub fn dispatch_alerts(app: &AppHandle<Wry>, alerts: &[PendingAlert]) {
    use tauri_plugin_notification::NotificationExt;

    if alerts.is_empty() {
        return;
    }

    for alert in alerts {
        let _ = app
            .notification()
            .builder()
            .title(&alert.title)
            .body(&alert.body)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::UsageMetricView;

    fn sample_usage(five_hour_pct: f64, seven_day_pct: f64) -> Vec<ProviderUsageView> {
        vec![ProviderUsageView {
            provider: "claude".to_string(),
            email: None,
            plan: Some("max".to_string()),
            metrics: vec![
                UsageMetricView {
                    label: "5-hour window".to_string(),
                    remaining_label: None,
                    remaining_percent: Some(100.0 - five_hour_pct),
                    used_percent: Some(five_hour_pct),
                    resets_at: None,
                },
                UsageMetricView {
                    label: "7-day window".to_string(),
                    remaining_label: None,
                    remaining_percent: Some(100.0 - seven_day_pct),
                    used_percent: Some(seven_day_pct),
                    resets_at: None,
                },
            ],
        }]
    }

    #[test]
    fn notifications_are_disabled_by_default() {
        let mut tracker = NotificationTracker::new();
        let prefs = CompanionPreferences {
            daily_budget_usd: Some(10.0),
            ..CompanionPreferences::default()
        };
        assert!(!prefs.notifications_enabled);

        let day = NaiveDate::from_ymd_opt(2026, 11, 21).unwrap();
        let alerts = tracker.evaluate(&prefs, day, 50.0, &sample_usage(95.0, 92.0));
        assert!(alerts.is_empty());
    }

    #[test]
    fn quota_and_budget_thresholds_fire_once_per_metric_per_day() {
        let mut tracker = NotificationTracker::new();
        let prefs = CompanionPreferences {
            notifications_enabled: true,
            quota_alert_percent: 85,
            daily_budget_usd: Some(25.0),
            ..CompanionPreferences::default()
        };

        let day1 = NaiveDate::from_ymd_opt(2026, 11, 21).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();

        // First evaluation on day1: 5-hour >= 85 and daily budget >= 25 fire.
        let first = tracker.evaluate(&prefs, day1, 25.0, &sample_usage(85.0, 40.0));
        assert_eq!(first.len(), 2);
        assert!(first.iter().any(|a| a.kind == AlertKind::DailyBudget));
        assert!(first.iter().any(|a| matches!(
            &a.kind,
            AlertKind::Quota { metric_label, .. } if metric_label == "5-hour window"
        )));

        // Second evaluation same day: already-fired metrics do not repeat,
        // while a newly crossed 7-day metric fires once.
        let second = tracker.evaluate(&prefs, day1, 40.0, &sample_usage(90.0, 88.0));
        assert_eq!(second.len(), 1);
        assert!(matches!(
            &second[0].kind,
            AlertKind::Quota { metric_label, .. } if metric_label == "7-day window"
        ));

        // Third evaluation same day: nothing fires.
        let third = tracker.evaluate(&prefs, day1, 60.0, &sample_usage(95.0, 95.0));
        assert!(third.is_empty());

        // Next day: alerts are eligible to fire once again.
        let next_day = tracker.evaluate(&prefs, day2, 30.0, &sample_usage(86.0, 40.0));
        assert_eq!(next_day.len(), 2);
    }
}
