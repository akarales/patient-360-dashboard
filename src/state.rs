//! In-memory state with synthetic seed data: 3 patients across labs,
//! vitals, and wearables — one healthy control, one lab-drift patient,
//! one vitals+wearables patient.

use std::collections::BTreeMap;
use std::sync::RwLock;

use chrono::{NaiveDate, Utc};

use crate::domain::{ReviewItem, ReviewState, Signal, Source, build_review_items};

#[derive(Clone)]
pub struct AppState {
    inner: std::sync::Arc<Inner>,
}

pub struct Inner {
    /// patient -> signals (multi-source)
    signals: RwLock<BTreeMap<String, Vec<Signal>>>,
    /// review items by id
    reviews: RwLock<BTreeMap<String, ReviewItem>>,
}

impl AppState {
    pub fn seeded() -> Self {
        let dt = |m: u32, d: u32| {
            NaiveDate::from_ymd_opt(2026, m, d)
                .expect("date")
                .and_hms_opt(9, 0, 0)
                .expect("time")
        };
        let mut signals: BTreeMap<String, Vec<Signal>> = BTreeMap::new();
        let mut ingest = |patient: &str, rows: Vec<Signal>| {
            signals.entry(patient.to_string()).or_default().extend(rows);
        };

        // ada: labs drift (HBA1C high, LDL borderline-high)
        ingest(
            "ada",
            vec![
                Signal {
                    source: Source::Labs,
                    code: "HBA1C".into(),
                    label: "Hemoglobin A1c".into(),
                    value: 7.2,
                    unit: "%".into(),
                    taken_at: dt(9, 1),
                },
                Signal {
                    source: Source::Labs,
                    code: "LDL".into(),
                    label: "LDL cholesterol".into(),
                    value: 142.0,
                    unit: "mg/dL".into(),
                    taken_at: dt(9, 1),
                },
            ],
        );
        // grace: vitals + wearables
        ingest(
            "grace",
            vec![
                Signal {
                    source: Source::Vitals,
                    code: "SYS_BP".into(),
                    label: "Systolic BP".into(),
                    value: 152.0,
                    unit: "mmHg".into(),
                    taken_at: dt(9, 2),
                },
                Signal {
                    source: Source::Wearables,
                    code: "HRV_MS".into(),
                    label: "HRV (nightly)".into(),
                    value: 14.0,
                    unit: "ms".into(),
                    taken_at: dt(9, 2),
                },
            ],
        );
        // linus: healthy control
        ingest(
            "linus",
            vec![
                Signal {
                    source: Source::Labs,
                    code: "HBA1C".into(),
                    label: "Hemoglobin A1c".into(),
                    value: 5.1,
                    unit: "%".into(),
                    taken_at: dt(9, 3),
                },
                Signal {
                    source: Source::Vitals,
                    code: "HR".into(),
                    label: "Heart rate".into(),
                    value: 68.0,
                    unit: "bpm".into(),
                    taken_at: dt(9, 3),
                },
            ],
        );

        // Build initial review items.
        let now = Utc::now();
        let mut reviews: BTreeMap<String, ReviewItem> = BTreeMap::new();
        for (patient, rows) in &signals {
            for item in build_review_items(patient, rows, now) {
                reviews.insert(item.id.clone(), item);
            }
        }
        Self {
            inner: std::sync::Arc::new(Inner {
                signals: RwLock::new(signals),
                reviews: RwLock::new(reviews),
            }),
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: std::sync::Arc::new(Inner {
                signals: RwLock::new(BTreeMap::new()),
                reviews: RwLock::new(BTreeMap::new()),
            }),
        }
    }

    pub fn patients(&self) -> Vec<String> {
        self.inner
            .signals
            .read()
            .expect("signals lock")
            .keys()
            .cloned()
            .collect()
    }

    pub fn signals_for(&self, patient_id: &str) -> Vec<Signal> {
        self.inner
            .signals
            .read()
            .expect("signals lock")
            .get(patient_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn all_reviews(&self) -> Vec<ReviewItem> {
        self.inner
            .reviews
            .read()
            .expect("reviews lock")
            .values()
            .cloned()
            .collect()
    }

    pub fn set_review_state(&self, id: &str, state: ReviewState) -> Option<ReviewItem> {
        let mut reviews = self.inner.reviews.write().expect("reviews lock");
        let item = reviews.get_mut(id)?;
        item.state = state;
        Some(item.clone())
    }
}
