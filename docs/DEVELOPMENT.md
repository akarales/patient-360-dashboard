# Development Guide

Machine-facing commands live in [AGENTS.md](../AGENTS.md).

## Prerequisites

Rust 1.96, cargo · pnpm 11 / Node 24.

## Daily loop

```bash
cargo run             # :8007, seeded synthetic patients
cd frontend && pnpm dev  # :5173 → /api proxied
cargo test -q         # 12 tests — 5 domain + 7 integration
cargo clippy --all-targets -- -D warnings
```

## Testing notes

- **Domain tests** assert semantics: out-of-band signals create items,
  in-band signals don't, queue ordering is risk-desc with recency
  tiebreak, acknowledged items leave the queue, synthesis is disclaimed
  and mentions the source
- **Integration tests** run the router in-process; the workflow test uses
  ONE app instance for its stateful acknowledge→escalate sequence

## Extending the risk table

Adding a biomarker or changing a weight is a deliberate, visible change:

1. Update `thresholds()` in `src/domain.rs` (band + weight + comment)
2. Add/adjust the domain test that pins the behavior
3. Add the signal to the seed if the demo should show it

Threshold changes are policy changes — they ship with their tests.

## Gotchas learned here

- BTreeMap keys need `Ord` — the `Source` enum derives it
- chrono deprecation: `DateTime::from_timestamp(...).naive_utc()` not
  `NaiveDateTime::from_timestamp_opt`
- Stateful flow tests need one router instance (clone it)

## Conventions

Conventional commits; hygiene hook strips AI attribution. Domain logic
stays pure in `domain.rs` — no HTTP, no clock access.
