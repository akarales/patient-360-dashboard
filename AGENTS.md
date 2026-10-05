# AGENTS.md

## Commands

```bash
cargo run                  # :8007 (seeded synthetic patients)
cargo test -q              # 12 tests (5 domain + 7 integration)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cd frontend && pnpm install && pnpm dev && pnpm build
```

## Environment

`APP_PORT` (8007)

## Conventions

- Conventional commits; hygiene hook strips AI attribution
- Domain logic stays pure in `src/domain.rs` (thresholds, queue, synthesis)
  — no HTTP, no clock; routes stay thin
- Threshold changes are documented in `domain.rs::thresholds` and covered
  by tests
- Deps ≥7 days old (BEST_PRACTICES/INDEX.md)
