use std::collections::{HashMap, HashSet};

use actix_web::{web::Bytes, HttpRequest, HttpResponse};
use serde::Serialize;
use timeline_lib::{
    api::common::parse_blocks_and_pointers,
    db::structs::{BlockRecord, Commit},
    exchange::structs::{decode_sync, encode_exchange, Exchange},
};

use crate::{
    db::{
        open_db, read_blocks, read_descendants_of_commit, write_blocks, write_commits,
        ServerDBError, DB,
    },
    utils::e500,
};

fn prepare_exchange_response(db: &DB, local_tips: Vec<String>) -> Result<Exchange, ServerDBError> {
    let mut all_commits: Vec<Commit> = vec![];
    let mut block_hashes: HashSet<String> = HashSet::new();

    for hash in local_tips {
        let commits = read_descendants_of_commit(db, &hash)?;

        for commit in commits.into_iter() {
            let blocks_of_this_commit = parse_blocks_and_pointers(&commit.blocks_and_pointers);

            all_commits.push(commit);

            for block in blocks_of_this_commit.into_iter() {
                block_hashes.insert(block.hash);
            }
        }
    }

    let mut all_blocks: HashMap<String, BlockRecord> = HashMap::new();
    for block in read_blocks(db, block_hashes.into_iter().collect())? {
        all_blocks.insert(block.hash.clone(), block);
    }

    let all_blocks_vec = all_blocks.into_values().collect();

    Ok(Exchange {
        commits: all_commits,
        blocks: all_blocks_vec,
    })
}

#[derive(Serialize)]
struct Size {
    commits: usize,
    blocks: usize,
}

pub async fn v1_sync(_: HttpRequest, body: Bytes) -> Result<HttpResponse, actix_web::Error> {
    let sync = decode_sync(&body).map_err(e500)?;
    println!(
        "Sync received! Sync info: Commits: {}, blocks: {}, tips: {}",
        sync.exchange.commits.len(),
        sync.exchange.blocks.len(),
        sync.local_tips.join(",")
    );

    let db_result = open_db("/Users/bertalankormendy/Developer/timeline-backend/timeline-backend");
    if let Err(error) = db_result {
        println!("{:?}", error);
        return Err(e500(format!("{:?}", error)));
    }
    let db = db_result.unwrap();
    println!("Opened DB!");

    let response = prepare_exchange_response(&db, sync.local_tips).map_err(e500)?;
    println!("Prepared exchange!");

    write_blocks(&db, sync.exchange.blocks).map_err(e500)?;
    write_commits(&db, sync.exchange.commits).map_err(e500)?;

    let response_bytes = encode_exchange(&response).map_err(e500)?;

    Ok(HttpResponse::Ok().body(response_bytes))
}
