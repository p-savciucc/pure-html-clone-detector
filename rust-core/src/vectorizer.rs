use crate::loader::Document;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;
use flate2::{Compression, write::GzEncoder};
use rayon::prelude::*;
use stop_words::{get, LANGUAGE};
use serde::Serialize;
use crate::constants::paths;

#[derive(Debug, Clone)]
pub struct TextFeatures {
    pub tfidf_vector: Vec<f64>,
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn build_vocabulary(docs: &[Document]) -> Vec<String> {
    let stop_words = get(LANGUAGE::English);
    let mut vocab_set: HashSet<String> = HashSet::new();

    let all_tokens: Vec<String> = docs
        .par_iter()
        .flat_map(|doc| tokenize(&doc.text))
        .collect();

    all_tokens.into_iter()
        .filter(|token| !stop_words.contains(token))
        .for_each(|token| {
            vocab_set.insert(token);
        });

    let mut vocab: Vec<String> = vocab_set.into_iter().collect();
    vocab.par_sort_unstable();
    vocab
}

pub fn write_compressed_json<T: Serialize>(data: &T) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(paths::CLUSTERS_OUTPUT_PATH)?;
    let encoder = GzEncoder::new(file, Compression::best());
    let mut writer = BufWriter::new(encoder);
    serde_json::to_writer(&mut writer, data)?;
    Ok(())
}

impl TextFeatures {
    pub fn from_document(doc: &Document, vocab: &[String]) -> Self {
        let tokens = tokenize(&doc.text);
        let tfidf = Self::compute_tfidf(&tokens, vocab);
        TextFeatures {
            tfidf_vector: tfidf,
        }
    }

    fn compute_tfidf(tokens: &[String], vocab: &[String]) -> Vec<f64> {
        let mut freq: HashMap<&str, f64> = HashMap::new();
        for token in tokens {
            *freq.entry(token).or_insert(0.0) += 1.0;
        }

        let total_terms = tokens.len() as f64;
        vocab.iter()
            .map(|w| freq.get(w.as_str()).unwrap_or(&0.0) / total_terms)
            .collect()
    }
}
