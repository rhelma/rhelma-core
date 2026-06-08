use metrics::{counter, histogram};
use std::time::Instant;
#[cfg(feature = "embeddings-torch")]
use tch::Device;

#[cfg(not(feature = "embeddings-torch"))]
#[derive(Debug, Clone, Copy)]
pub enum Device {
    /// Variant `Cpu`.
    Cpu,
}

use thiserror::Error;
use tracing::info;

/// Embedding engine abstraction.
///
/// In a real deployment this would load a transformer model and run it on
/// CPU/GPU/MPS. Here we keep it minimal but metrics-friendly.
pub struct EmbeddingEngine {
    model_name: String,
    device: Device,
}

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("failed to initialize embedding model: {0}")]
    /// Variant `Init`.
    Init(String),

    #[error("failed to compute embeddings: {0}")]
    /// Variant `Inference`.
    Inference(String),
}

impl EmbeddingEngine {
    pub async fn new(model_name: String) -> Result<Self, EmbeddingError> {
        // Very naive device selection; in real code you would query CUDA/MPS.
        #[cfg(feature = "embeddings-torch")]
        let device = if tch::Cuda::is_available() {
            Device::Cuda(0)
        } else {
            Device::Cpu
        };

        #[cfg(not(feature = "embeddings-torch"))]
        let device = Device::Cpu;

        info!(%model_name, ?device, "initializing embedding engine");

        // Real model loading omitted; we just simulate.
        Ok(Self { model_name, device })
    }

    /// Compute embeddings for the given list of texts.
    ///
    /// This stub returns random vectors but wires metrics and tracing in a
    /// realistic way.
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let start = Instant::now();
        let batch_size = texts.len() as u64;

        // In real implementation, run model here.
        let dim = 384;
        let mut out = Vec::with_capacity(texts.len());
        for _ in texts {
            out.push(vec![0.0_f32; dim]);
        }

        let elapsed = start.elapsed();
        histogram!(
            "search_embedding_duration_seconds",
            "model" => self.model_name.clone()
        )
        .record(elapsed.as_secs_f64());

        counter!(
            "search_embedding_requests_total",
            "model" => self.model_name.clone()
        )
        .increment(batch_size);

        Ok(out)
    }
}
