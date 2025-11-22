use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use crate::rag::embeddings::EmbeddingModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct VectorStore {
    documents: Vec<Document>,
    #[serde(skip)]
    embedding_model: Option<EmbeddingModel>,
    path: PathBuf,
}

impl VectorStore {
    pub fn new() -> Result<Self> {
        let mut path = std::env::current_dir()?;
        path.push(".air");
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        path.push("knowledge.json");

        let mut store = if path.exists() {
            let content = fs::read_to_string(&path)?;
            let mut store: VectorStore = serde_json::from_str(&content)?;
            store.path = path;
            store
        } else {
            Self {
                documents: Vec::new(),
                embedding_model: None,
                path,
            }
        };

        // Lazily load model only when needed? No, let's try to load it if we can,
        // but maybe we should let the caller handle model loading to avoid heavy startup.
        // For now, we'll leave it None and init on first use.

        Ok(store)
    }

    fn ensure_model(&mut self) -> Result<()> {
        if self.embedding_model.is_none() {
            self.embedding_model = Some(EmbeddingModel::new()?);
        }
        Ok(())
    }

    pub fn add_text(&mut self, content: &str, metadata: serde_json::Value) -> Result<()> {
        self.ensure_model()?;
        let model = self.embedding_model.as_mut().unwrap();

        let embedding = model.embed(content)?;

        let doc = Document {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            metadata,
            embedding,
        };

        self.documents.push(doc);
        self.save()?;

        Ok(())
    }

    pub fn search(&mut self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>> {
        self.ensure_model()?;
        let model = self.embedding_model.as_mut().unwrap();

        let query_embedding = model.embed(query)?;

        let mut scores: Vec<(usize, f32)> = self.documents.iter().enumerate()
            .map(|(i, doc)| {
                let score = cosine_similarity(&query_embedding, &doc.embedding);
                (i, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scores.into_iter()
            .take(limit)
            .map(|(i, score)| (self.documents[i].clone(), score))
            .collect())
    }

    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
