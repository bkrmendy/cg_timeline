use std::{fmt::Display, path::Path};

use super::structs::{BlockRecord, Commit};

pub struct ShortCommitRecord {
    pub hash: String,
    pub branch: String,
    pub message: String,
}

#[derive(Debug)]
pub enum DBError {
    Fundamental(String), // means that stuff is very wrong
    Consistency(String), // the timeline maybe in an inconsistent state
    Error(String),       // a recoverable error
}

impl Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBError::Fundamental(msg) => write!(f, "Fundamental error: {}", msg),
            DBError::Consistency(msg) => write!(f, "Consistency error: {}", msg),
            DBError::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

pub trait DB: Sized {
    fn open(path: &str) -> Result<Self, DBError>;

    fn write_blocks(tx: &rusqlite::Transaction, blocks: &[BlockRecord]) -> Result<(), DBError>;
    fn read_blocks(&self, hashes: Vec<String>) -> Result<Vec<BlockRecord>, DBError>;

    fn write_commit(tx: &rusqlite::Transaction, commit: Commit) -> Result<(), DBError>;
    fn read_commit(&self, hash: &str) -> Result<Option<Commit>, DBError>;

    fn read_ancestors_of_commit(
        &self,
        starting_from_hash: &str,
    ) -> Result<Vec<ShortCommitRecord>, DBError>;

    fn read_descendants_of_commit(&self, hash: &str) -> Result<Vec<Commit>, DBError>;

    fn read_current_branch_name(&self) -> Result<String, DBError>;
    fn write_current_branch_name(
        tx: &rusqlite::Transaction,
        brach_name: &str,
    ) -> Result<(), DBError>;

    fn read_current_commit_pointer(&self) -> Result<String, DBError>;
    fn write_current_commit_pointer(tx: &rusqlite::Transaction, hash: &str) -> Result<(), DBError>;

    fn read_all_branches(&self) -> Result<Vec<String>, DBError>;

    fn read_branch_tip(&self, branch_name: &str) -> Result<Option<String>, DBError>;
    fn write_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> Result<(), DBError>;

    fn read_remote_branch_tip(&self, branch_name: &str) -> Result<String, DBError>;
    fn write_remote_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> Result<(), DBError>;

    fn read_project_id(&self) -> Result<String, DBError>;
    fn write_project_id(tx: &rusqlite::Transaction, last_mod_time: &str) -> Result<(), DBError>;

    fn read_last_modification_time(&self) -> Result<Option<i64>, DBError>;
    fn write_last_modifiction_time(
        tx: &rusqlite::Transaction,
        last_mod_time: i64,
    ) -> Result<(), DBError>;

    fn read_name(&self) -> Result<Option<String>, DBError>;
    fn write_name(tx: &rusqlite::Transaction, name: &str) -> Result<(), DBError>;

    fn delete_branch_with_commits(
        tx: &rusqlite::Transaction,
        branch_name: &str,
    ) -> Result<(), DBError>;

    fn execute_in_transaction<F>(&mut self, f: F) -> Result<(), DBError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<(), DBError>;
}

pub struct Persistence {
    sqlite_db: rusqlite::Connection,
}

#[inline]
fn last_mod_time_key() -> String {
    "LAST_MOD_TIME".to_string()
}

#[inline]
fn current_branch_name_key() -> String {
    "CURRENT_BRANCH_NAME".to_string()
}

#[inline]
fn current_latest_commit_key() -> String {
    "CURRENT_LATEST_COMMIT".to_string()
}

#[inline]
fn project_id_key() -> String {
    "PROJECT_ID".to_string()
}

#[inline]
fn user_name_key() -> String {
    "USER_NAME".to_string()
}

fn write_config_inner(tx: &rusqlite::Transaction, key: &str, value: &str) -> Result<(), DBError> {
    tx.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        [key, value],
    )
    .map_err(|_| DBError::Error(format!("Cannot set {:?} for {:?}", value, key)))
    .map(|_| ())
}

