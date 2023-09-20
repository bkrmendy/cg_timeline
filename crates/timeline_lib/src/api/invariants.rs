// pub fn check_at_least_one_commit(conn: &Persistence) -> Result<(), DBError> {
//     let latest_commit_hash = read_latest_commit_hash_on_branch(conn, MAIN_BRANCH_NAME)?;
//     if latest_commit_hash == INITIAL_COMMIT_HASH {
//         return Err(DBError::Consistency(String::from(
//             "Cannot perform operation with no commits",
//         )));
//     }

//     let main_commits = conn.read_ancestors_of_commit(&latest_commit_hash)?;
//     if main_commits.is_empty() {
//         // tbh a bigger problem
//         return Err(DBError::Consistency(String::from("No commits on main")));
//     }

//     Ok(())
// }
