use anyhow::Result;
use langchain_rust::schemas::Document;
use langchain_rust::embedding::Embedder;
use crate::rag::langchain_embedding::CandleEmbedder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use std::collections::HashMap;
use tokio::sync::Mutex;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::io::{Read, Write};

pub struct KnowledgeStore<E: Embedder + Send + Sync + 'static> {
    // We use Arc/Mutex for shared state across threads
    documents: Arc<Mutex<Vec<Document>>>,
    embeddings: Arc<Mutex<Vec<Vec<f64>>>>,
    embedder: Arc<E>,
    path: PathBuf,
}

impl KnowledgeStore<CandleEmbedder> {
    pub async fn new(app_data: &str) -> Result<Self> {
        let embedder = CandleEmbedder::new()?;
        Self::new_with_embedder(app_data, embedder).await
    }
}

impl<E: Embedder + Send + Sync + 'static> KnowledgeStore<E> {
    pub async fn new_with_embedder(app_data: &str, embedder: E) -> Result<Self> {
        // Use .gz extension for compressed storage
        let db_path = std::path::Path::new(app_data).join("air").join("knowledge.json.gz");

        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let store = Self {
            documents: Arc::new(Mutex::new(Vec::new())),
            embeddings: Arc::new(Mutex::new(Vec::new())),
            embedder: Arc::new(embedder),
            path: db_path.clone(),
        };

        if db_path.exists() {
            // Read compressed file
            let compressed_data = fs::read(&db_path).await?;
            if !compressed_data.is_empty() {
                // Decompress
                let mut d = GzDecoder::new(&compressed_data[..]);
                let mut s = String::new();
                d.read_to_string(&mut s)?;

                if !s.is_empty() {
                    let serialized: Vec<SerializedDocument> = serde_json::from_str(&s).unwrap_or_default();
                    if !serialized.is_empty() {
                        let mut docs = store.documents.lock().await;
                        let mut embs = store.embeddings.lock().await;

                        let mut new_docs = Vec::new();
                        let mut new_texts = Vec::new();

                        for s in serialized {
                            let doc: Document = s.into();
                            new_docs.push(doc.clone());
                            new_texts.push(doc.page_content);
                        }

                        // Re-embed on load (since we don't save embeddings to keep file small)
                        let embeddings_result = store.embedder.embed_documents(&new_texts).await.map_err(|e| anyhow::anyhow!("Embedding failed: {:?}", e))?;

                        *docs = new_docs;
                        *embs = embeddings_result;
                    }
                }
            }
        }

        Ok(store)
    }

    pub async fn add_text(&self, content: &str, metadata: serde_json::Value) -> Result<()> {
        let mut meta_map: HashMap<String, serde_json::Value> = HashMap::new();
        if let serde_json::Value::Object(map) = metadata {
            for (k, v) in map {
                meta_map.insert(k, v);
            }
        }

        let doc = Document::new(content.to_string()).with_metadata(meta_map);

        let embedding = self.embedder.embed_query(&content).await.map_err(|e| anyhow::anyhow!("Embedding failed: {:?}", e))?;

        {
            let mut docs = self.documents.lock().await;
            let mut embs = self.embeddings.lock().await;
            docs.push(doc);
            embs.push(embedding);
        }

        self.save().await?;
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f64)>> {
        let query_embedding = self.embedder.embed_query(query).await.map_err(|e| anyhow::anyhow!("Embedding failed: {:?}", e))?;

        let docs = self.documents.lock().await;
        let embs = self.embeddings.lock().await;

        let mut scores: Vec<(usize, f64)> = embs.iter().enumerate()
            .map(|(i, emb)| {
                let score = cosine_similarity(&query_embedding, emb);
                (i, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let results = scores.into_iter()
            .take(limit)
            .map(|(i, score)| (docs[i].clone(), score))
            .collect();

        Ok(results)
    }

    async fn save(&self) -> Result<()> {
        let docs = self.documents.lock().await;
        let serialized_docs: Vec<SerializedDocument> = docs.iter().map(|d| SerializedDocument::from(d.clone())).collect();
        let content = serde_json::to_string(&serialized_docs)?;

        // Compress data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content.as_bytes())?;
        let compressed_data = encoder.finish()?;

        fs::write(&self.path, compressed_data).await?;
        Ok(())
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot_product: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedDocument {
    page_content: String,
    metadata: HashMap<String, serde_json::Value>,
}

impl From<Document> for SerializedDocument {
    fn from(doc: Document) -> Self {
        Self {
            page_content: doc.page_content,
            metadata: doc.metadata,
        }
    }
}

impl From<SerializedDocument> for Document {
    fn from(val: SerializedDocument) -> Self {
        Document::new(val.page_content).with_metadata(val.metadata)
    }
}