fn read_config_inner(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>, DBError> {
    let mut stmt = conn
        .prepare("SELECT value FROM config WHERE key = ?1")
        .map_err(|_| DBError::Fundamental("Cannot prepare read commits query".to_owned()))?;

    let mut rows = stmt
        .query([key])
        .map_err(|_| DBError::Fundamental("Cannot query config table".to_owned()))?;

    match rows.next() {
        Ok(Some(row)) => row
            .get(0)
            .map_err(|_| DBError::Fundamental("Cannot read config key".to_owned())),
        _ => Ok(None),
    }
}

impl DB for Persistence {
    fn open(sqlite_path: &str) -> Result<Self, DBError> {
        let sqlite_db = rusqlite::Connection::open(sqlite_path)
            .map_err(|e| DBError::Fundamental(format!("Cannot open SQLite: {:?}", e)))?;

        sqlite_db
            .execute(
                "CREATE TABLE IF NOT EXISTS commits (
                    hash TEXT PRIMARY KEY,
                    prev_commit_hash TEXT,
                    project_id TEXT,
                    branch TEXT,
                    message TEXT,
                    author TEXT,
                    date INTEGER,
                    header BLOB,
                    blocks TEXT
                )",
                [],
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot create commits table: {:?}", e)))?;

        sqlite_db
            .execute(
                "CREATE TABLE IF NOT EXISTS branches (
                    name TEXT PRIMARY KEY,
                    tip TEXT
                )",
                [],
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot create branches table: {:?}", e)))?;

        sqlite_db
            .execute(
                "CREATE TABLE IF NOT EXISTS remote_branches (
                    name TEXT PRIMARY KEY,
                    tip TEXT
                )",
                [],
            )
            .map_err(|e| {
                DBError::Fundamental(format!("Cannot create remote_branches table: {:?}", e))
            })?;

        sqlite_db
            .execute(
                "CREATE TABLE IF NOT EXISTS blocks (
                    key TEXT PRIMARY KEY,
                    value BLOB
                )",
                [],
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot create blocks table: {:?}", e)))?;

        sqlite_db
            .execute(
                "CREATE TABLE IF NOT EXISTS config (
                    key TEXT PRIMARY KEY,
                    value TEXT
                )",
                [],
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot create config table: {:?}", e)))?;

        Ok(Self { sqlite_db })
    }

    fn write_blocks(tx: &rusqlite::Transaction, blocks: &[BlockRecord]) -> Result<(), DBError> {
        let mut stmt = tx
            .prepare("INSERT INTO blocks (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO NOTHING")
            .map_err(|e| DBError::Fundamental(format!("Cannot prepare query: {:?}", e)))?;

        for block in blocks {
            stmt.execute((&block.hash, &block.data))
                .map_err(|e| DBError::Error(format!("Cannot write block: {:?}", e)))?;
        }

        Ok(())
    }

    fn read_blocks(&self, hashes: Vec<String>) -> Result<Vec<BlockRecord>, DBError> {
        let mut result: Vec<BlockRecord> = Vec::new();
        for hash in hashes {
            let block_data = self
                .sqlite_db
                .query_row("SELECT value FROM blocks WHERE key = ?1", [&hash], |row| {
                    Ok(Some(row.get(0).expect("No value in row")))
                })
                .map_err(|e| DBError::Error(format!("Error reading block: {:?}", e)))?
                .ok_or(DBError::Error("No block with hash found".to_owned()))?;

            result.push(BlockRecord {
                hash,
                data: block_data,
            })
        }

        Ok(result)
    }

    fn write_commit(tx: &rusqlite::Transaction, commit: Commit) -> Result<(), DBError> {
        tx.execute(
            "INSERT INTO commits (hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                commit.hash,
                commit.prev_commit_hash,
                commit.project_id,
                commit.branch,
                commit.message,
                commit.author,
                commit.date,
                commit.header,
                commit.blocks
            ),
        )
        .map_err(|e| DBError::Error(format!("Cannot insert commit object: {:?}", e)))?;

        Ok(())
    }

    fn read_commit(&self, hash: &str) -> Result<Option<Commit>, DBError> {
        self.sqlite_db.query_row("SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks FROM commits WHERE hash = ?1", [hash], |row| Ok(Some(Commit {
            hash: row.get(0).expect("No hash found in row"),
            prev_commit_hash: row.get(1).expect("No prev_commit_hash found in row"),
            project_id: row.get(2).expect("No project_id found in row"),
            branch: row.get(3).expect("No branch found in row"),
            message: row.get(4).expect("No message found in row"),
            author: row.get(5).expect("No author found in row"),
            date: row.get(6).expect("No date found in row"),
            header: row.get(7).expect("No header found in row"),
            blocks: row.get(8).expect("No blocks found in row")
        }))).map_err(|e| DBError::Error(format!("Cannot read commit: {:?}", e)))
    }

    fn read_ancestors_of_commit(
        &self,
        starting_from_hash: &str,
    ) -> Result<Vec<ShortCommitRecord>, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare(
                "
                WITH RECURSIVE ancestor_commits(hash, branch, message, prev_commit_hash, date) AS (
                    SELECT hash, branch, message, prev_commit_hash, date FROM commits WHERE hash = ?1
                    UNION ALL
                    SELECT c.hash, c.branch, c.message, c.prev_commit_hash, c.date FROM commits c
                    JOIN ancestor_commits a ON a.prev_commit_hash = c.hash
                )
                SELECT hash, branch, message FROM ancestor_commits ORDER BY date DESC;
                ",
            )
            .map_err(|e| {
                DBError::Fundamental(format!("Cannot prepare read commits query: {:?}", e))
            })?;

        let mut rows = stmt
            .query([starting_from_hash])
            .map_err(|e| DBError::Error(format!("Cannot read commits: {:?}", e)))?;

        let mut result: Vec<ShortCommitRecord> = vec![];
        while let Ok(Some(data)) = rows.next() {
            result.push(ShortCommitRecord {
                hash: data.get(0).expect("cannot get hash"),
                branch: data.get(1).expect("cannot get branch"),
                message: data.get(2).expect("cannot read message"),
            })
        }

        Ok(result)
    }

    fn read_descendants_of_commit(&self, hash: &str) -> Result<Vec<Commit>, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare(
                "
                WITH RECURSIVE descendant_commits(hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks) AS (
                    SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks FROM commits WHERE hash = ?1
                    UNION ALL
                    SELECT c.hash, c.prev_commit_hash, c.project_id, c.branch, c.message, c.author, c.date, c.header, c.blocks FROM commits c
                    JOIN descendant_commits a ON c.prev_commit_hash = a.hash
                )
                SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks FROM descendant_commits ORDER BY date ASC;
                ",
            )
            .map_err(|e| {
                DBError::Fundamental(format!("Cannot prepare read commits query: {:?}", e))
            })?;

        let mut rows = stmt
            .query([hash])
            .map_err(|e| DBError::Error(format!("Cannot read commits: {:?}", e)))?;

        let mut result: Vec<Commit> = vec![];

        while let Ok(Some(data)) = rows.next() {
            let hash: String = data
                .get::<usize, String>(0)
                .expect("No hash found in row")
                .to_string();

            result.push(Commit {
                hash,
                prev_commit_hash: data.get(1).expect("No prev_commit_hash found in row"),
                project_id: data.get(2).expect("No project_id found in row"),
                branch: data.get(3).expect("No branch found in row"),
                message: data.get(4).expect("No message found in row"),
                author: data.get(5).expect("No author found in row"),
                date: data.get(6).expect("No date found in row"),
                header: data.get(7).expect("No header found in row"),
                blocks: data.get(8).expect("No blocks found in row"),
            })
        }

        Ok(result)
    }

    fn read_current_branch_name(&self) -> Result<String, DBError> {
        read_config_inner(&self.sqlite_db, &current_branch_name_key())
            .map_err(|_| DBError::Error("Cannot read current branch name".to_owned()))
            .and_then(|v| {
                v.map_or(
                    Err(DBError::Consistency(
                        "Cannot read current branch name".to_owned(),
                    )),
                    Ok,
                )
            })
    }

    fn write_current_branch_name(
        tx: &rusqlite::Transaction,
        brach_name: &str,
    ) -> Result<(), DBError> {
        write_config_inner(tx, &current_branch_name_key(), brach_name)
    }

    fn read_all_branches(&self) -> Result<Vec<String>, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT name FROM branches")
            .map_err(|e| DBError::Error(format!("Cannot query branches: {:?}", e)))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| DBError::Error(format!("Cannot query branches: {:?}", e)))?;

        let mut result: Vec<String> = vec![];

        while let Ok(Some(data)) = rows.next() {
            let name = data.get(0).map_err(|e| {
                DBError::Fundamental(format!("Branch name not returned in result set: {:?}", e))
            })?;

            result.push(name);
        }

        Ok(result)
    }

    fn read_branch_tip(&self, branch_name: &str) -> Result<Option<String>, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT tip FROM branches WHERE name = ?1")
            .map_err(|e| DBError::Error(format!("Cannot query branch: {:?}", e)))?;

        let mut rows = stmt
            .query([branch_name])
            .map_err(|e| DBError::Error(format!("Cannot query branch: {:?}", e)))?;

        let row = rows.next();

        if let Ok(Some(data)) = row {
            Ok(Some(data.get(0).unwrap()))
        } else if let Ok(None) = row {
            Ok(None)
        } else {
            Err(DBError::Error("Cannot query branch".to_owned()))
        }
    }

    fn write_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> Result<(), DBError> {
        tx.execute(
            "INSERT OR REPLACE INTO branches (name, tip) VALUES (?1, ?2)",
            [&brach_name, &tip],
        )
        .map_err(|e| {
            DBError::Error(format!(
                "Cannot create new branch {:?}: {:?}",
                brach_name, e
            ))
        })
        .map(|_| ())
    }

    fn read_remote_branch_tip(&self, branch_name: &str) -> Result<String, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT tip FROM remote_branches WHERE name = ?1")
            .map_err(|e| DBError::Error(format!("Cannot query branch: {:?}", e)))?;

        let mut rows = stmt
            .query([branch_name])
            .map_err(|e| DBError::Error(format!("Cannot query branch: {:?}", e)))?;

        let row = rows.next();

        if let Ok(Some(data)) = row {
            Ok(data.get(0).unwrap())
        } else if let Ok(None) = row {
            Err(DBError::Consistency(format!(
                "No remote branch tip exists for {:?}",
                branch_name
            )))
        } else {
            Err(DBError::Error("Cannot query branch".to_owned()))
        }
    }

    fn write_remote_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> Result<(), DBError> {
        tx.execute(
            "INSERT OR REPLACE INTO remote_branches (name, tip) VALUES (?1, ?2)",
            [&brach_name, &tip],
        )
        .map_err(|e| {
            DBError::Error(format!(
                "Cannot create new branch {:?}: {:?}",
                brach_name, e
            ))
        })
        .map(|_| ())
    }

    fn read_current_commit_pointer(&self) -> Result<String, DBError> {
        read_config_inner(&self.sqlite_db, &current_latest_commit_key())
            .map_err(|_| DBError::Error("Cannot read current commit pointer".to_owned()))
            .and_then(|v| {
                v.ok_or(DBError::Consistency(
                    "Current commit pointer not set".to_owned(),
                ))
            })
    }

    fn write_current_commit_pointer(tx: &rusqlite::Transaction, hash: &str) -> Result<(), DBError> {
        write_config_inner(tx, &current_latest_commit_key(), hash)
            .map_err(|e| DBError::Error(format!("Cannot write latest commit hash: {:?}", e)))
    }

    fn execute_in_transaction<F>(&mut self, f: F) -> Result<(), DBError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<(), DBError>,
    {
        let tx = self
            .sqlite_db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(|_| DBError::Fundamental("Cannot create transaction".to_owned()))?;

        f(&tx)?;

        tx.commit()
            .map_err(|_| DBError::Fundamental("Cannot commit transaction".to_owned()))
    }

    fn read_project_id(&self) -> Result<String, DBError> {
        read_config_inner(&self.sqlite_db, &project_id_key())
            .map_err(|_| DBError::Error("Cannot read project id".to_owned()))
            .and_then(|v| {
                v.map_or(
                    Err(DBError::Fundamental(
                        "Current project key not set".to_owned(),
                    )),
                    Ok,
                )
            })
    }

    fn write_project_id(tx: &rusqlite::Transaction, project_id: &str) -> Result<(), DBError> {
        write_config_inner(tx, &project_id_key(), project_id)
    }

    fn delete_branch_with_commits(
        tx: &rusqlite::Transaction,
        branch_name: &str,
    ) -> Result<(), DBError> {
        let mut delete_commits_stmt = tx
            .prepare(
                "
            DELETE FROM commits WHERE branch = ?1;
            ",
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot prepare query: {:?}", e)))?;

        let mut delete_branch_stmt = tx
            .prepare(
                "
            DELETE FROM branches WHERE name = ?1;
            ",
            )
            .map_err(|e| DBError::Fundamental(format!("Cannot prepare query: {:?}", e)))?;

        delete_commits_stmt
            .execute([branch_name])
            .map_err(|e| DBError::Error(format!("Cannot execute statement: {:?}", e)))?;

        delete_branch_stmt
            .execute([branch_name])
            .map_err(|e| DBError::Error(format!("Cannot execute statement: {:?}", e)))?;

        Ok(())
    }

    fn read_name(&self) -> Result<Option<String>, DBError> {
        read_config_inner(&self.sqlite_db, &user_name_key())
    }

    fn write_name(tx: &rusqlite::Transaction, name: &str) -> Result<(), DBError> {
        write_config_inner(tx, &user_name_key(), name)
    }

    fn read_last_modification_time(&self) -> Result<Option<i64>, DBError> {
        let raw = read_config_inner(&self.sqlite_db, &last_mod_time_key())?;
        if let Some(raw) = raw {
            return raw
                .parse::<i64>()
                .map(Some)
                .map_err(|e| DBError::Error(format!("Cannot parse timestamp from string: {}", e)));
        }

        Ok(None)
    }

    fn write_last_modifiction_time(
        tx: &rusqlite::Transaction,
        last_mod_time: i64,
    ) -> Result<(), DBError> {
        write_config_inner(tx, &last_mod_time_key(), &last_mod_time.to_string())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::api::{
        init_command::{INITIAL_COMMIT_HASH, MAIN_BRANCH_NAME},
        test_utils::{init_db_from_simple_timeline, SimpleCommit, SimpleTimeline},
    };

    use super::*;

    #[test]
    fn test_read_descendants_of_commit() {
        let tmp_dir = TempDir::new().expect("Cannot create temp dir");
        let tmp_path = tmp_dir.path().to_str().expect("Cannot get temp dir path");

        let mut db = Persistence::open(tmp_path).expect("Cannot open test DB");

        /*
                    x
                  /
          1 - 2 - 3 - 4
           \
              a - b
        */
        db.execute_in_transaction(|tx| {
            Persistence::write_commit(
                tx,
                Commit {
                    hash: "1".to_owned(),
                    prev_commit_hash: String::from(INITIAL_COMMIT_HASH),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 1,
                    header: vec![],
                    blocks: "aaa".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "2".to_owned(),
                    prev_commit_hash: "1".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 2,
                    header: vec![],
                    blocks: "bbb".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "3".to_owned(),
                    prev_commit_hash: "2".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 3,
                    header: vec![],
                    blocks: "ccc".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "4".to_owned(),
                    prev_commit_hash: "3".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 4,
                    header: vec![],
                    blocks: "ddd".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "a".to_owned(),
                    prev_commit_hash: "1".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 10,
                    header: vec![],
                    blocks: "eee".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "b".to_owned(),
                    prev_commit_hash: "a".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 11,
                    header: vec![],
                    blocks: "fff".to_owned(),
                },
            )?;

            Persistence::write_commit(
                tx,
                Commit {
                    hash: "x".to_owned(),
                    prev_commit_hash: "3".to_owned(),
                    project_id: "a".to_owned(),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: "hi".to_owned(),
                    author: "test".to_owned(),
                    date: 10,
                    header: vec![],
                    blocks: "xxx".to_owned(),
                },
            )?;

            Ok(())
        })
        .expect("Cannot execute transaction");

        // Check the diagram above
        {
            // Descendants of 1
            let commits = db
                .read_descendants_of_commit("1")
                .expect("Cannot read commits");

            let hashes: Vec<String> = commits.iter().map(|c| c.hash.clone()).collect();

            assert_eq!(hashes, vec!["1", "2", "3", "4", "a", "x", "b"]);
        }

        {
            // Descendants of a
            let commits = db
                .read_descendants_of_commit("a")
                .expect("Cannot read commits");

            let hashes: Vec<String> = commits.iter().map(|c| c.hash.clone()).collect();

            assert_eq!(hashes, vec!["a", "b"]);
        }

        {
            // Descendants of 2
            let commits = db
                .read_descendants_of_commit("2")
                .expect("Cannot read commits");

            let hashes: Vec<String> = commits.iter().map(|c| c.hash.clone()).collect();

            assert_eq!(hashes, vec!["2", "3", "4", "x"]);
        }

        {
            // Descendants of 3
            let hashes: Vec<String> = db
                .read_descendants_of_commit("3")
                .expect("Cannot read commits")
                .into_iter()
                .map(|c| c.hash)
                .collect();

            assert_eq!(hashes, vec!["3", "4", "x"]);
        }

        {
            // Descendants of x
            let commits = db
                .read_descendants_of_commit("x")
                .expect("Cannot read commits");

            let hashes: Vec<String> = commits.iter().map(|c| c.hash.clone()).collect();

            assert_eq!(hashes, vec!["x"]);
        }
    }

    #[test]
    fn test_delete_branch_with_commits() {
        let tmp_dir = TempDir::new().expect("Cannot create temp dir");
        let tmp_path = tmp_dir.path().to_str().expect("Cannot get temp dir path");

        /*
          alt1           x
                        /
          main 1 - 2 - 3 - 4
                \
          alt2    a - b
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

        let mut db = Persistence::open(tmp_path).expect("Cannot open test DB");

        db.execute_in_transaction(|tx| Persistence::delete_branch_with_commits(tx, "alt2"))
            .expect("cannot delete");

        let branches = db.read_all_branches().expect("Cannot read branches");
        assert_eq!(branches, vec!["alt1", "main"]);

        let commits: Vec<String> = db
            .read_descendants_of_commit("1")
            .expect("cannot read descendants of commit")
            .into_iter()
            .map(|c| c.hash)
            .collect();

        assert_eq!(commits, vec!["1", "2", "3", "4", "x"]);
    }
}
