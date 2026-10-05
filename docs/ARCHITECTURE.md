# Architecture

## Modules

```
src/
├── domain.rs    # pure: Source, Signal, thresholds, ReviewItem, queue, synthesis
├── routes.rs    # thin axum handlers over the domain
└── state.rs     # seeded AppState: signals + reviews (RwLock)
```

Domain logic is pure (no HTTP, no clock — callers pass `now`) so every
rule is unit-testable without infrastructure; routes stay thin.

## The domain model

- **Signal** — one reading: source (Labs/Vitals/Wearables), code, label,
  value, unit, timestamp
- **Threshold bands** (the documented risk table):

| Code | Low | High | Weight |
|------|-----|------|--------|
| HBA1C | 4.0 | 5.7 | 25 |
| LDL | 0 | 130 | 20 |
| SYS_BP | 90 | 130 | 15 |
| HR | 50 | 100 | 10 |
| HRV_MS | 25 | — | 10 (low HRV is the risk) |

- **ReviewItem** — per (patient, source) group of out-of-band signals;
  risk score = Σ weights; rationale strings name every breach with the
  bound and direction
- **Priority queue** — pending items sorted by risk desc, recency
  tiebreak; `Acknowledged`/`Escalated` items leave the queue
- **Review state machine** — `Pending → Acknowledged | Escalated`
  (terminal for the queue's purposes; full history is a roadmap item)
- **Synthesis** — deterministic stub narrative for the patient's
  highest-risk pending item (source, signal count, rationale, disclaimer)

## Store (src/state.rs)

Seeded synthetic patients (ada: labs drift, grace: vitals + wearables,
linus: healthy control). Reviews are built once at seed; the workflow
mutates state through `set_review_state`. The Postgres store is Phase 1.

## Design decisions

| Decision | Why |
|----------|-----|
| Thresholds as a pure function + table | Auditable — changing a weight is a visible, tested change |
| Per-source grouping of items | Mirrors how physicians triage: one review per data source per patient |
| Queue excludes non-pending | "What needs my attention now" — the whole point of the view |
