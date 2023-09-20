use crate::db::db_ops::{DBError, Persistence, DB};

pub fn list_braches(db_path: &str) -> Result<Vec<String>, DBError> {
    Persistence::open(db_path).and_then(|db| db.read_all_branches())
}
