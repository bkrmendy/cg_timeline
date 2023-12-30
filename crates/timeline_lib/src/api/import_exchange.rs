use std::collections::HashMap;

use crate::{
    db::db_ops::{DBError, Persistence, DB},
    exchange::structs::Exchange,
};

// TODO: rename conflicting commits
pub fn import_exchange(db_path: &str, exchange: Exchange) -> Result<(), DBError> {
    let mut branches_to_commits: HashMap<String, String> = HashMap::new();

    for commit in &exchange.commits {
        branches_to_commits.insert(commit.branch.clone(), commit.hash.clone());
    }

    let mut db = Persistence::open(db_path)?;

    db.execute_in_transaction(|tx| {
        Persistence::write_blocks(tx, &exchange.blocks)?;
        for commit in exchange.commits.into_iter() {
            Persistence::write_commit(tx, commit)?;
        }

        Ok(())
    })?;

    let mut branches_to_tips: HashMap<String, String> = HashMap::new();

    for (branch, commit_hash) in branches_to_commits.into_iter() {
        let tip = db
            .read_descendants_of_commit(&commit_hash)?
            .last()
            .map(|c| c.hash.clone())
            .unwrap_or(commit_hash);
        branches_to_tips.insert(branch, tip);
    }

    db.execute_in_transaction(|tx| {
        for (branch, tip) in branches_to_tips.into_iter() {
            Persistence::write_branch_tip(tx, &branch, &tip)?;
        }

        Ok(())
    })
}

/*
test
- commits have to be present
- blocks have to be present
- branches have to be updated
 */

#[cfg(test)]
mod test {

    use tempfile::NamedTempFile;

    use crate::{
        api::{
            init_command::{INITIAL_COMMIT_HASH, MAIN_BRANCH_NAME},
            test_utils::{init_db_from_simple_timeline, SimpleCommit, SimpleTimeline},
        },
        db::{
            db_ops::{Persistence, DB},
            structs::{BlockRecord, Commit},
        },
        exchange::structs::Exchange,
    };

    use super::import_exchange;

    // #[ignore]
    // #[test]
    // fn test_import_exchange() {
    //     let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
    //     let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

    //     /*
    //     Start:
    //       1 - 2 - 3

    //     Result:
    //                 x
    //               /
    //       1 - 2 - 3 - 4
    //        \
    //           a - b
    //     */

    //     init_db_from_simple_timeline(
    //         tmp_path,
    //         SimpleTimeline {
    //             project_id: String::from("a"),
    //             author: "test".to_owned(),
    //             blocks: vec![
    //                 String::from("aaa"),
    //                 String::from("bbb"),
    //                 String::from("ccc"),
    //                 String::from("ddd"),
    //             ],
    //             commits: vec![
    //                 SimpleCommit {
    //                     hash: "1".to_owned(),
    //                     prev_hash: String::from(INITIAL_COMMIT_HASH),
    //                     branch: String::from(MAIN_BRANCH_NAME),
    //                     message: "hi".to_owned(),
    //                     blocks: "aaa,bbb".to_owned(),
    //                 },
    //                 SimpleCommit {
    //                     hash: "2".to_owned(),
    //                     prev_hash: "1".to_owned(),
    //                     branch: String::from(MAIN_BRANCH_NAME),
    //                     message: "hi".to_owned(),
    //                     blocks: "bbb,ccc".to_owned(),
    //                 },
    //                 SimpleCommit {
    //                     hash: "3".to_owned(),
    //                     prev_hash: "2".to_owned(),
    //                     branch: String::from(MAIN_BRANCH_NAME),
    //                     message: "hi".to_owned(),
    //                     blocks: "ccc,ddd".to_owned(),
    //                 },
    //             ],
    //         },
    //     );

