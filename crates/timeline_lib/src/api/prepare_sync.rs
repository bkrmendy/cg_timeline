use std::collections::{HashMap, HashSet};

use crate::{
    db::{
        db_ops::{DBError, Persistence, DB},
        structs::{BlockRecord, Commit},
    },
    exchange::structs::{Exchange, Sync},
};

use super::common::parse_blocks_and_pointers;

pub fn prepare_sync(db_path: &str) -> Result<Sync, DBError> {
    let db = Persistence::open(db_path)?;

    let branches = db.read_all_branches()?;
    let mut local_tips: Vec<String> = vec![];
    let mut remote_tips: Vec<String> = vec![];

    for branch in branches {
        db.read_branch_tip(&branch)
            .map(|branch| branch.map(|tip| local_tips.push(tip)))?;

        db.read_remote_branch_tip(&branch)
            .map(|tip| remote_tips.push(tip))?;
    }

    let mut all_commits: Vec<Commit> = vec![];
    let mut block_hashes: HashSet<String> = HashSet::new();

    for hash in remote_tips {
        let commits = db.read_descendants_of_commit(&hash)?;

        for commit in commits.into_iter() {
            let blocks_of_this_commit: Vec<String> =
                parse_blocks_and_pointers(&commit.blocks_and_pointers)
                    .into_iter()
                    .map(|b| b.hash)
                    .collect();

            all_commits.push(commit);

            for block in blocks_of_this_commit.into_iter() {
                block_hashes.insert(block);
            }
        }
    }

    let mut all_blocks: HashMap<String, BlockRecord> = HashMap::new();
    for block in db.read_blocks(block_hashes.into_iter().collect())? {
        all_blocks.insert(block.hash.clone(), block);
    }

    let all_blocks_vec = all_blocks.into_values().collect();

    Ok(Sync {
        local_tips,
        exchange: Exchange {
            commits: all_commits,
            blocks: all_blocks_vec,
        },
    })
}
