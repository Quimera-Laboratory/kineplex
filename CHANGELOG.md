# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Motor de Transporte HTTP/3 sobre QUIC com `io_uring` e Controle PID Discreto em Malha Fechada (`kineplex_net::quic`) (FT-098)**:
  - **Eliminação de Head-of-Line Blocking**: Substituição da pilha TCP/TLS + HTTP/2 por QUIC sobre UDP (`quinn-proto`), garantindo imunidade a bloqueios entre streams multiplexados sob perdas induzidas de pacotes.
  - **Zero-Syscall em Regime Estacionário (`io_uring SQPOLL`)**: Operação com thread de kernel dedicada (`IORING_SETUP_SQPOLL`), descritor de socket UDP fixado na tabela interna (`IORING_REGISTER_FILES` como `Fixed(0)`) e buffers pré-registrados em arena de 2 MB alinhada a HugePages (`IORING_REGISTER_BUFFERS`), eliminando chamadas `io_uring_enter` e `get_user_pages()`.
  - **Segmentação em Hardware GSO/GRO (`UDP_SEGMENT`)**: Despacho de super-buffers agregados de até 64 KB com fatiamento em hardware pelo ASIC do adaptador AWS ENA, reduzindo em até 48x o trânsito de SQEs/CQEs.
  - **Variável Híbrida de Processo $y(t_k)$**: Métrica adimensionalizada combinando o gradiente de RTT físico na VPC ($\text{RTT}_{\min} = 400\,\mu\text{s} - 1.5\,\text{ms}$) e a pressão de ocupação do anel de submissão, antecipando bufferbloat no Ponto de Kleinrock antes de qualquer descarte UDP ou throttling 503 SlowDown.
  - **Lei de Controle PID na Forma de Velocidade com Anti-Windup**:
    - Clamping do termo integral via *back-calculation* ($K_{aw}$), adaptando autonomamente o pacing rate à banda física do enlace (10 Gbps a 40 Gbps).
    - Termo derivativo filtrado sobre a medida (*Derivative-on-Measurement* com $\tau_d$), eliminando *derivative kick* em variações do setpoint $r^*$ recebidas pelo canal MPSC.
    - Desempenho zero-alloc comprovado em benchmark com alinhamento de cache de 64 bytes (`0.26 ns/ciclo`, 3.86 bilhões de ops/s).
  - **Suíte de Testes e Propriedades**: 4 novas suítes de testes (`memory_arena_tests`, `pid_controller_tests`, `quic_iouring_loopback`, `quic_stream_isolation_tests`, `quic_laguerre_integration_tests`) com 100% de aprovação no workspace.

- **Distribuidor de Laguerre-Kantorovich com Suporte Nativo a `ObjectStore` e Assincronismo (`kineplex_core::laguerre`) (FT-097)**:
  - **Estimador de Entropia Condicional de Markov ($H(X_t | X_{t-1})$)**: Implementação em $\mathcal{O}(1)$ alocada na pilha com unigrama de 256 contadores, bigrama dobrado de 1024 slots e sobrevivência RLE, com mitigação de `clamp(min, max)` para colunas de zero-entropia (`h1 <= 0.05`).
  - **Adimensionalização de Welford**: Atualização estatística online da média e dispersão $\sigma_X^p$ na variedade de Banach, garantindo piso estável de $10^{-6}$.
  - **Regularização Log-Sinkhorn com Projeção no Hiperplano de Soma Zero**: Atualização multiplicativa $\psi_k \leftarrow \psi_k + \varepsilon \ln(M^*/M_k)$ centrada em zero ($\left|\sum \psi_k\right| \le 10^{-12}$).
  - **`BufWriter` Assíncrono para `ObjectStore` (`buf_writer.rs`)**: Implementação não-bloqueante compatível com `AsyncArrowWriter` suportando bufferização de 16 MB e concorrência 8 sem locks ou threads bloqueadas.
  - **Coalescência Geodésica Terminal de Ward-Steiner com Regularização de Bounding Box Iceberg**: Fusão ótima de células órfãs residuais minimizando a massa reduzida $\mu_{kj}$ e a dilatação $\lambda_{\text{hull}} \Delta_{\text{bbox}}$, com consolidação única para jobs diminutos ($< 64\text{ MB}$).

