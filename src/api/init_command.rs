use std::time::Instant;

use crate::{
    api::utils::get_file_size_str,
    db::{
        db_ops::{DBError, Persistence, DB},
        structs::Commit,
    },
    measure_time,
};

use super::common::{blend_file_data_from_file, get_file_mod_time};

pub const INITIAL_COMMIT_HASH: &str = "initial";
pub const MAIN_BRANCH_NAME: &str = "main";

pub fn init_db(db_path: &str, project_id: &str, path_to_blend: &str) -> anyhow::Result<()> {
    let connect_command_timer = Instant::now();
    let blend_data = blend_file_data_from_file(path_to_blend)
        .map_err(|e| DBError::Error(format!("Error parsing blend file: {}", e)))?;

    let file_last_mod_time = get_file_mod_time(path_to_blend)?;

    let mut db = Persistence::open(db_path)?;

    let name = db.read_name()?.unwrap_or("Anon".to_owned());

    let hash = blend_data.hash.clone();

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
            blocks_and_pointers: blend_data.blocks_and_pointers_bytes,
        };

        Persistence::write_commit(tx, commit)
    })?;

    db.execute_in_transaction(|tx| {
        measure_time!(format!("Writing blocks {:?}", path_to_blend), {
            Persistence::write_blocks(tx, &blend_data.block_data)?;
        });
        Persistence::write_branch_tip(tx, MAIN_BRANCH_NAME, &hash)?;
        Persistence::write_current_commit_pointer(tx, &hash)?;
        Persistence::write_current_branch_name(tx, MAIN_BRANCH_NAME)?;
        Persistence::write_project_id(tx, project_id)?;
        Persistence::write_last_modifiction_time(tx, file_last_mod_time)?;
        Ok(())
    })?;

    println!("Connecting took {:?}", connect_command_timer.elapsed());
    println!("Size of timeline db: {}", get_file_size_str(db_path));

    Ok(())
}

#[cfg(test)]
mod test {

    use tempfile::NamedTempFile;

    use crate::{
        api::init_command::MAIN_BRANCH_NAME,
        db::db_ops::{Persistence, DB},
    };

    use super::init_db;

    #[test]
    fn test_post_init_state() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");
        init_db(
            tmp_path,
            "my amazing project",
            "data/fixtures/untitled.blend",
        )
        .expect("Cannot init DB");

        let db = Persistence::open(tmp_path).expect("Cannot open db");
        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);
        let current_commit_hash = db
            .read_current_commit_pointer()
            .expect("Cannot read current commit pointer");

        insta::assert_debug_snapshot!(current_commit_hash, @r###""74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d""###);

        let project_id = db.read_project_id().expect("Cannot read project id");
        assert_eq!(project_id, "my amazing project")
    }
}
