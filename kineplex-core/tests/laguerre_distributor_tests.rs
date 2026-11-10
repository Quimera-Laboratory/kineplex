//! Testes da Distribuição Ótima de Laguerre-Kantorovich (MK-EMD Anti-Small-Files)

use arrow::datatypes::{DataType, Field, Schema};
use arrow_array::{ArrayRef, Float64Array, RecordBatch};
use kineplex_core::laguerre::{
    compute_shannon_entropy, estimate_zstd_mass, InMemoryParquetSink, LaguerreOptimalDistributor,
    OptimalDistributionConfig,
};
use std::sync::Arc;

fn make_test_batch(values: Vec<f64>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![Field::new("metric", DataType::Float64, false)]));
    let array = Arc::new(Float64Array::from(values)) as ArrayRef;
    RecordBatch::try_new(schema, vec![array]).expect("Falha ao criar RecordBatch")
}

#[test]
fn test_shannon_entropy_properties() {
    let constant_bytes = vec![42u8; 2048];
    let h_low = compute_shannon_entropy(&constant_bytes);
    assert_eq!(h_low, 0.0, "Entropia de bytes constantes deve ser zero");

    let mut uniform_bytes = Vec::with_capacity(256 * 16);
    for _ in 0..16 {
        for b in 0..=255u8 {
            uniform_bytes.push(b);
        }
    }
    let h_high = compute_shannon_entropy(&uniform_bytes);
    assert!((h_high - 8.0).abs() < 1e-4, "Entropia uniforme de 256 bytes deve ser 8.0");
}

#[test]
fn test_zstd_mass_estimation() {
    let batch = make_test_batch(vec![1.0; 1000]);
    let est_mass = estimate_zstd_mass(&batch);
    let raw = batch.get_array_memory_size() as u64;

    assert!(est_mass > 0);
    assert!(est_mass <= raw, "Massa estimada comprimida deve ser <= tamanho bruto");
}

#[test]
fn test_laguerre_power_diagram_attraction_by_potential() {
    let sink = Arc::new(InMemoryParquetSink::new());
    let config = OptimalDistributionConfig::test_scale(10_000, 50_000, 100_000);

    // Duas células: Célula 0 em [0, 0, 0, 0] e Célula 1 em [10, 10, 10, 10]
    let centroids = vec![[0.0, 0.0, 0.0, 0.0], [10.0, 10.0, 10.0, 10.0]];
    let mut distributor = LaguerreOptimalDistributor::new(
        sink,
        "lakehouse/tests",
        centroids,
        config,
    );

    // Ponto x mais próximo da Célula 1 metricamente: [8, 8, 8, 8]
    let sig = [8.0, 8.0, 8.0, 8.0];
    let d0_initial = distributor.laguerre_distance(&sig, &distributor.cells()[0]);
    let d1_initial = distributor.laguerre_distance(&sig, &distributor.cells()[1]);
    assert!(d1_initial < d0_initial, "Célula 1 deve ser mais próxima inicialmente");

    // Agora forçamos um enorme déficit na Célula 0 (psi_0 muito positivo)
    distributor.rebalance_kantorovich_potentials();
    // Injetamos potencial de atração na Célula 0
    let batch = make_test_batch(vec![0.5; 50]);
    distributor.route_and_distribute(batch, [0.0, 0.0, 0.0, 0.0]).unwrap();
}

#[test]
fn test_lazy_promotion_and_arena_staged() {
    let sink = Arc::new(InMemoryParquetSink::new());
    // M_min = 20_000 bytes, M* = 100_000 bytes
    let config = OptimalDistributionConfig::test_scale(20_000, 100_000, 200_000);

    let mut distributor = LaguerreOptimalDistributor::new(
        sink,
        "lakehouse/tests",
        vec![[0.0, 0.0, 0.0, 0.0]],
        config,
    );

    // Lote pequeno: tamanho estimado < M_min
    let small_batch = make_test_batch(vec![1.23; 50]);
    distributor.route_auto(small_batch).unwrap();

    // Deve estar na arena staged e NÃO deve ter aberto writer
    assert_eq!(distributor.cells()[0].staged_count(), 1);
    assert!(!distributor.cells()[0].is_promoted());
    assert_eq!(distributor.completed_files().len(), 0);
}

