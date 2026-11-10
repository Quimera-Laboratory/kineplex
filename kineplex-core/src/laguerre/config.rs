//! Parâmetros de Configuração da Distribuição Ótima de Laguerre-Kantorovich (MK-EMD)

/// Parâmetros da Lei de Distribuição Ótima de Monge-Kantorovich (MK-EMD Anti-Small-Files).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptimalDistributionConfig {
    /// Massa Crítica Ótima de S3 / Object Storage: M^* = 128 MB (ponto ótimo para Range-GET e RowGroups Parquet).
    pub target_file_bytes: u64,
    /// Limiar Mínimo de Abertura: M_min = 64 MB (abaixo disso é proibido abrir/fechar arquivo isolado).
    pub min_file_bytes: u64,
    /// Teto Máximo de Corte: M_max = 256 MB (teto para flush imediato do Multipart Upload).
    pub max_file_bytes: u64,
    /// Ordem p da métrica L^p para coesão topológica e Data Skipping.
    pub banach_p_order: f64,
    /// Taxa de aprendizado \eta do potencial dual hidrostático de Kantorovich \psi_k.
    pub laguerre_step_rate: f64,
}

impl Default for OptimalDistributionConfig {
    fn default() -> Self {
        Self {
            target_file_bytes: 128 * 1024 * 1024, // 128 MB
            min_file_bytes: 64 * 1024 * 1024,     // 64 MB
            max_file_bytes: 256 * 1024 * 1024,    // 256 MB
            banach_p_order: 2.0,
            laguerre_step_rate: 15.0,
        }
    }
}

impl OptimalDistributionConfig {
    /// Construtor de teste com escala reduzida para validação instantânea em memória.
    pub fn test_scale(min_bytes: u64, target_bytes: u64, max_bytes: u64) -> Self {
        Self {
            target_file_bytes: target_bytes,
            min_file_bytes: min_bytes,
            max_file_bytes: max_bytes,
            banach_p_order: 2.0,
            laguerre_step_rate: 15.0,
        }
    }
}
