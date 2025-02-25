use anyhow::bail;

use crate::{
    api::{
        common::{
            blend_file_data_from_file, get_file_mod_time, parse_blocks_and_pointers,
            read_latest_commit_hash_on_branch,
        },
        utils::{block_hash_diff, get_file_size_str},
    },
    db::{
        db_ops::{DBError, Persistence, DB},
        structs::Commit,
    },
    measure_time,
};

use std::time::Instant;

pub fn create_new_checkpoint(
    file_path: &str,
    db_path: &str,
    message: Option<String>,
) -> anyhow::Result<()> {
    let mut conn = Persistence::open(db_path)?;

    let file_last_mod_time: i64 = get_file_mod_time(file_path)?;
    println!(
        "Size of timeline db before creating checkpoint: {}",
        get_file_size_str(db_path)
    );

    let start_checkpoint_command = Instant::now();
    let blend_data = blend_file_data_from_file(file_path)
        .map_err(|e| DBError::Error(format!("Error parsing blend file: {}", e)))?;

    let hash_already_exists = conn.check_commit_exists(&blend_data.hash)?;
    // A checkpoint with the same hash already exists, bail out
    if hash_already_exists {
        return Ok(());
    }

    let current_commit_hash = conn.read_current_commit_pointer()?;
    let current_branch_name = conn.read_current_branch_name()?;

    let latest_commit_hash_on_branch =
        read_latest_commit_hash_on_branch(&conn, &current_branch_name)?;

    // This is the detached HEAD situation
    if current_commit_hash != latest_commit_hash_on_branch {
        bail!(DBError::Consistency(String::from(
            "Create a new branch to create a checkpoint",
        )));
    }

    let latest_commit = conn
        .read_commit(&latest_commit_hash_on_branch)
        .ok()
        .flatten();

    let new_blocks_since_latest = match latest_commit {
        None => blend_data.block_data,
        Some(commit) => {
            let hashes = parse_blocks_and_pointers(&commit.blocks_and_pointers)
                .into_iter()
                .map(|b| b.hash)
                .collect();
            block_hash_diff(hashes, blend_data.block_data)
        }
    };

    println!(
        "Number new of blocks since latest: {}",
        new_blocks_since_latest.len()
    );

    let project_id = conn.read_project_id()?;

    let name = conn.read_name()?.unwrap_or("".to_owned());

    conn.execute_in_transaction(|tx| {
        measure_time!(format!("Writing blocks {:?}", file_path), {
            Persistence::write_blocks(tx, &new_blocks_since_latest[..])?
        });
        Persistence::write_branch_tip(tx, &current_branch_name, &blend_data.hash)?;
        Persistence::write_last_modifiction_time(tx, file_last_mod_time)?;
        Persistence::write_current_commit_pointer(tx, &blend_data.hash)?;

        let commit = Commit {
            hash: blend_data.hash,
            prev_commit_hash: latest_commit_hash_on_branch,
            project_id,
            branch: current_branch_name,
            message: message.unwrap_or_default(),
            author: name,
            date: file_last_mod_time as u64,
            header: blend_data.header_bytes,
            blocks_and_pointers: blend_data.blocks_and_pointers_bytes,
        };
        Persistence::write_commit(tx, commit)
    })?;

    println!(
        "Creating checkpoint took {:?}",
        start_checkpoint_command.elapsed()
    );
    println!(
        "Size of timeline db after creating checkpoint: {}",
        get_file_size_str(db_path)
    );
    Ok(())
}

#[cfg(test)]
mod test {
    use tempfile::NamedTempFile;

    use crate::{
        api::{
            common::read_latest_commit_hash_on_branch, init_command::MAIN_BRANCH_NAME, test_utils,
        },
        db::db_ops::{Persistence, DB},
    };

    use super::create_new_checkpoint;

    #[test]
    fn test_initial_commit() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        // Creates exactly one commit
        assert_eq!(
            test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME).len(),
            1
        );

        insta::assert_debug_snapshot!(
            test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME),
                @r###"
        [
            ShortCommitRecord {
                hash: "74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d",
                branch: "main",
                message: "Initial checkpoint",
            },
        ]
        "###
        );

        create_new_checkpoint(
            "data/fixtures/untitled_2.blend",
            tmp_path,
            Some("Initial checkpoint".to_owned()),
        )
        .unwrap();

        // Creates exactly one commit
        insta::assert_debug_snapshot!(test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME), @r###"
        [
            ShortCommitRecord {
                hash: "94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84",
                branch: "main",
                message: "Initial checkpoint",
            },
            ShortCommitRecord {
                hash: "74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d",
                branch: "main",
                message: "Initial checkpoint",
            },
        ]
        "###);

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        let commit = db
            .read_commit("94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84")
            .unwrap()
            .unwrap();

        // commit.blocks omitted, too long
        // commit.date omitted, not stable
        // commit.header omitted, not interesting enough
        assert_eq!(commit.author, "");
        assert_eq!(commit.branch, MAIN_BRANCH_NAME);
        assert_eq!(commit.hash, "94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84");
        assert_eq!(commit.message, "Initial checkpoint");
        assert_eq!(commit.prev_commit_hash, "74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d");
        assert_eq!(commit.project_id, "my-cool-project");

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        // The current branch name stays the same
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);

        let latest_commit_hash = read_latest_commit_hash_on_branch(&db, &current_branch_name)
            .expect("Cannot read latest commit");

        // The latest commit hash is updated to the hash of the new commit
        assert_eq!(latest_commit_hash, "94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84");

        // The tip of `main` is updated to the hash of the new commit
        let main_tip = db.read_branch_tip(MAIN_BRANCH_NAME).unwrap().unwrap();
        assert_eq!(main_tip, "94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84");
    }

    #[test]
    fn test_next_commit() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        create_new_checkpoint(
            "data/fixtures/untitled_2.blend",
            tmp_path,
            Some("Message".to_owned()),
        )
        .unwrap();
        create_new_checkpoint(
            "data/fixtures/untitled_3.blend",
            tmp_path,
            Some("Message".to_owned()),
        )
        .unwrap();

        assert_eq!(
            test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME).len(),
            3
        );

        insta::assert_debug_snapshot!(
            test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME)
                .into_iter()
                .map(|c| c.hash)
                .collect::<Vec<String>>(),
            @r###"
        [
            "5e0e611ae1c01a131edd79b57d96d9ca4714a823a567c5fa73f3a973503aa0f6c660f2570ea5d9c04942a3e4ab34d35f71598be62e1cb8a7a40b4826aac4009c",
            "94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84",
            "74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d",
        ]
        "###
        );

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        // The current branch name stays the same
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);

        let latest_commit_hash = read_latest_commit_hash_on_branch(&db, &current_branch_name)
            .expect("Cannot read latest commit");

        // The latest commit hash is updated to the hash of the new commit
        insta::assert_debug_snapshot!(latest_commit_hash, @r###""5e0e611ae1c01a131edd79b57d96d9ca4714a823a567c5fa73f3a973503aa0f6c660f2570ea5d9c04942a3e4ab34d35f71598be62e1cb8a7a40b4826aac4009c""###);

        // The tip of `main` is updated to the hash of the new commit
        let main_tip = db.read_branch_tip(MAIN_BRANCH_NAME).unwrap().unwrap();
        insta::assert_debug_snapshot!(main_tip, @r###""5e0e611ae1c01a131edd79b57d96d9ca4714a823a567c5fa73f3a973503aa0f6c660f2570ea5d9c04942a3e4ab34d35f71598be62e1cb8a7a40b4826aac4009c""###);
    }
}
