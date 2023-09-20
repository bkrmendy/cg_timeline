use crate::db::db_ops::{DBError, Persistence, DB};

use super::common::read_latest_commit_hash_on_branch;

pub fn get_latest_commit(db_path: &str) -> Result<String, DBError> {
    let db = Persistence::open(db_path)?;
    let current_branch_name = db.read_current_branch_name()?;
    read_latest_commit_hash_on_branch(&db, &current_branch_name)
}