    //     let exchange = Exchange {
    //         commits: vec![
    //             Commit {
    //                 hash: "4".to_owned(),
    //                 prev_commit_hash: "3".to_owned(),
    //                 project_id: "a".to_owned(),
    //                 branch: String::from(MAIN_BRANCH_NAME),
    //                 message: "hi".to_owned(),
    //                 author: "test".to_owned(),
    //                 date: 4,
    //                 header: vec![],
    //                 blocks: "ddd,eee".to_owned(),
    //             },
    //             Commit {
    //                 hash: "a".to_owned(),
    //                 prev_commit_hash: "1".to_owned(),
    //                 project_id: "a".to_owned(),
    //                 branch: "ab".to_owned(),
    //                 message: "hi".to_owned(),
    //                 author: "test".to_owned(),
    //                 date: 10,
    //                 header: vec![],
    //                 blocks: "eee,fff".to_owned(),
    //             },
    //             Commit {
    //                 hash: "b".to_owned(),
    //                 prev_commit_hash: "a".to_owned(),
    //                 project_id: "a".to_owned(),
    //                 branch: "ab".to_owned(),
    //                 message: "hi".to_owned(),
    //                 author: "test".to_owned(),
    //                 date: 11,
    //                 header: vec![],
    //                 blocks: "fff,111".to_owned(),
    //             },
    //             Commit {
    //                 hash: "x".to_owned(),
    //                 prev_commit_hash: "3".to_owned(),
    //                 project_id: "a".to_owned(),
    //                 branch: "xs".to_owned(),
    //                 message: "hi".to_owned(),
    //                 author: "test".to_owned(),
    //                 date: 10,
    //                 header: vec![],
    //                 blocks: "222,aaa".to_owned(),
    //             },
    //         ],
    //         blocks: vec![
    //             BlockRecord {
    //                 hash: String::from("aaa"),
    //                 data: vec![1, 1, 1],
    //             },
    //             BlockRecord {
    //                 hash: String::from("ddd"),
    //                 data: vec![4, 4, 4],
    //             },
    //             BlockRecord {
    //                 hash: String::from("eee"),
    //                 data: vec![5, 5, 5],
    //             },
    //             BlockRecord {
    //                 hash: String::from("fff"),
    //                 data: vec![6, 6, 6],
    //             },
    //             BlockRecord {
    //                 hash: String::from("111"),
    //                 data: vec![7, 7, 7],
    //             },
    //             BlockRecord {
    //                 hash: String::from("222"),
    //                 data: vec![7, 7, 7],
    //             },
    //         ],
    //     };

    //     import_exchange(tmp_path, exchange).expect("Cannot import exchange");
    //     let db = Persistence::open(tmp_path).expect("Cannot open test DB");

    //     let all_commits = db
    //         .read_descendants_of_commit("1")
    //         .expect("Cannot read descendants of 1");

    //     let all_commit_hashes: Vec<String> = all_commits.iter().map(|c| c.hash.clone()).collect();

    //     assert_eq!(all_commit_hashes, vec!["4", "a", "x", "b", "1", "2", "3"]); // all commits are present

    //     let mut all_branches: Vec<String> = all_commits.iter().map(|c| c.branch.clone()).collect();

    //     // poor man's `unique`
    //     all_branches.sort_unstable();
    //     all_branches.dedup();

    //     assert_eq!(all_branches, vec!["ab", "main", "xs"]);

    //     let ab_tip = db
    //         .read_branch_tip("ab")
    //         .expect("Cannot read tip for branch 'ab'")
    //         .expect("Branch 'ab' should have a tip");
    //     assert_eq!(ab_tip, "b");

    //     let main_tip = db
    //         .read_branch_tip(MAIN_BRANCH_NAME)
    //         .expect("Cannot read tip for branch 'main'")
    //         .expect("Branch 'main' should have a tip");
    //     assert_eq!(main_tip, "4");

    //     let main_tip = db
    //         .read_branch_tip("xs")
    //         .expect("Cannot read tip for branch 'xs'")
    //         .expect("Branch 'xs' should have a tip");
    //     assert_eq!(main_tip, "x");
    // }
}
