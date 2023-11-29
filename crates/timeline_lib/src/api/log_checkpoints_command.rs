use crate::db::db_ops::{DBError, Persistence, ShortCommitRecord, DB};

pub fn list_checkpoints(
    db_path: &str,
    branch_name: &str,
) -> Result<Vec<ShortCommitRecord>, DBError> {
    let conn = Persistence::open(db_path)?;
    let tip = conn
        .read_branch_tip(branch_name)?
        .ok_or(DBError::Consistency(format!(
            "Cannot read tip for branch {}",
            branch_name
        )))?;
    conn.read_ancestors_of_commit(&tip)
}

#[cfg(test)]
mod test {

    use tempfile::NamedTempFile;

    use crate::api::{
        init_command::INITIAL_COMMIT_HASH,
        log_checkpoints_command::list_checkpoints,
        test_utils::{init_db_from_simple_timeline, SimpleCommit, SimpleTimeline},
    };

    #[test]
    fn test_list_checkpoints() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        /*
        alt1           x
                      /
        main  1 - 2 - 3 - 4
               \
        alt2     a - b
        */

        init_db_from_simple_timeline(
            tmp_path,
            SimpleTimeline {
                project_id: String::from("a"),
                author: "test".to_owned(),
                blocks: vec![
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    String::from("fff"),
                    String::from("ggg"),
                    String::from("111"),
                    String::from("222"),
                ],
                commits: vec![
                    SimpleCommit {
                        hash: "1".to_owned(),
                        prev_hash: String::from(INITIAL_COMMIT_HASH),
                        branch: String::from("main"),
                        message: "hi".to_owned(),
                        blocks: "aaa,bbb".to_owned(),
                    },
                    SimpleCommit {
                        hash: "2".to_owned(),
                        prev_hash: "1".to_owned(),
                        branch: String::from("main"),
                        message: "hi".to_owned(),
                        blocks: "bbb,ccc".to_owned(),
                    },
                    SimpleCommit {
                        hash: "3".to_owned(),
                        prev_hash: "2".to_owned(),
                        branch: String::from("main"),
                        message: "hi".to_owned(),
                        blocks: "ccc,ddd".to_owned(),
                    },
                    SimpleCommit {
                        hash: "4".to_owned(),
                        prev_hash: "3".to_owned(),
                        branch: String::from("main"),
                        message: "hi".to_owned(),
                        blocks: "ddd,eee".to_owned(),
                    },
                    SimpleCommit {
                        hash: "a".to_owned(),
                        prev_hash: "1".to_owned(),
                        branch: String::from("alt2"),
                        message: "hi".to_owned(),
                        blocks: "eee,fff".to_owned(),
                    },
                    SimpleCommit {
                        hash: "b".to_owned(),
                        prev_hash: "a".to_owned(),
                        branch: String::from("alt2"),
                        message: "hi".to_owned(),
                        blocks: "fff,111".to_owned(),
                    },
                    SimpleCommit {
                        hash: "x".to_owned(),
                        prev_hash: "3".to_owned(),
                        branch: String::from("alt1"),
                        message: "hi".to_owned(),
                        blocks: "222,aaa".to_owned(),
                    },
                ],
            },
        );

        let desc_of_4: Vec<String> = list_checkpoints(tmp_path, "main")
            .expect("Cannot list checkpoints of main")
            .into_iter()
            .map(|c| c.hash)
            .collect();

        assert_eq!(desc_of_4, vec!["4", "3", "2", "1"]);

        let desc_of_b: Vec<String> = list_checkpoints(tmp_path, "alt2")
            .expect("Cannot list checkpoints of alt2")
            .into_iter()
            .map(|c| c.hash)
            .collect();

        assert_eq!(desc_of_b, vec!["b", "a", "1"]);

        let desc_of_b: Vec<String> = list_checkpoints(tmp_path, "alt1")
            .expect("Cannot list checkpoints of alt1")
            .into_iter()
            .map(|c| c.hash)
            .collect();

        assert_eq!(desc_of_b, vec!["x", "3", "2", "1"]);
    }
}
