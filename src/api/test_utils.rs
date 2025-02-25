#[cfg(test)]
pub fn init_db_from_file(db_path: &str, project_id: &str, blend_file_path: &str) {
    use super::init_command::init_db;

    init_db(db_path, project_id, blend_file_path).expect("Cannot init DB")
}

#[cfg(test)]
pub fn commit(db_path: &str, message: &str, blend_path: &str) {
    use super::create_new_checkpoint_command::create_new_checkpoint;

    create_new_checkpoint(blend_path, db_path, Some(message.to_owned()))
        .expect("Cannot create new commit")
}

#[cfg(test)]
pub fn new_branch(db_path: &str, name: &str) {
    use super::new_branch_command::create_new_branch;

    create_new_branch(db_path, name).expect("Cannot create new branch")
}

#[cfg(test)]
use crate::db::db_ops::ShortCommitRecord;

#[cfg(test)]
pub fn list_checkpoints(db_path: &str, branch: &str) -> Vec<ShortCommitRecord> {
    use super::log_checkpoints_command;

    log_checkpoints_command::list_checkpoints(db_path, branch).expect("Cannot list checkpoints")
}

#[cfg(test)]
pub struct SimpleCommit {
    pub hash: String,
    pub prev_hash: String,
    pub branch: String,
    pub message: String,
    pub blocks: String,
}

#[cfg(test)]
pub struct SimpleTimeline {
    pub project_id: String,
    pub author: String,
    pub blocks: Vec<String>,
    pub commits: Vec<SimpleCommit>,
}

#[cfg(test)]
pub fn init_db_from_simple_timeline(db_path: &str, simple_timeline: SimpleTimeline) {
    use crate::{
        api::init_command::{INITIAL_COMMIT_HASH, MAIN_BRANCH_NAME},
        db::{
            db_ops::{Persistence, DB},
            structs::{BlockRecord, Commit},
        },
    };

    let mut db = Persistence::open(db_path).expect("cannot open DB");

    let block_records: Vec<BlockRecord> = simple_timeline
        .blocks
        .into_iter()
        .map(|b| BlockRecord {
            hash: b.clone(),
            data: b.into_bytes(),
        })
        .collect();

    let mut last_commit_hash = String::from(INITIAL_COMMIT_HASH);
    let mut last_branch_name = String::from(MAIN_BRANCH_NAME);
    let mut date: u64 = 314;
    for commit in simple_timeline.commits {
        db.execute_in_transaction(|tx| {
            let this_hash = commit.hash.clone();
            let this_branch_name = commit.branch.clone();
            Persistence::write_branch_tip(tx, &this_branch_name, &this_hash)
                .expect("cannot write branch tip");

            Persistence::write_current_branch_name(tx, &last_branch_name)
                .expect("Cannot write current branch");

            Persistence::write_commit(
                tx,
                Commit {
                    hash: commit.hash,
                    prev_commit_hash: commit.prev_hash,
                    project_id: simple_timeline.project_id.clone(),
                    branch: commit.branch,
                    message: commit.message,
                    author: simple_timeline.author.clone(),
                    date,
                    header: vec![1, 2, 3],
                    blocks_and_pointers: vec![], // commit.blocks,
                },
            )
            .expect("cannot write commits");
            date += 1;
            last_commit_hash = this_hash;
            last_branch_name = this_branch_name;
            Ok(())
        })
        .unwrap();
    }

    db.execute_in_transaction(|tx| {
        Persistence::write_blocks(tx, &block_records).expect("cannot write blocks");
        Persistence::write_project_id(tx, &simple_timeline.project_id)?;
        Ok(())
    })
    .expect("cannot set pointers");
}

#[cfg(test)]
pub fn create_temp_file_path() -> String {
    use std::env;

    use uuid::Uuid;

    // Get the temporary directory
    let temp_dir = env::temp_dir();

    // Generate a unique file name, e.g., using UUID
    let file_name = format!("{}.tmp", Uuid::new_v4().to_string());

    // Combine the temp directory path with the new file name
    String::from(temp_dir.join(file_name).to_str().unwrap())

}