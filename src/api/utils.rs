use humansize::{format_size, DECIMAL};
use std::{collections::HashSet, fs};

use crate::db::structs::BlockRecord;

#[macro_export]
macro_rules! measure_time {
    ($name:expr, $block:expr) => {{
        #[cfg(debug_assertions)]
        {
            println!("{}...", $name);
            let start = std::time::Instant::now();
            let result = $block;
            let duration = start.elapsed();
            println!("{} took: {:?}", $name, duration);
            result
        }
        #[cfg(not(debug_assertions))]
        {
            $block
        }
    }};
}

pub fn block_hash_diff(older: Vec<String>, newer: Vec<BlockRecord>) -> Vec<BlockRecord> {
    let new_block_hashes = newer.iter().map(|b| b.hash.clone());
    let older_set: HashSet<String> = HashSet::from_iter(older);
    let newer_set: HashSet<String> = HashSet::from_iter(new_block_hashes);

    let diff: HashSet<&String> = newer_set.difference(&older_set).collect();

    newer
        .into_iter()
        .filter(|b| diff.contains(&b.hash))
        .collect()
}

pub fn get_file_size_str(file_path: &str) -> String {
    match fs::metadata(file_path) {
        Ok(metadata) => {
            let raw_size = metadata.len();
            format_size(raw_size, DECIMAL)
        }
        Err(_) => String::from("Failed to get file size"),
    }
}
