use std::fmt::Display;

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
    fn check_commit_exists(&self, hash: &str) -> Result<bool, DBError>;

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
                    blocks_and_pointers BLOB
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
            "INSERT INTO commits (hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks_and_pointers) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                commit.hash,
                commit.prev_commit_hash,
                commit.project_id,
                commit.branch,
                commit.message,
                commit.author,
                commit.date,
                commit.header,
                commit.blocks_and_pointers
            ),
        )
        .map_err(|e| DBError::Error(format!("Cannot insert commit object: {:?}", e)))?;

        Ok(())
    }

    fn read_commit(&self, hash: &str) -> Result<Option<Commit>, DBError> {
        self.sqlite_db.query_row("SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks_and_pointers FROM commits WHERE hash = ?1", [hash], |row| Ok(Some(Commit {
            hash: row.get(0).expect("No hash found in row"),
            prev_commit_hash: row.get(1).expect("No prev_commit_hash found in row"),
            project_id: row.get(2).expect("No project_id found in row"),
            branch: row.get(3).expect("No branch found in row"),
            message: row.get(4).expect("No message found in row"),
            author: row.get(5).expect("No author found in row"),
            date: row.get(6).expect("No date found in row"),
            header: row.get(7).expect("No header found in row"),
            blocks_and_pointers: row.get(8).expect("No blocks found in row")
        }))).map_err(|e| DBError::Error(format!("Cannot read commit: {:?}", e)))
    }

    fn check_commit_exists(&self, hash: &str) -> Result<bool, DBError> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT hash FROM commits WHERE hash = ?1")
            .map_err(|e| DBError::Error(format!("Cannot create statement: {:?}", e)))?;

        let mut rows = stmt
            .query([hash])
            .map_err(|e| DBError::Error(format!("Cannot query branch: {:?}", e)))?;

        let next = rows
            .next()
            .map_err(|e| DBError::Error(format!("Cannot get next row: {:?}", e)))?;

        Ok(next.is_some())
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
                WITH RECURSIVE descendant_commits(hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks_and_pointers) AS (
                    SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks_and_pointers FROM commits WHERE hash = ?1
                    UNION ALL
                    SELECT c.hash, c.prev_commit_hash, c.project_id, c.branch, c.message, c.author, c.date, c.header, c.blocks_and_pointers FROM commits c
                    JOIN descendant_commits a ON c.prev_commit_hash = a.hash
                )
                SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header, blocks_and_pointers FROM descendant_commits ORDER BY date ASC;
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
                blocks_and_pointers: data.get(8).expect("No blocks found in row"),
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