#[test]
fn test_zero_residue_quantum_eof_consolidation() {
    let sink = Arc::new(InMemoryParquetSink::new());
    // M_min = 50_000 bytes. Enviamos apenas ~1.000 bytes totais em 3 células.
    let config = OptimalDistributionConfig::test_scale(50_000, 100_000, 200_000);

    let centroids = vec![
        [1.0, 1.0, 1.0, 1.0],
        [5.0, 5.0, 5.0, 5.0],
        [9.0, 9.0, 9.0, 9.0],
    ];
    let mut distributor = LaguerreOptimalDistributor::new(
        sink.clone(),
        "lakehouse/zero_residue",
        centroids,
        config,
    );

    distributor.route_auto(make_test_batch(vec![1.0; 20])).unwrap();
    distributor.route_auto(make_test_batch(vec![5.0; 20])).unwrap();
    distributor.route_auto(make_test_batch(vec![9.0; 20])).unwrap();

    // Nenhuma célula atingiu M_min, logo todas retêm lotes na arena
    for cell in distributor.cells() {
        assert!(!cell.is_promoted());
    }

    // Fechamento Quântico: Consolida todos os resíduos em 1 único arquivo!
    let files = distributor.finalize_optimal_distribution().unwrap();
    assert_eq!(files.len(), 1, "Deve consolidar em exatamente 1 único arquivo Parquet!");
    assert!(files[0].contains("consolidated/kinehaus-single-"));
}

#[test]
fn test_zero_residue_quantum_eof_absorption_into_active_cell() {
    let sink = Arc::new(InMemoryParquetSink::new());
    // M_min = 1_000 bytes, M* = 500_000 bytes
    let config = OptimalDistributionConfig::test_scale(1_000, 500_000, 1_000_000);

    let centroids = vec![[0.0, 0.0, 0.0, 0.0], [100.0, 100.0, 100.0, 100.0]];
    let mut distributor = LaguerreOptimalDistributor::new(
        sink,
        "lakehouse/absorption",
        centroids,
        config,
    );

    // Alimenta a Célula 0 com dados suficientes para promover (passar de 1_000 bytes)
    for _ in 0..15 {
        distributor.route_and_distribute(
            make_test_batch(vec![0.1; 100]),
            [0.0, 0.0, 0.0, 0.0],
        ).unwrap();
    }
    assert!(distributor.cells()[0].is_promoted(), "Célula 0 deve ter sido promovida");

    // Envia um lote isolado na Célula 1 (< 1_000 bytes, não promovida)
    distributor.route_and_distribute(
        make_test_batch(vec![99.9; 10]),
        [100.0, 100.0, 100.0, 100.0],
    ).unwrap();
    assert!(!distributor.cells()[1].is_promoted(), "Célula 1 não deve estar promovida");

    // Fechamento Quântico: A Célula 1 é absorvida pela Célula 0 ativa!
    let files = distributor.finalize_optimal_distribution().unwrap();
    // Apenas 1 arquivo deve ter sido gerado (o da Célula 0 que absorveu os órfãos)
    assert_eq!(files.len(), 1);
    assert!(files[0].contains("cell_00"));
}

#[test]
fn test_banach_signature_extraction() {
    let sink = Arc::new(InMemoryParquetSink::new());
    let config = OptimalDistributionConfig::default();
    let distributor = LaguerreOptimalDistributor::new(
        sink,
        "lakehouse/sig",
        vec![[0.0, 0.0, 0.0, 0.0]],
        config,
    );

    let batch = make_test_batch(vec![2.0, -4.0, 6.0, -8.0]);
    let sig = distributor.extract_banach_signature(&batch);

    // L1 normalizada = (2 + 4 + 6 + 8) / 4 = 5.0
    assert!((sig[0] - 5.0).abs() < 1e-4);
    // Linf = 8.0
    assert!((sig[3] - 8.0).abs() < 1e-4);
    assert!(sig[1] > 0.0);
    assert!(sig[2] > 0.0);
}
