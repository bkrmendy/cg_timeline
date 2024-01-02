use crate::db::db_ops::{DBError, Persistence, DB};

pub fn get_current_branch(db_path: &str) -> Result<String, DBError> {
    Persistence::open(db_path).and_then(|conn| conn.read_current_branch_name())
}
