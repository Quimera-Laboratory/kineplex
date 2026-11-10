//! Célula de Laguerre-Voronoi (CCVT com Restrição de Capacidade)
//!
//! Representa uma célula ativa no diagrama de potência de Laguerre em memória
//! antes de se tornar um arquivo Parquet no Object Storage / S3 / Iceberg.

use super::sink::ActiveWriterHandle;
use arrow_array::RecordBatch;

/// Representa uma Célula de Laguerre-Voronoi ativa em memória.
pub struct LaguerreCell {
    /// Identificador numérico da célula.
    pub cell_id: usize,
    /// Baricentro funcional c_k \in R^4 (ex: L1, L2, Lp, Linf ou estatísticas geométricas).
    pub centroid: [f64; 4],
    /// Potencial dual de Kantorovich \psi_k (atrai massa se M_k < M^*, repele se M_k >= M^*).
    pub dual_potential_psi: f64,
    /// Massa acumulada em bytes comprimidos estimados na célula.
    pub accumulated_compressed_bytes: u64,
    /// Arena em memória de lotes retidos até atingir M_min (evita a criação de Small Files!).
    pub staged_batches: Vec<RecordBatch>,
    /// Writer Parquet ativo associado à célula após a promoção preguiçosa.
    pub active_writer: Option<ActiveWriterHandle>,
}

impl LaguerreCell {
    /// Cria uma nova célula com baricentro inicial.
    pub fn new(cell_id: usize, centroid: [f64; 4]) -> Self {
        Self {
            cell_id,
            centroid,
            dual_potential_psi: 0.0,
            accumulated_compressed_bytes: 0,
            staged_batches: Vec::with_capacity(32),
            active_writer: None,
        }
    }

    /// Atualiza o baricentro funcional c_k da célula como média móvel ponderada pela massa.
    pub fn update_centroid(&mut self, batch_signature: [f64; 4], batch_mass: u64) {
        let old_m = self.accumulated_compressed_bytes as f64;
        let new_m = batch_mass as f64;
        let tot_m = (old_m + new_m).max(1.0);

        for d in 0..4 {
            self.centroid[d] = (old_m * self.centroid[d] + new_m * batch_signature[d]) / tot_m;
        }
        self.accumulated_compressed_bytes += batch_mass;
    }

    /// Retorna verdadeiro se a célula possui um writer físico aberto.
    #[inline(always)]
    pub fn is_promoted(&self) -> bool {
        self.active_writer.is_some()
    }

    /// Retorna a quantidade de lotes atualmente retidos na arena.
    #[inline(always)]
    pub fn staged_count(&self) -> usize {
        self.staged_batches.len()
    }
}
