use crate::db::{
    db_ops::{DBError, Persistence, DB},
    structs::Commit,
};

use super::common::{blend_file_data_from_file, get_file_mod_time};

pub const INITIAL_COMMIT_HASH: &str = "initial";
pub const MAIN_BRANCH_NAME: &str = "main";

pub fn init_db(db_path: &str, project_id: &str, path_to_blend: &str) -> Result<(), DBError> {
    let blend_data = blend_file_data_from_file(path_to_blend)
        .map_err(|e| DBError::Error(format!("Error parsing blend file: {}", e)))?;

    let file_last_mod_time = get_file_mod_time(path_to_blend)?;

    let mut db = Persistence::open(db_path)?;

    let name = db.read_name()?.unwrap_or("Anon".to_owned());

    let hash = blend_data.hash.clone();

    db.write_blocks_str(&blend_data.hash, &blend_data.blocks)?;

    db.write_blocks(&blend_data.block_data)?;

    db.execute_in_transaction(|tx| {
        Persistence::write_branch_tip(tx, MAIN_BRANCH_NAME, &blend_data.hash)?;

        let commit = Commit {
            hash: blend_data.hash,
            prev_commit_hash: String::from(INITIAL_COMMIT_HASH),
            project_id: String::from(project_id),
            branch: String::from(MAIN_BRANCH_NAME),
            message: String::from("Initial checkpoint"),
            author: name,
            date: file_last_mod_time as u64,
            header: blend_data.header_bytes,
            blocks: blend_data.blocks,
        };

        Persistence::write_commit(tx, commit)
    })?;

    db.execute_in_transaction(|tx| {
        Persistence::write_branch_tip(tx, MAIN_BRANCH_NAME, &hash)?;
        Persistence::write_remote_branch_tip(tx, MAIN_BRANCH_NAME, &hash)?;
        Persistence::write_current_commit_pointer(tx, &hash)?;
        Persistence::write_current_branch_name(tx, MAIN_BRANCH_NAME)?;
        Persistence::write_project_id(tx, project_id)?;
        Persistence::write_last_modifiction_time(tx, file_last_mod_time)?;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;

    use crate::{
        api::init_command::MAIN_BRANCH_NAME,
        db::db_ops::{Persistence, DB},
    };

    use super::init_db;

    #[test]
    fn test_post_init_state() {
        let tmp_dir = TempDir::new().expect("Cannot create temp dir");
        let tmp_path = tmp_dir.path().to_str().expect("Cannot get temp dir path");
        init_db(tmp_path, "my amazing project", "data/untitled.blend").expect("Cannot init DB");

        let db = Persistence::open(tmp_path).expect("Cannot open db");
        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);
        let current_commit_hash = db
            .read_current_commit_pointer()
            .expect("Cannot read current commit pointer");
        assert_eq!(current_commit_hash, "5bdd30ea8c1523bc75eddbcb1e59e4c7q");

        let project_id = db.read_project_id().expect("Cannot read project id");
        assert_eq!(project_id, "my amazing project")
    }
}
