use crate::db::db_ops::{Persistence, DB};

pub fn get_current_commit(db_path: &str) -> anyhow::Result<String> {
    let db = Persistence::open(db_path)?;
    db.read_current_commit_pointer()
}
