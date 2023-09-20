use std::collections::{HashMap, HashSet};

use actix_web::{web, HttpResponse};
use timeline_lib::{
    api::common::parse_hash_list,
    db::structs::{BlockRecord, Commit},
    exchange::structs::{encode_exchange, Exchange},
};

use crate::{
    db::{open_db, read_blocks, read_commits_with_project_id, ServerDBError, DB},
    utils::e500,
};

fn prepare_exchange_response(db: &DB, project_id: &str) -> Result<Exchange, ServerDBError> {
    let mut all_commits: Vec<Commit> = vec![];
    let mut block_hashes: HashSet<String> = HashSet::new();

    let commits = read_commits_with_project_id(db, project_id)?;

    for commit in commits.into_iter() {
        let blocks_of_this_commit = parse_hash_list(commit.blocks.clone());

        all_commits.push(commit);

        for block in blocks_of_this_commit.into_iter() {
            block_hashes.insert(block);
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

pub async fn clone_project(path: web::Path<(String,)>) -> Result<HttpResponse, actix_web::Error> {
    let (project_id,) = path.into_inner();
    let db_result = open_db("/Users/bertalankormendy/Developer/timeline-backend/timeline-backend");
    if let Err(error) = db_result {
        println!("{:?}", error);
        return Err(e500(format!("{:?}", error)));
    }
    let db = db_result.unwrap();

    let exchange = prepare_exchange_response(&db, &project_id).map_err(e500)?;
    let response_bytes = encode_exchange(&exchange).map_err(e500)?;

    Ok(HttpResponse::Ok().body(response_bytes))
}
