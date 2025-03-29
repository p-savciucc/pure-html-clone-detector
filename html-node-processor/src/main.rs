use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

mod constants;
use constants::*;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use kuchiki::traits::TendrilSink;
use kuchiki::NodeData;
use rayon::prelude::*;
use serde_json::json;
use serde::Serialize;

#[derive(Debug)]
struct ProcessedDocument {
    tier: String,
    filename: String,
    text: String,
    metadata: Metadata,
}

#[derive(Debug, Serialize)]
struct Metadata {
    title: String,
    headings: Vec<String>,
    links: Vec<String>,
    word_count: usize,
    meta_description: Option<String>,
    og_image: Option<String>,
    language: Option<String>,
    schema_types: Vec<String>,
}

struct FileObj {
    tier: String,
    path: PathBuf,
    filename: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    fs::create_dir_all(OUTPUT_DIR)?;

    let files = collect_html_files(DATASET_DIR)?;
    println!("📂 Found {} HTML files", files.len());

    let multi = MultiProgress::new();
    let main_progress = multi.add(ProgressBar::new(files.len() as u64));
    main_progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let counter = Arc::new(AtomicUsize::new(0));
    
    let (results, errors): (Vec<_>, Vec<_>) = files
        .par_iter()
        .map(|file| {
            let cnt = counter.fetch_add(1, Ordering::Relaxed) + 1;
            main_progress.set_position(cnt as u64);

            let content = match fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(e) => return (None, Some(format!("{}: {}", file.path.display(), e))),
            };

            let (text, metadata) = analyze_content(&content);
            let truncated_text = smart_truncate(text, TEXT_TRUNCATE);

            let doc = ProcessedDocument {
                tier: file.tier.clone(),
                filename: file.filename.clone(),
                text: truncated_text,
                metadata,
            };

            (Some(doc), None)
        })
        .unzip();

    main_progress.finish();
    log_errors(errors)?;

    let mut tiers: BTreeMap<String, Vec<ProcessedDocument>> = BTreeMap::new();
    for doc in results.into_iter().flatten() {
        tiers.entry(doc.tier.clone()).or_default().push(doc);
    }

    let output_data: BTreeMap<_, _> = tiers
        .into_iter()
        .map(|(tier, mut docs)| {
            docs.sort_by_cached_key(|d| d.filename.clone());
            let tier_total = docs.len();

            let documents = docs
                .into_iter()
                .enumerate()
                .map(|(idx, doc)| {
                    json!({
                        "filename": doc.filename,
                        "text": doc.text,
                        "tier": tier,
                        "tierIndex": idx + 1,
                        "tierTotal": tier_total,
                        "metadata": doc.metadata
                    })
                })
                .collect();

            (tier, documents)
        })
        .collect();

    write_output(output_data)?;

    println!("✅ Processing completed in {:.2?}", start_time.elapsed());
    Ok(())
}

