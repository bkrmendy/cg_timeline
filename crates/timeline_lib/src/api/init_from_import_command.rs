use std::collections::HashMap;

use crate::{
    db::db_ops::{DBError, Persistence, DB},
    exchange::structs::Exchange,
};

use super::{
    import_exchange::import_exchange, init_command::MAIN_BRANCH_NAME,
    restore_command::restore_checkpoint,
};

pub fn init_from_import_command(
    db_path: &str,
    exchange: Exchange,
    file_path: &str,
) -> Result<(), DBError> {
    let hash = {
        import_exchange(db_path, exchange)?;
        let mut db = Persistence::open(db_path)?;

        let mut remote_tips: HashMap<String, String> = HashMap::new();
        let branches = db.read_all_branches()?;
        for branch in branches {
            let tip = db.read_branch_tip(&branch)?;
            if let Some(hash) = tip {
                remote_tips.insert(branch, hash);
            }
        }

        db.execute_in_transaction(|tx| {
            for (branch, tip) in remote_tips.into_iter() {
                Persistence::write_remote_branch_tip(tx, &branch, &tip)?;
            }

            Ok(())
        })?;
        db.read_branch_tip(MAIN_BRANCH_NAME)?.unwrap() // TODO
    };

    restore_checkpoint(file_path, db_path, &hash)
}
