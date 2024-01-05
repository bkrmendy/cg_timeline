use crate::db::db_ops::{Persistence, DB};

pub fn get_current_branch(db_path: &str) -> anyhow::Result<String> {
    Persistence::open(db_path).and_then(|conn| conn.read_current_branch_name())
}
