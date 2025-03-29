use crate::loader::Document;
use crate::vectorizer::TextFeatures;
use std::collections::{HashSet, HashMap};

#[derive(Debug)]
struct Cluster {
    centroid: Vec<f64>,
    members: HashSet<String>,
}

pub fn cluster_documents(
    docs: &[Document],
    text_features: &[TextFeatures],
    threshold: f64,
) -> Vec<Vec<String>> {
    let feature_map: HashMap<_, _> = docs.iter()
        .enumerate()
        .map(|(i, doc)| (doc.filename.clone(), &text_features[i].tfidf_vector))
        .collect();

    let mut clusters: Vec<Cluster> = Vec::new();

    for doc in docs {
        let doc_vector = feature_map[&doc.filename];
        let mut best_cluster = None;
        let mut best_score = 0.0;

        for (i, cluster) in clusters.iter().enumerate() {
            let sim = cosine_similarity(doc_vector, &cluster.centroid);
            if sim > best_score && sim >= threshold {
                best_score = sim;
                best_cluster = Some(i);
            }
        }

        match best_cluster {
            Some(idx) => {
                clusters[idx].members.insert(doc.filename.clone());
                update_centroid(&mut clusters[idx], doc_vector);
            },
            None => {
                let mut members = HashSet::new();
                members.insert(doc.filename.clone());
                clusters.push(Cluster {
                    centroid: doc_vector.to_vec(),
                    members,
                });
            }
        }
    }

    clusters.into_iter()
        .map(|c| c.members.into_iter().collect())
        .collect()
}


fn update_centroid(cluster: &mut Cluster, doc_vector: &[f64]) {
    let n = cluster.members.len() as f64;
    for (i, val) in doc_vector.iter().enumerate() {
        cluster.centroid[i] = (cluster.centroid[i] * (n - 1.0) + val) / n;
    }
}

fn cosine_similarity(v1: &[f64], v2: &[f64]) -> f64 {
    let dot: f64 = v1.iter().zip(v2).map(|(a, b)| a * b).sum();
    let norm1: f64 = v1.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm2: f64 = v2.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm1 == 0.0 || norm2 == 0.0 {
        0.0
    } else {
        dot / (norm1 * norm2)
    }
}
