mod loader;
mod vectorizer;
mod clustering;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::error::Error;
use std::collections::HashMap;
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};

use loader::{load_documents, LoaderError};
use vectorizer::{build_vocabulary, TextFeatures};
use clustering::cluster_documents;

mod constants;
use constants::{paths, ui, _cluestering};

fn main() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(paths::OUTPUT_CORE_DIR)?;

    let tier_docs = load_documents(paths::INPUT_JSON_PATH)?;
    println!("Found tiers: {}", tier_docs.len());

    let total_docs: usize = tier_docs.values().map(|v| v.len()).sum();
    let pb = ProgressBar::new(total_docs as u64);
    pb.set_style(ProgressStyle::with_template(ui::PROGRESS_TEMPLATE)
    .unwrap()
    .progress_chars(ui::PROGRESS_CHARS));

    let counter = Arc::new(AtomicUsize::new(0));
    let threshold = _cluestering::THRESHOLD;

    let results: Vec<_> = tier_docs
        .into_par_iter()
        .map(|(tier, docs)| {
            let local_counter = Arc::clone(&counter);
            
            let vocab = build_vocabulary(&docs);

            let text_feats: Vec<TextFeatures> = docs
                .par_iter()
                .map(|doc| {
                    let feats = TextFeatures::from_document(doc, &vocab);
                    local_counter.fetch_add(1, Ordering::Relaxed);
                    pb.inc(1);
                    feats
                })
                .collect();

            let clusters = cluster_documents(&docs, &text_feats, threshold);
            Ok((tier, clusters))
        })
        .collect::<Result<Vec<_>, LoaderError>>()?;

    pb.finish();

    let mut overall_results = HashMap::new();
    println!("\nRezultate clustering:");
    
    for (tier, cluster_vec) in results {
        overall_results.insert(tier.clone(), cluster_vec.clone());
        println!(" • Tier '{}': {} clustere", tier, cluster_vec.len());
    }

    let out_path = paths::CLUSTERS_OUTPUT_PATH;
    vectorizer::write_compressed_json(&overall_results)?;
    
    println!("\n✅ Procesare completă. Rezultate salvate în {}", out_path);
    Ok(())
}