fn analyze_content(html: &str) -> (String, Metadata) {
    let document = kuchiki::parse_html().one(html);
    let mut metadata = Metadata {
        title: String::new(),
        headings: Vec::new(),
        links: Vec::new(),
        word_count: 0,
        meta_description: None,
        og_image: None,
        language: None,
        schema_types: Vec::new(),
    };

    if let Ok(html_node) = document.select_first("html") {
        if let Some(lang) = html_node.attributes.borrow().get("lang") {
            metadata.language = Some(lang.to_string());
        }
    }

    if let Ok(meta_nodes) = document.select("meta") {
        for node in meta_nodes {
            let attrs = node.attributes.borrow();
            if let Some(name) = attrs.get("name") {
                if name == "description" {
                    metadata.meta_description = attrs.get("content").map(|s| s.to_string());
                }
            }
            if let Some(property) = attrs.get("property") {
                if property == "og:image" {
                    metadata.og_image = attrs.get("content").map(|s| s.to_string());
                }
            }
        }
    }

    if let Ok(schema_nodes) = document.select("[itemtype]") {
        for node in schema_nodes {
            if let Some(itemtype) = node.attributes.borrow().get("itemtype") {
                metadata.schema_types.push(itemtype.to_string());
            }
        }
    }

    if let Ok(title_node) = document.select_first("title") {
        metadata.title = title_node.text_contents().trim().to_string();
    }

    let mut text_parts = Vec::new();
    let mut headings = Vec::new();

    for node in document.descendants() {
        match node.data() {
            NodeData::Element(el) => {
                let tag_name = el.name.local.to_string();

                match tag_name.as_str() {
                    "script" | "style" | "noscript" => continue,
                    "a" => {
                        if let Some(href) = el.attributes.borrow().get("href") {
                            metadata.links.push(href.to_string());
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let heading = node.text_contents().trim().to_string();
                        if !heading.is_empty() {
                            headings.push(heading.clone());
                            text_parts.push(format!("\n[{}] {}\n", tag_name.to_uppercase(), heading));
                        }
                    }
                    "p" | "div" | "section" => {
                        let content = node.text_contents().trim().to_string();
                        if !content.is_empty() {
                            text_parts.push(content);
                        }
                    }
                    "li" => {
                        let li_text = node.text_contents().trim().to_string();
                        if !li_text.is_empty() {
                            text_parts.push(format!("• {}", li_text));
                        }
                    }
                    "br" => text_parts.push("\n".to_string()),
                    _ => {
                        let content = node.text_contents().trim().to_string();
                        if !content.is_empty() {
                            text_parts.push(content);
                        }
                    }
                }
            }
            NodeData::Text(text) => {
                let text_ref = text.borrow();
                let content = text_ref.trim();
                if !content.is_empty() {
                    text_parts.push(content.to_string());
                }
            }
            _ => {}
        }
    }

    metadata.headings = headings;
    let full_text = text_parts.join(" ");
    metadata.word_count = full_text.split_whitespace().count();

    (full_text, metadata)
}

fn smart_truncate(text: String, max_len: usize) -> String {
    let important_sections = vec!["main", "article", "section"];
    let mut prioritized_text = String::new();
    
    let document = kuchiki::parse_html().one(text.clone());
    
    for section in important_sections {
        if let Ok(nodes) = document.select(section) {
            for node in nodes {
                prioritized_text.push_str(&node.text_contents());
            }
        }
    }

    let final_text = if prioritized_text.is_empty() {
        text 
    } else {
        prioritized_text
    };

    if final_text.len() <= max_len {
        return final_text;
    }

    match final_text.char_indices().nth(max_len) {
        Some((idx, _)) => {
            let trunc_point = final_text[..idx].rfind(|c: char| c.is_whitespace()).unwrap_or(idx);
            format!("{}... [truncated]", &final_text[..trunc_point].trim_end())
        }
        None => final_text,
    }
}

fn collect_html_files(base_dir: &str) -> Result<Vec<FileObj>, std::io::Error> {
    let mut collector = Vec::new();
    for tier_entry in fs::read_dir(base_dir)? {
        let tier_entry = tier_entry?;
        let tier_path = tier_entry.path();
        if tier_path.is_dir() {
            let tier_name = tier_entry.file_name().to_string_lossy().into_owned();
            for file_entry in fs::read_dir(&tier_path)? {
                let file_entry = file_entry?;
                let path = file_entry.path();
                if path.extension().map_or(false, |ext| ext == "html") {
                    let filename = path.file_name().unwrap().to_string_lossy().into_owned();
                    collector.push(FileObj {
                        tier: tier_name.clone(),
                        path,
                        filename,
                    });
                }
            }
        }
    }
    Ok(collector)
}

fn log_errors(errors: Vec<Option<String>>) -> Result<(), Box<dyn std::error::Error>> {
    let errors: Vec<_> = errors.into_iter().flatten().collect();
    if errors.is_empty() {
        return Ok(());
    }
    let mut error_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ERROR_LOG)?;
    for err in errors {
        writeln!(error_file, "{}", err)?;
    }
    Ok(())
}

fn write_output(data: BTreeMap<String, Vec<serde_json::Value>>) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new(OUTPUT_DIR).join("output_pool.json");
    let file = fs::File::create(&output_path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &data)?;
    writer.flush()?;
    println!("📊 Results saved to {}", output_path.display());
    Ok(())
}