use anyhow::bail;

use crate::db::db_ops::{DBError, Persistence, DB};

use super::{common::read_latest_commit_hash_on_branch, init_command::MAIN_BRANCH_NAME};

pub fn create_new_branch(db_path: &str, new_branch_name: &str) -> anyhow::Result<()> {
    let mut db = Persistence::open(db_path)?;

    let current_brach_name = db.read_current_branch_name()?;

    if current_brach_name != MAIN_BRANCH_NAME {
        bail!(DBError::Error(
            "New branches can only be created if main is the current branch".to_owned(),
        ));
    }

    let tip = read_latest_commit_hash_on_branch(&db, &current_brach_name)?;

    db.execute_in_transaction(|tx| {
        Persistence::write_branch_tip(tx, new_branch_name, &tip)?;
        Persistence::write_current_branch_name(tx, new_branch_name)?;
        Ok(())
    })?;

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

    use super::create_new_branch;

    #[test]
    fn test_create_new_branch() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        {
            let db = Persistence::open(tmp_path).expect("Cannot open test DB");
            let latest_commit_hash = read_latest_commit_hash_on_branch(&db, MAIN_BRANCH_NAME)
                .expect("Cannot read latest commit");
            insta::assert_debug_snapshot!(latest_commit_hash, @r###""74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d""###);
        }

        create_new_branch(tmp_path, "dev").unwrap();

        assert_eq!(test_utils::list_checkpoints(tmp_path, "dev").len(), 1);

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        let branches = db.read_all_branches().unwrap();
        assert_eq!(branches, vec!["dev", "main"]);

        // the current branch name is updated to the name of the new branch
        assert_eq!(current_branch_name, "dev");

        let latest_commit_hash = read_latest_commit_hash_on_branch(&db, &current_branch_name)
            .expect("Cannot read latest commit");

        // the latest commit hash stays the same
        insta::assert_debug_snapshot!(latest_commit_hash, @r###""74ae7a3e82bc3106ae7c510c7c75f9ec704c96a9d9f2bb2ed889f38ff2c0ead2f349aeb43aba7ddb435c8ba8b2ffdd00406ec41bb3c3b0092e6f5062852c542d""###);
    }

    #[test]
    fn test_commit_to_new_branch() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        // a commit to `main`
        test_utils::commit(tmp_path, "Commit", "data/fixtures/untitled_2.blend");

        create_new_branch(tmp_path, "dev").unwrap();

        // a commit to `dev`
        test_utils::commit(tmp_path, "Commit 2", "data/fixtures/untitled_3.blend");

        let commits = test_utils::list_checkpoints(tmp_path, "dev");

        assert_eq!(commits.len(), 3);

        // latest commit first
        assert_eq!(commits.get(0).unwrap().branch, "dev");
        assert_eq!(commits.get(0).unwrap().message, "Commit 2");
        assert_eq!(commits.get(1).unwrap().branch, "main");
        assert_eq!(commits.get(1).unwrap().message, "Commit");
        assert_eq!(commits.get(2).unwrap().branch, "main");
    }
}
