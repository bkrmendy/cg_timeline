use crate::db::db_ops::{Persistence, DB};

pub fn list_braches(db_path: &str) -> anyhow::Result<Vec<String>> {
    Persistence::open(db_path).and_then(|db| db.read_all_branches())
}
