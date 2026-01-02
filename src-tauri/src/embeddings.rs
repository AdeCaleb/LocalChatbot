//! Embedding module for converting text to vectors.
//!
//! This module uses the `candle` ML framework to run a sentence transformer model
//! (all-MiniLM-L6-v2) locally. The model converts text into 384-dimensional vectors
//! that can be compared for semantic similarity.
//!
//! ## How It Works
//!
//! 1. Text is tokenized into word pieces (subwords)
//! 2. Tokens are fed through a transformer encoder
//! 3. The output is mean-pooled to get a single vector
//! 4. The vector is normalized for cosine similarity
//!
//! ## Why This Model?
//!
//! all-MiniLM-L6-v2 is a good balance of:
//! - Size: ~90MB (small enough to bundle or download quickly)
//! - Speed: Fast inference on CPU
//! - Quality: Good semantic similarity for retrieval tasks

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};
use std::path::PathBuf;
use tokenizers::Tokenizer;

/// The embedding dimension for all-MiniLM-L6-v2.
/// This is fixed by the model architecture.
pub const EMBEDDING_DIM: usize = 384;

/// The model ID on Hugging Face Hub.
const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// Errors that can occur during embedding operations.
#[derive(Debug)]
pub enum EmbeddingError {
    /// Failed to download or access model files
    ModelLoad(String),
    /// Failed to tokenize input text
    Tokenization(String),
    /// Failed during model inference
    Inference(String),
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingError::ModelLoad(msg) => write!(f, "Model load error: {}", msg),
            EmbeddingError::Tokenization(msg) => write!(f, "Tokenization error: {}", msg),
            EmbeddingError::Inference(msg) => write!(f, "Inference error: {}", msg),
        }
    }
}

impl std::error::Error for EmbeddingError {}

/// Wrapper around the BERT model for generating embeddings.
///
/// This struct owns both the model and tokenizer, providing a simple
/// interface for encoding text into vectors.
pub struct EmbeddingModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingModel {
    /// Creates a new embedding model, downloading weights if needed.
    ///
    /// The model files are cached in the Hugging Face cache directory:
    /// - Linux: ~/.cache/huggingface/hub/
    /// - macOS: ~/Library/Caches/huggingface/hub/
    /// - Windows: %USERPROFILE%\.cache\huggingface\hub\
    ///
    /// First load will download ~90MB of model files.
    pub fn new() -> Result<Self, EmbeddingError> {
        println!("Loading embedding model: {}", MODEL_ID);

        // Use CPU device (GPU support requires feature flags)
        let device = Device::Cpu;

        // Download model files from Hugging Face Hub
        let (config_path, tokenizer_path, weights_path) = download_model_files()?;

        // Load the tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbeddingError::Tokenization(e.to_string()))?;

        // Load and parse the model config
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to read config: {}", e)))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to parse config: {}", e)))?;

        // Load model weights from safetensors file
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)
                .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to load weights: {}", e)))?
        };

        // Build the model
        let model = BertModel::load(vb, &config)
            .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to build model: {}", e)))?;

        println!("Embedding model loaded successfully");

        Ok(EmbeddingModel {
            model,
            tokenizer,
            device,
        })
    }

    /// Encodes a single text string into a vector embedding.
    ///
    /// Returns a Vec<f32> of length EMBEDDING_DIM (384).
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = self.encode_batch(&[text])?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    /// Encodes multiple texts into vector embeddings.
    ///
    /// Batch encoding is more efficient than encoding one at a time
    /// because it allows better GPU/CPU utilization.
    ///
    /// Returns a Vec of embeddings, one per input text.
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Tokenize all texts
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| EmbeddingError::Tokenization(e.to_string()))?;

        // Find the maximum sequence length for padding
        let max_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0);

        // Build input tensors with padding
        let mut all_input_ids = Vec::new();
        let mut all_attention_mask = Vec::new();
        let mut all_token_type_ids = Vec::new();

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let attention = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            // Pad to max_len
            let mut padded_ids = ids.to_vec();
            let mut padded_attention = attention.to_vec();
            let mut padded_type_ids = type_ids.to_vec();

            padded_ids.resize(max_len, 0);
            padded_attention.resize(max_len, 0);
            padded_type_ids.resize(max_len, 0);

            all_input_ids.extend(padded_ids);
            all_attention_mask.extend(padded_attention);
            all_token_type_ids.extend(padded_type_ids);
        }

        let batch_size = texts.len();

        // Convert to tensors
        let input_ids = Tensor::from_vec(
            all_input_ids.iter().map(|&x| x as i64).collect::<Vec<_>>(),
            (batch_size, max_len),
            &self.device,
        )
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let attention_mask = Tensor::from_vec(
            all_attention_mask.iter().map(|&x| x as i64).collect::<Vec<_>>(),
            (batch_size, max_len),
            &self.device,
        )
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let token_type_ids = Tensor::from_vec(
            all_token_type_ids.iter().map(|&x| x as i64).collect::<Vec<_>>(),
            (batch_size, max_len),
            &self.device,
        )
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        // Run the model
        let output = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        // Mean pooling: average the token embeddings, considering attention mask
        let embeddings = mean_pooling(&output, &attention_mask)?;

        // Normalize embeddings for cosine similarity
        let normalized = normalize(&embeddings)?;

        // Convert to Vec<Vec<f32>>
        let result = normalized
            .to_vec2::<f32>()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        Ok(result)
    }
}

