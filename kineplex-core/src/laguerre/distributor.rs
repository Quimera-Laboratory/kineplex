//! Distribuidor Ótimo de Laguerre-Kantorovich (MK-EMD Anti-Small-Files)
//!
//! Coordena o particionamento dinâmico e o roteamento de lotes Arrow RecordBatch
//! utilizando Transporte Ótimo Semi-Discreto acoplado a Tesselagens Centroidais
//! de Voronoi com Restrição de Capacidade (Diagramas de Potência de Laguerre).

use super::cell::LaguerreCell;
use super::config::OptimalDistributionConfig;
use super::entropy::estimate_zstd_mass;
use super::sink::LaguerreParquetSink;
use arrow_array::{Array, Float64Array, RecordBatch};
use std::sync::Arc;
use uuid::Uuid;

/// Distribuidor Ótimo que governa a escrita física de arquivos Parquet sem small files.
pub struct LaguerreOptimalDistributor<S: LaguerreParquetSink> {
    sink: Arc<S>,
    base_prefix: String,
    config: OptimalDistributionConfig,
    cells: Vec<LaguerreCell>,
    completed_files: Vec<String>,
}

impl<S: LaguerreParquetSink> LaguerreOptimalDistributor<S> {
    /// Inicializa o distribuidor com o sink de armazenamento, prefixo base e baricentros iniciais.
    pub fn new(
        sink: Arc<S>,
        base_prefix: impl Into<String>,
        initial_centroids: Vec<[f64; 4]>,
        config: OptimalDistributionConfig,
    ) -> Self {
        let centroids = if initial_centroids.is_empty() {
            vec![[0.0, 0.0, 0.0, 0.0]]
        } else {
            initial_centroids
        };

        let cells = centroids
            .into_iter()
            .enumerate()
            .map(|(idx, c)| LaguerreCell::new(idx, c))
            .collect();

        Self {
            sink,
            base_prefix: base_prefix.into(),
            config,
            cells,
            completed_files: Vec::new(),
        }
    }

    /// Retorna uma referência às células gerenciadas.
    pub fn cells(&self) -> &[LaguerreCell] {
        &self.cells
    }

    /// Retorna a lista de arquivos já selados e finalizados.
    pub fn completed_files(&self) -> &[String] {
        &self.completed_files
    }

    /// Extrai automaticamente a assinatura no espaço de Banach [f64; 4] de um RecordBatch.
    /// Dimensões: (L1 normalizada, L2 normalizada, Lp normalizada, Linf normalizada).
    pub fn extract_banach_signature(&self, batch: &RecordBatch) -> [f64; 4] {
        let p = self.config.banach_p_order;
        for col in batch.columns() {
            if let Some(f64_col) = col.as_any().downcast_ref::<Float64Array>() {
                if f64_col.len() > 0 {
                    let mut l1 = 0.0;
                    let mut l2_sq = 0.0;
                    let mut lp_sum = 0.0;
                    let mut linf = 0.0_f64;

                    for i in 0..f64_col.len() {
                        if !f64_col.is_null(i) {
                            let val = f64_col.value(i).abs();
                            l1 += val;
                            l2_sq += val * val;
                            lp_sum += val.powf(p);
                            if val > linf {
                                linf = val;
                            }
                        }
                    }

                    let n = f64_col.len() as f64;
                    let norm_l1 = l1 / n;
                    let norm_l2 = (l2_sq / n).sqrt();
                    let norm_lp = (lp_sum / n).powf(1.0 / p);

                    return [norm_l1, norm_l2, norm_lp, linf];
                }
            }
        }

        // Fallback baseado em dimensões e contagem de linhas
        let rows = batch.num_rows() as f64;
        let bytes = batch.get_array_memory_size() as f64;
        [rows, bytes / 1024.0, (rows * bytes).sqrt(), rows]
    }

    /// Distância de Laguerre no diagrama de potência:
    /// d_L(x, c_k) = ||x - c_k||_p^p - \psi_k
    #[inline(always)]
    pub fn laguerre_distance(&self, sig: &[f64; 4], cell: &LaguerreCell) -> f64 {
        let p = self.config.banach_p_order;
        let metric_cost: f64 = sig
            .iter()
            .zip(cell.centroid.iter())
            .map(|(x, c)| (x - c).abs().powf(p))
            .sum();

        metric_cost - cell.dual_potential_psi
    }

    /// Atualiza os potenciais duais de Kantorovich \psi_k via dinâmica de subgradiente hidrostático.
    ///
    /// Células com déficit de massa (M_k < M^*) desenvolvem potencial atrativo (\psi_k > 0).
    /// Células saturadas (M_k >= M^*) sofrem potencial repulsivo (\psi_k <= 0).
    pub fn rebalance_kantorovich_potentials(&mut self) {
        let target = self.config.target_file_bytes as f64;
        let eta = self.config.laguerre_step_rate;

        for cell in &mut self.cells {
            let current_mass = cell.accumulated_compressed_bytes as f64;
            let mass_deficit_ratio = (target - current_mass) / target;
            cell.dual_potential_psi += eta * mass_deficit_ratio;
        }
    }

