# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`kineplex-df` Functional Analysis & Banach Space (`banach.rs`)**:
  - `BanachFrame` canonical multi-resolution structure ($N = 65.536$).
  - Sobolev $W^{1,2}$ norm and Dirichlet energy computation for topological field analysis.
  - Haar Wavelet multi-scale decomposition and Gram-Schmidt orthogonalization.
  - Cauchy fixed-point contraction operator ($T(x)$ with contraction factor $k < 1$).
  - Optimal Transport ($W_p$ / Wasserstein distance) between topological measure distributions.
  - Native bridge from Spark SQL DataFrame to `BanachFrame`.
- **Topological Metric Pruning VP-Tree (`vptree.rs`)**:
  - Cache-conscious `FlatVpNode` (32 bytes, cache-line aligned) in contiguous memory without pointer chasing.
  - Strict metric pruning via Triangle Inequality ($|d(v, q) - \mu| > r$).
  - Non-allocating $k$-Nearest Neighbors ($k$-NN) search over high-dimensional metric spaces.
- **Bare-metal SIMD Acceleration & Hardware-Sympathetic Bitboards (`baremetal.rs`)**:
  - Native FMA (Fused Multiply-Add) vector execution (`execute_native_vector_fma`).
  - Bitboard filtering using BMI instructions (`BLSR` to clear lowest set bit, `TZCNT` for trailing zeros).
  - Zero-allocation execution within dedicated L2 scratch arena (`VectorArena`, `process_with_l2_arena`).
  - Montgomery fast integer division and modulo arithmetic for prime cyclic groups.
  - Schreier 2-cocycle normalization and fast scale16 operations.
- **Zero-Dependency Micro Thread Pool (`concurrency.rs`)**:
  - Lightweight, std-only worker pool (`MicroThreadPool`) for parallel chunk evaluation across CPU cores.
- **Integration Tests for Bare-metal & Banach Engine**:
  - `banach_vptree_baremetal.rs`: 7 comprehensive tests verifying Banach frames, Cauchy contractions, VP-tree pruning, and L2 arena execution.
  - `baremetal_perf.rs`: 8 performance and correctness tests covering Schreier cocycles, bitboards, Montgomery arithmetic, and Ollivier-Ricci whole-stage pipelines.
- **Strategic Documentation & Architecture Specifications (`docs/plans/`)**:
  - `plano_diretor_kineplex_ignis_v1_0.md`: Master architectural plan for KinePlex/Ignis.
  - `racional_disrupcao_kineplex_ignis.md`: Technical and market disruption rationale.
  - `relatorio_acessibilidade_wcag_kineplex_ui.md`: WCAG 2.2 AA audit and compliance report.
  - `metrics-budget-controller.md`: Specification for per-job metrics budget control.
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

- **Accessibility Remediations WCAG 2.2 Level AA (`kineplex-ui`)**:
  - Full keyboard navigation across tabs, panels, and interactive node elements (`ArrowLeft`/`ArrowRight`, `Home`/`End`, `Enter`/`Space`).
  - Accessible ARIA semantics (`role="img"`, `role="tablist"`, `role="tab"`, `role="tabpanel"`, descriptive `aria-label`s) for Canvas visualizations (Neural mesh, Poincaré disc, Three.js 3D topology, Heatmaps, PID curves).
  - Native skip link (`.skip-link`) for keyboard accessibility.
  - Improved color contrast ratios (>6.2:1) across typography and data visualizations.
  - Support for `prefers-reduced-motion: reduce` across CSS and JavaScript animations.
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
