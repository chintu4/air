use anyhow::Result;
use async_trait::async_trait;
use langchain_rust::embedding::{Embedder, EmbedderError};
use crate::rag::embeddings::EmbeddingModel;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CandleEmbedder {
    inner: Arc<Mutex<EmbeddingModel>>,
}

impl CandleEmbedder {
    pub fn new() -> Result<Self> {
        let model = EmbeddingModel::new()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(model)),
        })
    }
}

#[async_trait]
impl Embedder for CandleEmbedder {
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError> {
        let mut results = Vec::new();

        // Trying to use a generic error if possible, or mapping to a known one.
        // Since we don't know variants, let's try to see if From<String> works or similar.
        // or just use a dummy error like fastembed if available.
        let mut model = self.inner.lock().map_err(|e| EmbedderError::FastEmbedError(e.to_string().into()))?;

        for doc in documents {
            let embedding_f32 = model.embed(doc).map_err(|e| EmbedderError::FastEmbedError(e.to_string().into()))?;
            let embedding_f64: Vec<f64> = embedding_f32.into_iter().map(|x| x as f64).collect();
            results.push(embedding_f64);
        }

        Ok(results)
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError> {
        let mut model = self.inner.lock().map_err(|e| EmbedderError::FastEmbedError(e.to_string().into()))?;
        let embedding_f32 = model.embed(text).map_err(|e| EmbedderError::FastEmbedError(e.to_string().into()))?;
        Ok(embedding_f32.into_iter().map(|x| x as f64).collect())
    }
}
