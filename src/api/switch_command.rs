use anyhow::bail;

use crate::db::db_ops::{DBError, Persistence, DB};

use super::restore_command::restore_checkpoint;

pub fn switch_branches(db_path: &str, branch_name: &str, file_path: &str) -> anyhow::Result<()> {
    let hash = {
        let mut db = Persistence::open(db_path)?;

        let tip = db.read_branch_tip(branch_name)?;

        if tip.is_none() {
            bail!(DBError::Consistency(
                "Branch has no corresponding tip".to_owned(),
            ));
        }

        let hash = tip.unwrap();

        db.execute_in_transaction(|tx| {
            Persistence::write_current_branch_name(tx, branch_name)?;
            Ok(())
        })?;

        hash
    };

    restore_checkpoint(file_path, db_path, &hash)
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

    use super::switch_branches;

    #[test]
    fn test_checkout_non_existent_branch() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        let res = switch_branches(tmp_path, "unknown", "void.blend");
        assert!(res.is_err());

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        let branches = db.read_all_branches().unwrap();

        // no new branch is added
        insta::assert_debug_snapshot!(branches, @r###"
        [
            "main",
        ]
        "###);

        let main_tip = db.read_branch_tip(MAIN_BRANCH_NAME).unwrap().unwrap();

        // tip of main stays the same
        insta::assert_debug_snapshot!(main_tip, @r###""74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d""###);

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        // The current branch name stays the same
        insta::assert_debug_snapshot!(&current_branch_name, @r###""main""###);

        let latest_commit_hash = read_latest_commit_hash_on_branch(&db, &current_branch_name)
            .expect("Cannot read latest commit");

        // The latest commit hash stays the same
        insta::assert_debug_snapshot!(latest_commit_hash, @r###""74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d""###);
    }

    #[test]
    fn test_checkout_real_branch() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path: &str = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        // a commit to `main`
        test_utils::commit(tmp_path, "Commit", "data/fixtures/untitled_2.blend");

        test_utils::new_branch(tmp_path, "dev");

        // a commit to `dev`
        test_utils::commit(tmp_path, "Commit 2", "data/fixtures/untitled_3.blend");

        let tmp_blend_path = NamedTempFile::new().expect("Cannot create temp file");

        switch_branches(
            tmp_path,
            MAIN_BRANCH_NAME,
            tmp_blend_path.path().to_str().unwrap(),
        )
        .expect("Cannot switch branches");

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        // current branch name is set to the checked out branch
        let current_branch_name = db.read_current_branch_name().unwrap();
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);

        // latest commit hash is set to the tip of the checked out branch
        let latest_commit_hash =
            read_latest_commit_hash_on_branch(&db, &current_branch_name).unwrap();
        insta::assert_debug_snapshot!(latest_commit_hash, @r###""94ab91e7ea864efd6cc228472d47d2a1ca648682ff25cbcb79a9d7a286811fb61d75bee6964aaeec2850f881f8b924dc88b626af405d0ffe813596c4f5033f84""###);

        let main_tip = db.read_branch_tip(MAIN_BRANCH_NAME).unwrap().unwrap();
        assert_eq!(latest_commit_hash, main_tip);
    }
}
