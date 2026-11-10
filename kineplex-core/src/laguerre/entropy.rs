//! Estimador Entrópico de Shannon-ZSTD (H_1) em O(1)
//!
//! Prediz o tamanho físico comprimido de RecordBatches do Apache Arrow antes de
//! despender ciclos de CPU em compressão real, medindo a entropia empírica de 1ª ordem.

use arrow_array::{Array, Float64Array, RecordBatch};

/// Calcula a entropia empírica de Shannon de 1ª ordem (bits por byte) amostrada em O(1).
///
/// H_1 = - \sum_{b=0}^{255} p(b) \log_2 p(b)
pub fn compute_shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }

    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }

    let n = bytes.len() as f64;
    let inv_n = 1.0 / n;
    let mut entropy = 0.0_f64;

    for &c in &counts {
        if c > 0 {
            let p = (c as f64) * inv_n;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Estima a massa comprimida ZSTD via Entropia de Shannon de 1ª ordem amostrada em O(1).
///
/// \widehat{m}_{\text{ZSTD}}(\text{Batch}) = \text{Bytes}_{\text{Arrow}} \cdot \text{clamp}\left(0.08 + 0.65 \cdot \frac{H_1}{8.0}, \; 0.12, \; 1.0\right)
pub fn estimate_zstd_mass(batch: &RecordBatch) -> u64 {
    let raw_bytes = batch.get_array_memory_size();
    if raw_bytes == 0 || batch.num_rows() == 0 {
        return 0;
    }

    // Amostra até 4096 bytes dos buffers do lote
    let mut sample_bytes = Vec::with_capacity(4096);

    // Prioriza coluna Float64Array se disponível
    let mut sampled = false;
    for col in batch.columns() {
        if let Some(f64_col) = col.as_any().downcast_ref::<Float64Array>() {
            let buffers = f64_col.to_data().buffers().to_vec();
            if let Some(buf) = buffers.first() {
                let slice = buf.as_slice();
                let take = slice.len().min(4096);
                sample_bytes.extend_from_slice(&slice[..take]);
                sampled = true;
                break;
            }
        }
    }

    // Se não encontrou Float64Array, extrai buffers de qualquer coluna
    if !sampled {
        for col in batch.columns() {
            let data = col.to_data();
            for buf in data.buffers() {
                let slice = buf.as_slice();
                let remaining = 4096 - sample_bytes.len();
                let take = slice.len().min(remaining);
                sample_bytes.extend_from_slice(&slice[..take]);
                if sample_bytes.len() >= 4096 {
                    break;
                }
            }
            if sample_bytes.len() >= 4096 {
                break;
            }
        }
    }

    if sample_bytes.is_empty() {
        return (raw_bytes as u64) / 3;
    }

    let h1 = compute_shannon_entropy(&sample_bytes);
    // Fator de compressão ZSTD calibrado pela entropia por byte (0.12 a 1.0)
    let compression_ratio = (0.08 + (h1 / 8.0) * 0.65).clamp(0.12, 1.0);
    ((raw_bytes as f64) * compression_ratio) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow_array::{ArrayRef, Float64Array};
    use std::sync::Arc;

    #[test]
    fn test_constant_bytes_zero_entropy() {
        let zeros = vec![0u8; 1024];
        let h = compute_shannon_entropy(&zeros);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_uniform_random_bytes_high_entropy() {
        let mut uniform = Vec::with_capacity(256 * 10);
        for _ in 0..10 {
            for b in 0..=255u8 {
                uniform.push(b);
            }
        }
        let h = compute_shannon_entropy(&uniform);
        assert!((h - 8.0).abs() < 1e-4);
    }

    #[test]
    fn test_batch_entropy_estimation() {
        let f_col = Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.0, 5.0])) as ArrayRef;
        let schema = Arc::new(Schema::new(vec![Field::new("val", DataType::Float64, false)]));
        let batch = RecordBatch::try_new(schema, vec![f_col]).unwrap();

        let est = estimate_zstd_mass(&batch);
        assert!(est > 0);
        assert!(est <= batch.get_array_memory_size() as u64);
    }
}
