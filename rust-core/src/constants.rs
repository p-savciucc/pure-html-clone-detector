pub mod paths {
    pub const OUTPUT_CORE_DIR: &str = "../../output/rust-core";
    pub const INPUT_JSON_PATH: &str = "../../output/node-renderer/output_pool.json";
    pub const CLUSTERS_OUTPUT_PATH: &str = "../../output/rust-core/clusters.json.gz";
}

pub mod ui {
    pub const PROGRESS_TEMPLATE: &str = "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})";
    pub const PROGRESS_CHARS: &str = "#>-";
}

pub mod _cluestering {
    pub const THRESHOLD: f64 = 0.94;
}