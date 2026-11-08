# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Metrics Budget Controller (FT-038/FT-079)** — per-job metrics with a hard
  1% CPU overhead budget:
  - `kineplex-core`: `Merge` monoid laws, Welford, DDSketch (α = 1%) and a
    bare-metal hot path (`#[repr(align(64))]` atomics, thread-local
    `JobScope`, zero alloc/lock/format).
  - `JobMetricsRegistry` with 1 s Prometheus export cache and finished-job
    TTL.
  - `BudgetController` PI loop (kp = 0.5, ki = 0.2) adjusting the sample rate
    so amortized overhead stays under `OVERHEAD_BUDGET`; gauges exported at
    `GET /metrics`.
  - Instrumentation of `execute_real_pipeline` and
    `PipelineExecutor::execute` with real stage timers; removed the
    hardcoded `500_000` row count.
  - On/off overhead test: ~4 µs per instrumented job, median overhead
    0.04 %–0.73 % (budget 1 %). Method documented in
    [`docs/observability.md`](docs/observability.md).
- **`kineplex-df`** — new workspace crate: zero-dependency, std-only columnar
  DataFrame and Spark-SQL-style engine (SQL DSL, Catalyst optimization
  pipeline with fusion/optimizer, B3/BACEN O(1) decimal kernel,
  hardware-sympathetic memory layout).

### Changed

- `GET /metrics` now also reports `kineplex_metrics_overhead_ratio` and
  `kineplex_metrics_sample_rate` gauges from the budget controller.

## [0.1.0] — 2026-11-08

### Added

- Distributed streaming module with telemetry server, dashboard UI and specs.
- Ollivier-Ricci geometric flow benchmarks and kernel 6.8 build fixes
  (FT-089..FT-096): per-CPU XDP telemetry, strict NUMA-local tensor
  allocation, eBPF link lifecycle management, CAP_NET_ADMIN enforcement on
  the geometry API.
- Reproducible local performance evaluation harness (FT-081, FT-082).
- Readiness features FT-071..FT-092 with 95 % test-coverage gate.
- Automated versioning and release pipeline (GitHub Actions).

### Fixed

- Restored workspace build and enforced 95 % coverage in CI.
