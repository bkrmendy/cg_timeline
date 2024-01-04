use crate::db::db_ops::{DBError, Persistence, DB};

pub fn get_current_commit(db_path: &str) -> Result<String, DBError> {
    let db = Persistence::open(db_path)?;
    db.read_current_commit_pointer()
}