/// Downloads model files from Hugging Face Hub.
///
/// Returns paths to (config.json, tokenizer.json, model.safetensors).
fn download_model_files() -> Result<(PathBuf, PathBuf, PathBuf), EmbeddingError> {
    // Set the HuggingFace endpoint explicitly to avoid URL parsing issues
    std::env::set_var("HF_ENDPOINT", "https://huggingface.co");

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("huggingface")
        .join("hub");

    let api = ApiBuilder::new()
        .with_cache_dir(cache_dir)
        .with_progress(true)
        .build()
        .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to create API: {}", e)))?;

    let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));

    println!("Downloading model files (if not cached)...");

    let config_path = repo
        .get("config.json")
        .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to get config.json: {}", e)))?;

    let tokenizer_path = repo
        .get("tokenizer.json")
        .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to get tokenizer.json: {}", e)))?;

    let weights_path = repo
        .get("model.safetensors")
        .map_err(|e| EmbeddingError::ModelLoad(format!("Failed to get model.safetensors: {}", e)))?;

    Ok((config_path, tokenizer_path, weights_path))
}

/// Mean pooling over token embeddings.
///
/// This averages all token embeddings, but weighted by the attention mask
/// so padding tokens don't contribute to the final embedding.
fn mean_pooling(embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor, EmbeddingError> {
    // embeddings shape: (batch_size, seq_len, hidden_dim)
    // attention_mask shape: (batch_size, seq_len)

    // Expand attention mask to match embedding dimensions
    let mask = attention_mask
        .unsqueeze(2)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?
        .to_dtype(DType::F32)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Broadcast mask to embedding size
    let mask = mask
        .broadcast_as(embeddings.shape())
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Multiply embeddings by mask and sum
    let masked = embeddings
        .mul(&mask)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    let summed = masked
        .sum(1)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Sum mask for averaging
    let mask_sum = mask
        .sum(1)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Avoid division by zero
    let mask_sum = mask_sum
        .clamp(1e-9, f64::MAX)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Divide to get mean
    summed
        .div(&mask_sum)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))
}

/// L2 normalize embeddings.
///
/// Normalized embeddings allow using dot product as cosine similarity,
/// which is faster than computing cosine similarity directly.
fn normalize(embeddings: &Tensor) -> Result<Tensor, EmbeddingError> {
    // Compute L2 norm along the last dimension
    let squared = embeddings
        .sqr()
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    let sum_squared = squared
        .sum_keepdim(1)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    let norm = sum_squared
        .sqrt()
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    // Avoid division by zero
    let norm = norm
        .clamp(1e-12, f64::MAX)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

    embeddings
        .broadcast_div(&norm)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))
}

/// Compute cosine similarity between two embeddings.
///
/// For normalized embeddings, this is just the dot product.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embedding dimensions must match");
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model download, run with: cargo test -- --ignored
    fn test_embedding_model() {
        let model = EmbeddingModel::new().expect("Failed to load model");

        let text = "This is a test sentence.";
        let embedding = model.encode(text).expect("Failed to encode");

        assert_eq!(embedding.len(), EMBEDDING_DIM);

        // Check that embedding is normalized (L2 norm ~= 1)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Embedding should be normalized");
    }

    #[test]
    #[ignore] // Requires model download
    fn test_batch_encoding() {
        let model = EmbeddingModel::new().expect("Failed to load model");

        let texts = vec!["First sentence.", "Second sentence.", "Third sentence."];
        let embeddings = model.encode_batch(&texts).expect("Failed to encode batch");

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), EMBEDDING_DIM);
        }
    }

    #[test]
    #[ignore] // Requires model download
    fn test_semantic_similarity() {
        let model = EmbeddingModel::new().expect("Failed to load model");

        let similar1 = model.encode("The cat sat on the mat").unwrap();
        let similar2 = model.encode("A cat is sitting on a mat").unwrap();
        let different = model.encode("The stock market crashed today").unwrap();

        let sim_similar = cosine_similarity(&similar1, &similar2);
        let sim_different = cosine_similarity(&similar1, &different);

        println!("Similarity (similar sentences): {}", sim_similar);
        println!("Similarity (different sentences): {}", sim_different);

        // Similar sentences should have higher similarity
        assert!(
            sim_similar > sim_different,
            "Similar sentences should have higher cosine similarity"
        );
    }

    #[test]
    fn test_cosine_similarity() {
        // Test with known vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001); // Orthogonal = 0
    }
}
