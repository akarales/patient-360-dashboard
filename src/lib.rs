//! patient-360-dashboard — a clinical Copilot.
//!
//! Aggregate patient data from multiple sources (labs, vitals,
//! wearables), derive risk signals, maintain a physician **priority
//! queue**, synthesize an AI narrative (stub mode default), and run the
//! **review workflow** (acknowledge / escalate). Pure domain logic in
//! `domain.rs` (tested without HTTP); `routes.rs` stays thin.

pub mod domain;
pub mod routes;
pub mod state;
