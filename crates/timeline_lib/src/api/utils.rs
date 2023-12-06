use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
};

use tempfile::NamedTempFile;

use crate::{
    db::structs::BlockRecord,
    exchange::structs::{decode_exchange, encode_exchange, Exchange},
};

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
    let older_set: HashSet<String> = HashSet::from_iter(older.into_iter());
    let newer_set: HashSet<String> = HashSet::from_iter(new_block_hashes);

    let diff: HashSet<String> = newer_set
        .difference(&older_set)
        .map(|a| a.to_owned())
        .collect();

    newer
        .into_iter()
        .filter(|b| diff.contains(&b.hash))
        .collect()
}

pub fn write_exchange_to_file(exchange: &Exchange, path: &str) -> Result<(), String> {
    let bytes = encode_exchange(exchange)?;

    let temp_file =
        NamedTempFile::new().map_err(|e| format!("Cannot create temp file: {:?}", e))?;

    fs::write(temp_file.path(), bytes).map_err(|e| format!("Cannot write bytes: {:?}", e))?;

    temp_file
        .persist(path)
        .map_err(|e| format!("Cannot persist file: {:?}", e))?;

    Ok(())
}

pub fn read_exchange_from_file(from_path: &str) -> Result<Exchange, String> {
    let mut file = File::open(from_path).map_err(|e| format!("Cannot open file: {:?}", e))?;

    let mut data = Vec::new();

    file.read_to_end(&mut data)
        .map_err(|e| format!("Cannot read file data: {:?}", e))?;

    let exchange: Exchange = decode_exchange(&data)?;

    Ok(exchange)
}

#[cfg(test)]
mod test {
    use tempfile::NamedTempFile;

    use crate::{
        api::init_command::MAIN_BRANCH_NAME,
        db::structs::{BlockRecord, Commit},
        exchange::structs::Exchange,
    };

    use super::{read_exchange_from_file, write_exchange_to_file};

    // #[test]
    // fn write_exchange_file_round_trip() {
    //     let file = NamedTempFile::new().unwrap();
    //     let path = file.path().to_str().unwrap();

    //     let original_exchange = Exchange {
    //         commits: vec![
    //             Commit {
    //                 hash: String::from("abc123"),
    //                 prev_commit_hash: String::from("def456"),
    //                 project_id: String::from("proj789"),
    //                 branch: String::from(MAIN_BRANCH_NAME),
    //                 message: String::from("Initial commit"),
    //                 author: String::from("John Doe"),
    //                 date: 1632870400, // Unix timestamp
    //                 header: vec![1, 2, 3, 4, 5],
    //                 blocks_and_pointers: String::from("blocks data 1"),
    //             },
    //             Commit {
    //                 hash: String::from("qwe234"),
    //                 prev_commit_hash: String::from("abc123"),
    //                 project_id: String::from("proj78"),
    //                 branch: String::from(MAIN_BRANCH_NAME),
    //                 message: String::from("Next commit"),
    //                 author: String::from("John Doe too"),
    //                 date: 1632870410, // Unix timestamp
    //                 header: vec![1, 2, 3, 4, 5],
    //                 blocks_and_pointers: String::from("blocks data 2"),
    //             },
    //         ],
    //         blocks: vec![
    //             BlockRecord {
    //                 hash: String::from("aaaabbbb"),
    //                 data: vec![1, 2, 3, 4],
    //             },
    //             BlockRecord {
    //                 hash: String::from("ccccdddd"),
    //                 data: vec![5, 6, 7, 8],
    //             },
    //         ],
    //     };

    //     write_exchange_to_file(&original_exchange, path).unwrap();
    //     let new_exchange = read_exchange_from_file(path).unwrap();

    //     assert_eq!(original_exchange, new_exchange);
    // }
}