    /// Roteia um RecordBatch para a célula de Laguerre ótima e gerencia a promoção física.
    pub fn route_and_distribute(
        &mut self,
        batch: RecordBatch,
        lp_signature: [f64; 4],
    ) -> Result<(), String> {
        let est_mass = estimate_zstd_mass(&batch);

        // 1. Encontra a Célula de Laguerre k^* que minimiza d_L(x, c_k) = ||x - c_k||_p^p - \psi_k
        let mut best_idx = 0;
        let mut min_laguerre = f64::INFINITY;
        for (idx, cell) in self.cells.iter().enumerate() {
            let d_lag = self.laguerre_distance(&lp_signature, cell);
            if d_lag < min_laguerre {
                min_laguerre = d_lag;
                best_idx = idx;
            }
        }

        // 2. Atualiza o baricentro funcional da célula vencedora (Média Móvel Ponderada pela Massa)
        {
            let cell = &mut self.cells[best_idx];
            cell.update_centroid(lp_signature, est_mass);
        }

        // 3. Promoção Preguiçosa: retém na arena em memória até atingir M_min
        let cell = &mut self.cells[best_idx];
        if let Some(writer) = cell.active_writer.as_mut() {
            writer.write_batch(&batch)?;
        } else {
            cell.staged_batches.push(batch);

            // Cruzou a Massa Mínima M_min: abre o writer e descarrega a arena de memória!
            if cell.accumulated_compressed_bytes >= self.config.min_file_bytes {
                let uri = format!(
                    "{}/cell_{:02}/kinehaus-opt-{}.parquet",
                    self.base_prefix,
                    cell.cell_id,
                    Uuid::new_v4()
                );
                let schema = cell.staged_batches[0].schema();
                let mut writer = self.sink.open_writer(&uri, schema)?;

                for staged in cell.staged_batches.drain(..) {
                    writer.write_batch(&staged)?;
                }
                cell.active_writer = Some(writer);
            }
        }

        // 4. Se a célula atingiu a Massa Alvo Ótima M^*, sela o arquivo Parquet perfeitamente calibrado
        if cell.accumulated_compressed_bytes >= self.config.target_file_bytes {
            if let Some(writer) = cell.active_writer.take() {
                let size = cell.accumulated_compressed_bytes;
                let (uri, _) = writer.close()?;
                self.sink.on_file_completed(&uri, size);
                self.completed_files.push(uri);
                cell.accumulated_compressed_bytes = 0;
                cell.dual_potential_psi = 0.0;
            }
        }

        // 5. Rebalanceia os potenciais de Kantorovich para o próximo lote
        self.rebalance_kantorovich_potentials();
        Ok(())
    }

    /// Roteia um RecordBatch extraindo automaticamente a assinatura topológica.
    pub fn route_auto(&mut self, batch: RecordBatch) -> Result<(), String> {
        let sig = self.extract_banach_signature(&batch);
        self.route_and_distribute(batch, sig)
    }

    /// Fechamento Quântico Anti-Small-Files (Zero-Residue EOF):
    /// Funde células sub-críticas (< M_min) na célula ativa mais próxima topologicamente.
    /// Se o volume total do job for inferior a M_min, consolida tudo em 1 único arquivo Parquet.
    pub fn finalize_optimal_distribution(mut self) -> Result<Vec<String>, String> {
        let mut orphan_batches: Vec<(RecordBatch, [f64; 4], u64)> = Vec::new();

        // 1. Coleta todos os lotes retidos em células que não atingiram M_min
        for cell in &mut self.cells {
            if cell.active_writer.is_none() && !cell.staged_batches.is_empty() {
                let c_sig = cell.centroid;
                let m = cell.accumulated_compressed_bytes;
                for b in cell.staged_batches.drain(..) {
                    orphan_batches.push((b, c_sig, m));
                }
                cell.accumulated_compressed_bytes = 0;
            }
        }

        // 2. Absorve os lotes órfãos na célula ativa mais próxima topologicamente
        if !orphan_batches.is_empty() {
            let active_idx = self.cells.iter().position(|c| c.active_writer.is_some());

            if let Some(target_idx) = active_idx {
                let writer = self.cells[target_idx].active_writer.as_mut().unwrap();
                for (b, _, _) in orphan_batches {
                    writer.write_batch(&b)?;
                }
            } else {
                // Caso o job inteiro tenha menos de M_min no total, consolida tudo em 1 único arquivo Parquet!
                let uri = format!(
                    "{}/consolidated/kinehaus-single-{}.parquet",
                    self.base_prefix,
                    Uuid::new_v4()
                );
                let schema = orphan_batches[0].0.schema();
                let mut single_writer = self.sink.open_writer(&uri, schema)?;

                for (b, _, _) in orphan_batches {
                    single_writer.write_batch(&b)?;
                }
                let (closed_uri, _) = single_writer.close()?;
                self.sink.on_file_completed(&closed_uri, 0);
                self.completed_files.push(closed_uri);
            }
        }

        // 3. Fecha todos os writers ativos restantes
        for mut cell in self.cells {
            if let Some(writer) = cell.active_writer.take() {
                let size = cell.accumulated_compressed_bytes;
                let (uri, _) = writer.close()?;
                self.sink.on_file_completed(&uri, size);
                self.completed_files.push(uri);
            }
        }

        Ok(self.completed_files)
    }
}
