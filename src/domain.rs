//! Domain: aggregation, risk signals, priority queue, review workflow.
//! Pure data + functions — no HTTP, no clock (callers pass "now").

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Source {
    Labs,
    Vitals,
    Wearables,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::Labs => "labs",
            Source::Vitals => "vitals",
            Source::Wearables => "wearables",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub source: Source,
    pub code: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub taken_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewState {
    Pending,
    Acknowledged,
    Escalated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub patient_id: String,
    pub signals: Vec<Signal>,
    pub risk_score: f64,
    pub rationale: String,
    pub state: ReviewState,
    pub created_at: DateTime<Utc>,
}

/// Thresholds per signal code: (low, high) panic bounds outside which a
/// signal contributes risk weight.
fn thresholds(code: &str) -> Option<(f64, f64, f64)> {
    // (low, high, weight) — weight is the risk contribution
    match code {
        "HBA1C" => Some((4.0, 5.7, 25.0)), // % — above 5.7 = prediabetic range
        "LDL" => Some((0.0, 130.0, 20.0)), // mg/dL
        "SYS_BP" => Some((90.0, 130.0, 15.0)), // mmHg
        "HR" => Some((50.0, 100.0, 10.0)), // bpm
        "HRV_MS" => Some((25.0, 999.0, 10.0)), // ms — low HRV is the risk
        _ => None,
    }
}

/// Aggregate signals per source, derive risk items: each signal outside
/// its threshold band contributes its weight; grouped per (patient,
/// source) into review items with a rationale.
pub fn build_review_items(
    patient_id: &str,
    signals: &[Signal],
    now: DateTime<Utc>,
) -> Vec<ReviewItem> {
    let by_source: BTreeMap<&Source, Vec<&Signal>> = {
        let mut map: BTreeMap<&Source, Vec<&Signal>> = BTreeMap::new();
        for signal in signals {
            map.entry(&signal.source).or_default().push(signal);
        }
        map
    };

    by_source
        .into_iter()
        .filter_map(|(source, group)| {
            let mut score = 0.0;
            let mut reasons: Vec<String> = Vec::new();
            let mut owned = Vec::new();
            for signal in group {
                if let Some((low, high, weight)) = thresholds(&signal.code) {
                    let (breach, direction) = if signal.value < low {
                        (true, "low")
                    } else if signal.value > high {
                        (true, "high")
                    } else {
                        (false, "")
                    };
                    if breach {
                        score += weight;
                        reasons.push(format!(
                            "{} {} {} {} (bound {}–{})",
                            signal.code, signal.value, signal.unit, direction, low, high
                        ));
                    }
                }
                owned.push(signal.clone());
            }
            if score <= 0.0 {
                return None;
            }
            Some(ReviewItem {
                id: format!("rev-{}-{}", patient_id, source.label()),
                patient_id: patient_id.to_string(),
                signals: owned,
                risk_score: score,
                rationale: reasons.join("; "),
                state: ReviewState::Pending,
                created_at: now,
            })
        })
        .collect()
}

/// Priority queue: pending items sorted by risk score desc, then recency.
pub fn prioritize(items: &[ReviewItem]) -> Vec<&ReviewItem> {
    let mut pending: Vec<&ReviewItem> = items
        .iter()
        .filter(|item| item.state == ReviewState::Pending)
        .collect();
    pending.sort_by(|a, b| {
        b.risk_score
            .partial_cmp(&a.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
    pending
}

/// Stub narrative synthesis — deterministic, GPU-less. Real mode (Ollama)
/// arrives in Phase 2.
pub fn synthesize(patient_id: &str, item: &ReviewItem) -> String {
    format!(
        "Patient {} has {} review-worthy signal(s) from {}: {}. Suggested action: \
review recent trends and confirm with the patient. This is a demo summary, \
not medical advice.",
        patient_id,
        item.signals.len(),
        item.signals[0].source.label(),
        item.rationale
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(source: Source, code: &str, value: f64, unit: &str) -> Signal {
        Signal {
            source,
            code: code.into(),
            label: code.into(),
            value,
            unit: unit.into(),
            taken_at: chrono::DateTime::from_timestamp(1759000000, 0)
                .expect("valid ts")
                .naive_utc(),
        }
    }

    const NOW: DateTime<Utc> = chrono::DateTime::from_timestamp(1759000000, 0).expect("ts");

    #[test]
    fn out_of_band_signals_create_items() {
        let signals = vec![
            signal(Source::Labs, "HBA1C", 7.2, "%"),
            signal(Source::Labs, "LDL", 100.0, "mg/dL"),
            signal(Source::Vitals, "SYS_BP", 155.0, "mmHg"),
        ];
        let items = build_review_items("ada", &signals, NOW);
        assert_eq!(items.len(), 2, "labs + vitals items, LDL in band");
        let labs = items
            .iter()
            .find(|i| i.signals[0].source == Source::Labs)
            .expect("labs");
        assert!(labs.risk_score >= 25.0, "HBA1C out of band contributes");
        assert!(labs.rationale.contains("HBA1C"));
    }

    #[test]
    fn in_band_signals_create_nothing() {
        let signals = vec![
            signal(Source::Labs, "HBA1C", 5.2, "%"),
            signal(Source::Vitals, "HR", 72.0, "bpm"),
        ];
        assert!(build_review_items("ada", &signals, NOW).is_empty());
    }

    #[test]
    fn priority_orders_by_risk_desc() {
        let make = |id: &str, score: f64| ReviewItem {
            id: id.into(),
            patient_id: "p".into(),
            signals: vec![signal(Source::Labs, "HBA1C", 7.0, "%")],
            risk_score: score,
            rationale: "x".into(),
            state: ReviewState::Pending,
            created_at: NOW,
        };
        let items = vec![make("a", 10.0), make("b", 45.0), make("c", 25.0)];
        let queue = prioritize(&items);
        let ids: Vec<&str> = queue.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn acknowledged_items_leave_the_queue() {
        let mut item = ReviewItem {
            id: "a".into(),
            patient_id: "p".into(),
            signals: vec![signal(Source::Wearables, "HRV_MS", 12.0, "ms")],
            risk_score: 10.0,
            rationale: "x".into(),
            state: ReviewState::Pending,
            created_at: NOW,
        };
        item.state = ReviewState::Acknowledged;
        assert!(prioritize(&[item]).is_empty());
    }

    #[test]
    fn synthesis_mentions_source_and_is_disclaimed() {
        let item = build_review_items(
            "ada",
            &[signal(Source::Vitals, "SYS_BP", 160.0, "mmHg")],
            NOW,
        )
        .pop()
        .expect("item");
        let text = synthesize("ada", &item);
        assert!(text.contains("vitals"));
        assert!(text.contains("not medical advice"));
    }
}
