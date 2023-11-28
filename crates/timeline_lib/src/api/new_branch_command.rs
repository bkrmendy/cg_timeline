use crate::db::db_ops::{DBError, Persistence, DB};

use super::{common::read_latest_commit_hash_on_branch, init_command::MAIN_BRANCH_NAME};

pub fn create_new_branch(db_path: &str, new_branch_name: &str) -> Result<(), DBError> {
    let mut db = Persistence::open(db_path)?;

    let current_brach_name = db.read_current_branch_name()?;

    if current_brach_name != MAIN_BRANCH_NAME {
        return Err(DBError::Error(
            "New branches can only be created if main is the current branch".to_owned(),
        ));
    }

    let tip = read_latest_commit_hash_on_branch(&db, &current_brach_name)?;

    db.execute_in_transaction(|tx| {
        Persistence::write_branch_tip(tx, new_branch_name, &tip)?;
        Persistence::write_remote_branch_tip(tx, new_branch_name, &tip)?;

        Persistence::write_current_branch_name(tx, new_branch_name)?;
        Ok(())
    })?;

    Ok(())
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;

    use crate::{
        api::{common::read_latest_commit_hash_on_branch, test_utils},
        db::db_ops::{Persistence, DB},
    };

    use super::create_new_branch;

    #[test]
    fn test_create_new_branch() {
        let tmp_dir = TempDir::new().expect("Cannot create temp dir");
        let tmp_db_path = tmp_dir.path().to_str().expect("Cannot get temp dir path");

        test_utils::init_db_from_file(
            tmp_db_path,
            "my-cool-project",
            "data/fixtures/untitled.blend",
        );

        create_new_branch(tmp_db_path, "dev").unwrap();

        assert_eq!(test_utils::list_checkpoints(tmp_db_path, "dev").len(), 1);

        let db = Persistence::open(tmp_db_path).expect("Cannot open test DB");

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        let branches = db.read_all_branches().unwrap();
        assert_eq!(branches, vec!["dev", "main"]);

        // the current branch name is updated to the name of the new branch
        assert_eq!(current_branch_name, "dev");

        let latest_commit_name = read_latest_commit_hash_on_branch(&db, &current_branch_name)
            .expect("Cannot read latest commit");

        // the latest commit hash stays the same
        assert_eq!(latest_commit_name, "5bdd30ea8c1523bc75eddbcb1e59e4c7");
    }

    #[test]
    fn test_commit_to_new_branch() {
        let tmp_dir = TempDir::new().expect("Cannot create temp dir");
        let tmp_db_path = tmp_dir.path().to_str().expect("Cannot get temp dir path");

        test_utils::init_db_from_file(
            tmp_db_path,
            "my-cool-project",
            "data/fixtures/untitled.blend",
        );

        // a commit to `main`
        test_utils::commit(tmp_db_path, "Commit", "data/fixtures/untitled_2.blend");

        create_new_branch(tmp_db_path, "dev").unwrap();

        // a commit to `dev`
        test_utils::commit(tmp_db_path, "Commit 2", "data/fixtures/untitled_3.blend");

        let commits = test_utils::list_checkpoints(tmp_db_path, "dev");

        assert_eq!(commits.len(), 3);

        // latest commit first
        assert_eq!(commits.get(0).unwrap().branch, "dev");
        assert_eq!(commits.get(0).unwrap().message, "Commit 2");
        assert_eq!(commits.get(1).unwrap().branch, "main");
        assert_eq!(commits.get(1).unwrap().message, "Commit");
        assert_eq!(commits.get(2).unwrap().branch, "main");
    }
}
