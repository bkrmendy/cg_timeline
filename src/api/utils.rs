use std::collections::HashSet;

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

    let diff: HashSet<&String> = newer_set
        .difference(&older_set)
        .collect();

    newer
        .into_iter()
        .filter(|b| diff.contains(&b.hash))
        .collect()
}
