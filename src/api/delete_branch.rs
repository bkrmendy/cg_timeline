use crate::db::db_ops::{DBError, Persistence, DB};

use super::init_command::MAIN_BRANCH_NAME;

pub fn delete_branch(db_path: &str, branch_name: &str) -> Result<(), DBError> {
    let mut db = Persistence::open(db_path)?;
    let current_branch_name = db.read_current_branch_name()?;

    if branch_name == MAIN_BRANCH_NAME {
        return Err(DBError::Error("Cannot delete the main branch".to_owned()));
    }

    if current_branch_name == branch_name {
        return Err(DBError::Error("Cannot delete current branch".to_owned()));
    }

    let branch = db.read_branch_tip(branch_name)?;
    if branch.is_none() {
        // not really an error but too lazy to make a new type
        return Err(DBError::Error(
            "Cannot delete non-existent branch".to_owned(),
        ));
    }

    db.execute_in_transaction(|tx| Persistence::delete_branch_with_commits(tx, branch_name))?;

    Ok(())
}