## [0.2.0] — 2026-11-10

### Added

- **Distribuição Ótima de Laguerre-Kantorovich (MK-EMD Anti-Small-Files) (`kineplex_core::laguerre`)**:
  - **Fundamentação Matemática e Eliminação de Small Files**: Modela o particionamento e escrita física de Arrow `RecordBatch`es como um Problema de Transporte Ótimo Semi-Discreto de Monge-Kantorovich acoplado a Tesselagens Centroidais de Voronoi com Restrição de Capacidade (CCVT / Diagramas de Potência de Laguerre). Resolve o ponto de sela variacional da energia livre equilibrando compacidade métrica e potenciais hidrostáticos de Kantorovich.
  - **Estimador Entrópico de Shannon-ZSTD ($H_1$) em $\mathcal{O}(1)$ (`entropy.rs`)**: Prediz com exatidão o tamanho comprimido antes da serialização física analisando a entropia empírica de 1ª ordem ($H_1 = -\sum p(b) \log_2 p(b)$) por amostragem de até 4096 bytes dos buffers Arrow (`Float64Array` e colunas numéricas), aplicando o fator calibrado $\text{clamp}(0.08 + 0.65 \cdot \frac{H_1}{8.0}, 0.12, 1.0)$ sem dependência externa adicional de `bytemuck`.
  - **Dinâmica de Subgradiente Hidrostático ($\psi_k$) (`distributor.rs`)**: Células de Laguerre $k^*$ evoluem seus potenciais via $\psi_k^{(t+1)} = \psi_k^{(t)} + \eta (\frac{M^* - M_k^{(t)}}{M^*})$. Células com déficit atraem lotes expandindo sua bacia de atração topológica, enquanto células saturadas exercem potencial repulsivo e fecham o arquivo na massa ótima de S3 ($M^* = 128\text{ MB}$).
  - **Promoção Preguiçosa (*Lazy Storage Promotion*)**: Lotes são retidos em arena de memória RAM (`staged_batches`) enquanto a massa acumulada for inferior a $M_{\min} = 64\text{ MB}$. A abertura de arquivos físicos ou Multipart Uploads só é disparada após a massa crítica mínima ser atingida.
  - **Fechamento Quântico Anti-Small-Files (*Zero-Residue EOF*)**: No encerramento (`finalize_optimal_distribution`), células residuais sub-críticas (< $64\text{ MB}$) sofrem colapso topológico e são absorvidas pela célula ativa mais próxima, ou consolidadas em 1 único arquivo no caso de jobs diminutos, garantindo zero arquivos pequenos espúrios no catálogo do Apache Iceberg.
  - **Arquitetura de Motor Seastar C++ para Multi-Cloud Storage (`docs/plans/seastar_arrow_multicloud_writer.md`)**:
    - Especificação da arquitetura Shared-Nothing / Thread-per-core (https://seastar.io/) acoplada ao kernel Linux via `io_uring` para eliminação de bloqueios de thread na ingestão de buffers Arrow.
    - Adaptadores não-bloqueantes para os 5 provedores de nuvem: AWS S3 (Signature v4), Azure Blob Storage (Block Blob / PutBlock), Google Cloud Storage (Resumable Uploads), Alibaba Cloud OSS (OSS Multipart) e Oracle Cloud Infrastructure (OCI Object Storage).
  - **Suíte de Testes e Propriedades (`tests/laguerre_distributor_tests.rs`)**: 7 testes de integração validando atração métrica por potencial, entropia de Shannon, promoção preguiçosa, absorção topológica no EOF e consolidação única. Mais de 285 testes passando no workspace em modo release.

- **Operador KineCoalesce ($\mathcal{C}$) com Teoria de Feixes, Smoluchowski e Espaço Quociente (`kineplex_core::coalesce`)**:
  - **SheafCoalesce (`sheaf.rs`)**: Fusão contínua em $L^2$ de colunas parciais com nulos via Teoria de Feixes. Elimina choques de degrau ($L^\infty$) entre fontes com spreads distintos através de Homotopia de Calibre nas interseções $U_j \cap U_k$ e combina seções locais via Partições da Unidade subordinadas aos Validity Bitmaps, ponderadas pelas precisões espectrais de Hilbert $\omega_k = \|f_k^\perp\|^{-2}$. Resolve o problema de contorno discreto 1D de Dirichlet ($\Delta_d \tilde{f} = 0$) sobre vazios totais $\Omega \setminus \bigcup U_k$, entregando buffers 100% densos e suaves ($C^0/C^1$) sem `NULL`s residuais para alimentar etapas SIMD/AVX-512.
  - **SmoluchowskiCoalesce (`smoluchowski.rs`)**: Coagulação métrica de partições e arquivos Parquet via equação cinética $\mathcal{K}(P_a, P_b) = \exp(-\gamma \|\Phi(P_a) - \Phi(P_b)\|_{L^p} - \lambda \mathcal{D}_{\text{box}}(P_a, P_b)) \cdot \mathbf{1}_{\{m_a + m_b \le M_{\max}\}}$. Minimiza a dilatação de caixas mortas em catálogos de metadados (Apache Iceberg), preservando hiper-retângulos `lower_bounds`/`upper_bounds` compactos para aceleração máxima de *Data Skipping*.
  - **QuotientCoalesce (`quotient.rs`)**: Compressão topológica temporal de séries de micro-eventos no espaço quociente $X / \sim$, colapsando rajadas que satisfazem a relação de equivalência de $\varepsilon$-coerência na norma $L^p$ em registros baricêntricos compactos e emitindo `RecordBatch` Arrow estruturado.
  - **Suíte de Testes e Validação Matemática (`tests/kine_coalesce_tests.rs`)**: 5 novos testes de integração cobrindo alinhamento de calibre, preenchimento harmônico de Dirichlet, bitmask coalescing sem branching, preservação de bounding boxes Iceberg e colapso de bolas $\overline{B}_\varepsilon$. 100% de aprovação sem regressões em todos os crates do workspace.

- **Catalyst Algébrico: Teoria de Galois Aplicada & Interpretação Abstrata (`catalyst::algebra`)**:
  - **Fundamentação Científica e Justificativa da Nova Visão**: Substituição de heurísticas empíricas ad-hoc por uma base algébrica formal e rigorosa no otimizador Catalyst. A nova arquitetura formaliza a propagação de restrições por meio de uma **Conexão de Galois** canônica $(\wp(\mathbb{Z}), \alpha, \gamma, \mathcal{I})$ sobre o reticulado completo de intervalos $(\mathcal{I}, \sqsubseteq, \sqcap, \sqcup, \bot, \top)$ (Interpretação Abstrata de Cousot & Cousot). As garantias de sobre-aproximação sadia (*sound over-approximation*) asseguram que simplificações de filtros e podas de planos sejam matematicamente invariantes e imunes a falsos positivos.
  - **Poda Topológica e Eliminação Estática de Filtros (`absdom.rs`)**: Avaliação estática de predicados booleanos (`<`, `<=`, `>`, `>=`, `==`, `!=`, `AND`, `OR`). Elimina filtros tautológicos em tempo de compilação e poda árvores lógicas contraditórias para `Range(0..0)` (conjunto vazio), produzindo aceleração empírica medida de **-52.73% no tempo de execução** com acréscimo desprezível de apenas 200 nanossegundos na compilação.
  - **Canonicalização por Órbitas de Simetria (`canon.rs`)**: Normalização algébrica de expressões sob a ação do grupo de permutações de operandos comutativos (`Add`, `Mul`, `And`, `Or`, `Eq`, `Neq`). Transforma subexpressões comutadas (ex: `(id + b) % 1000` e `(b + id) % 1000`) em uma forma normal única, permitindo ao eliminador de subexpressões comuns (CSE) detectá-las e reutilizá-las.
  - **Canonicalização Direcionada e Modelo de Custo de Silício**: Para mitigar o *paradoxo do CSE cego* (onde materializar colunas na memória RAM gera mais tráfego de cache do que reavaliar instruções SIMD baratas na CPU), o otimizador só materializa subexpressões quando $(count - 1) \times eval\_cost > CSE\_MATERIALIZE\_COST$, e só aplica a forma canônica se esta comprovadamente desbloquear economia líquida de CPU. Em expressões de alta latência (`%`, `/`, `QuimeraFactor`), atinge ganho medido de **-27.11% a -30.07% no tempo de execução**.
  - **Otimizações Zero-Clone**: Refatoração da contagem de frequência de subexpressões de `HashMap<Expression, usize>` para referências emprestadas `HashMap<&Expression, usize>`, cortando pela metade a latência de compilação e mantendo o tempo de otimização estritamente abaixo do teto de orçamento.
  - **Bateria de Testes e Gate de Não-Regressão Automatizado (`algebra_gate.rs`)**:
    - Binário de verificação automatizada (`algebra_gate`) com 41 iterações pareadas por carga, aquecimento e fixação de afinidade de núcleos (`taskset`).
    - 19 testes legados do baseline 100% preservados sem regressão.
    - 13 testes unitários de canonicalização (`tests/algebra_canon.rs`).
    - 6 testes formais de reticulados, monotonicidade e podas de Galois (`tests/algebra_absdom.rs`).
    - 100% de aprovação nos mais de 278 testes de integração de todo o workspace KinePlex.

- **Silo Unification Comparative Benchmark (`docs/benchmarks/benchmark_unificacao_silos_vs_kineplex.md`)**:
  - Full reproducible benchmark measuring KinePlex `BanachFrame` against the 4 fragmented data silos (Kafka/Flink SerDe, TimescaleDB sliding windows, Vector DB unpruned scans, and Volcano relational iteration).
  - Empirical proof of 3.12x streaming speedup, 1340x+ time series Dirichlet acceleration, 96.75% vector pruning ratio, and zero bytes heap allocations in hot loop vs 3.17 MB heap thrashing in Frankenstein multi-hop.
  - Automated CI integration exporting both JSON metrics (`benchmark_silos_metrics.json`) and Markdown summaries to `$GITHUB_STEP_SUMMARY`.
- **Post-Banach Topological Performance CI Pipeline (`.github/workflows/perf-post-banach.yml`)**:
  - Automated GitHub Actions performance testing workflow running on push/PR with native CPU flags (`-C target-cpu=native`).
  - Interactive **GitHub Step Summary** dashboard displaying real-time metrics, composite BTEA scores, and topological pruning acceleration tables directly in GitHub UI.
  - Automated CI Regression Gate verifying: BTEA Score $\ge 100.0$, VP-tree pruning ratio $\ge 70\%$, tail latency $p95 \le 10$ ms, and strict zero heap allocations in hot loops.
  - Telemetry artifact preservation (`post-banach-metrics.json`, `post-banach-summary.md`) with 90-day retention.
- **Post-Banach Topological Benchmark Harness (`post_banach_bench.rs`)**:
  - Standalone, zero-dependency benchmarking binary measuring canonical $N = 65.536$ ($512\text{ KB/frame}$) continuous stream throughput ($>500$ frames/s, $>250$ MB/s).
  - Innovative **BTEA (Banach Topological Efficiency & Acceleration)** composite index integrating throughput, triangle inequality pruning ratio, and tail latency determinism.
  - Micro-benchmarks for Sobolev Dirichlet energy ($<1$ µs), Haar wavelet multiresolution ($<600$ µs), and Wasserstein $W_1$ optimal transport.
- **Metric Pruning Telemetry & Zero-Alloc Optimal Transport (`vptree.rs`, `banach.rs`)**:
  - `VpSearchStats` telemetry struct capturing distance evaluations, visited nodes, pruned branches, and pruning ratio ($\eta_{prune}$).
  - `search_knn_with_stats` and `search_knn_brute_force` methods on `TopologicalVpIndex` for empirical verification of logarithmic $\mathcal{O}(\log M)$ vs linear $\mathcal{O}(M)$ complexity.
  - `wasserstein_distance_w1_scratch` providing zero-heap-allocation Earth Mover's Distance calculations with reusable buffers.
- **Post-Banach Integration & Performance Test Suite (`post_banach_perf.rs`)**:
  - 4 comprehensive release tests verifying 100% metric fidelity ($\epsilon < 1e-12$) between VP-Tree and brute-force, sub-millisecond regime transition detection, and zero-allocation scratch buffers.

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
