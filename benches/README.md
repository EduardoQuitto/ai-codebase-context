# Benchmarks (skeleton — real datasets in Fase 23–24)

Planned criterion benches (not wired yet to keep the foundation lean):

- `cold_index`: fresh index over `small / medium / large` fixtures.
- `warm_index` / `incremental_index`: single-file change cost.
- `search_lexical` / `context_build`: latency + memory + disk footprint.

Rules: measure before/after, document hardware, never tune weights to a demo. See ROADMAP §58.
