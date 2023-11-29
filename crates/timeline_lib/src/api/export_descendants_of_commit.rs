use std::collections::HashSet;

use crate::{
    db::db_ops::{DBError, Persistence, DB},
    exchange::structs::Exchange,
};

use super::common::parse_hash_list;

pub fn export_descendants_of_commit(
    db_path: &str,
    starting_from_commit_hash: &str,
) -> Result<Exchange, DBError> {
    let db = Persistence::open(db_path)?;
    let commits = db.read_descendants_of_commit(starting_from_commit_hash)?;
    let mut block_hashes: HashSet<String> = HashSet::new();

    for commit in commits.iter() {
        let blocks_of_this_commit = parse_hash_list(commit.blocks.clone());

        for block in blocks_of_this_commit.into_iter() {
            if !block_hashes.contains(&block) {
                block_hashes.insert(block);
            }
        }
    }

    let blocks = db.read_blocks(block_hashes.into_iter().collect())?;

    Ok(Exchange { commits, blocks })
}

#[cfg(test)]
mod test {
    use tempfile::NamedTempFile;

    use crate::api::{
        init_command::{INITIAL_COMMIT_HASH, MAIN_BRANCH_NAME},
        test_utils::{init_db_from_simple_timeline, SimpleCommit, SimpleTimeline},
    };

    use super::export_descendants_of_commit;

    #[test]
    fn test_export_exchange() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        /*
                    x
                  /
          1 - 2 - 3 - 4
           \
              a - b
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
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "aaa,bbb".to_owned(),
                    },
                    SimpleCommit {
                        hash: "2".to_owned(),
                        prev_hash: "1".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "bbb,ccc".to_owned(),
                    },
                    SimpleCommit {
                        hash: "3".to_owned(),
                        prev_hash: "2".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "ccc,ddd".to_owned(),
                    },
                    SimpleCommit {
                        hash: "4".to_owned(),
                        prev_hash: "3".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "ddd,eee".to_owned(),
                    },
                    SimpleCommit {
                        hash: "a".to_owned(),
                        prev_hash: "1".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "eee,fff".to_owned(),
                    },
                    SimpleCommit {
                        hash: "b".to_owned(),
                        prev_hash: "a".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "fff,111".to_owned(),
                    },
                    SimpleCommit {
                        hash: "x".to_owned(),
                        prev_hash: "3".to_owned(),
                        branch: String::from(MAIN_BRANCH_NAME),
                        message: "hi".to_owned(),
                        blocks: "222,aaa".to_owned(),
                    },
                ],
            },
        );

        let exchange = export_descendants_of_commit(tmp_path, "3").expect("Cannot export commits");
        assert_eq!(exchange.commits.len(), 3);
        assert_eq!(exchange.blocks.len(), 5);
        assert_eq!(exchange.commits.get(0).unwrap().hash, "3");
        assert_eq!(exchange.commits.get(1).unwrap().hash, "4");
        assert_eq!(exchange.commits.get(2).unwrap().hash, "x");

        let mut hashes: Vec<String> = exchange.blocks.into_iter().map(|b| b.hash).collect();
        hashes.sort();

        assert_eq!(hashes, vec!["222", "aaa", "ccc", "ddd", "eee"]);
    }
}
