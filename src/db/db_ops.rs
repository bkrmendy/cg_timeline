use anyhow::{bail, Context};
use std::{error::Error, fmt::Display};

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

impl Error for DBError {}

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
    fn open(path: &str) -> anyhow::Result<Self>;

    fn write_blocks(tx: &rusqlite::Transaction, blocks: &[BlockRecord]) -> anyhow::Result<()>;
    fn read_blocks(&self, hashes: Vec<String>) -> anyhow::Result<Vec<BlockRecord>>;

    fn write_commit(tx: &rusqlite::Transaction, commit: Commit) -> anyhow::Result<()>;
    fn read_commit(&self, hash: &str) -> anyhow::Result<Option<Commit>>;
    fn check_commit_exists(&self, hash: &str) -> anyhow::Result<bool>;

    fn read_ancestors_of_commit(
        &self,
        starting_from_hash: &str,
    ) -> anyhow::Result<Vec<ShortCommitRecord>>;

    fn read_descendants_of_commit(&self, hash: &str) -> anyhow::Result<Vec<Commit>>;

    fn read_current_branch_name(&self) -> anyhow::Result<String>;
    fn write_current_branch_name(
        tx: &rusqlite::Transaction,
        brach_name: &str,
    ) -> anyhow::Result<()>;

    fn read_current_commit_pointer(&self) -> anyhow::Result<String>;
    fn write_current_commit_pointer(tx: &rusqlite::Transaction, hash: &str) -> anyhow::Result<()>;

    fn read_all_branches(&self) -> anyhow::Result<Vec<String>>;

    fn read_branch_tip(&self, branch_name: &str) -> anyhow::Result<Option<String>>;
    fn write_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> anyhow::Result<()>;

    fn read_project_id(&self) -> anyhow::Result<String>;
    fn write_project_id(tx: &rusqlite::Transaction, last_mod_time: &str) -> anyhow::Result<()>;

    fn read_last_modification_time(&self) -> anyhow::Result<Option<i64>>;
    fn write_last_modifiction_time(
        tx: &rusqlite::Transaction,
        last_mod_time: i64,
    ) -> anyhow::Result<()>;

    fn read_name(&self) -> anyhow::Result<Option<String>>;
    fn write_name(tx: &rusqlite::Transaction, name: &str) -> anyhow::Result<()>;

    fn delete_branch_with_commits(
        tx: &rusqlite::Transaction,
        branch_name: &str,
    ) -> anyhow::Result<()>;

    fn execute_in_transaction<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&rusqlite::Transaction) -> anyhow::Result<()>;
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

fn write_config_inner(tx: &rusqlite::Transaction, key: &str, value: &str) -> anyhow::Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        [key, value],
    )
    .context(format!("Cannot set {:?} for {:?}", value, key))
    .map(|_| ())
}

fn read_config_inner(conn: &rusqlite::Connection, key: &str) -> anyhow::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;

    let mut rows = stmt.query([key])?;
    let row = rows.next()?;

    match row {
        Some(r) => {
            let value = r.get(0)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

impl DB for Persistence {
    fn open(sqlite_path: &str) -> anyhow::Result<Self> {
        let sqlite_db = rusqlite::Connection::open(sqlite_path)?;

        sqlite_db.execute(
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
        )?;

        sqlite_db.execute(
            "CREATE TABLE IF NOT EXISTS branches (
                    name TEXT PRIMARY KEY,
                    tip TEXT
                )",
            [],
        )?;

        sqlite_db.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                    key TEXT PRIMARY KEY,
                    value BLOB
                )",
            [],
        )?;

        sqlite_db.execute(
            "CREATE TABLE IF NOT EXISTS config (
                    key TEXT PRIMARY KEY,
                    value TEXT
                )",
            [],
        )?;

        Ok(Self { sqlite_db })
    }

    fn write_blocks(tx: &rusqlite::Transaction, blocks: &[BlockRecord]) -> anyhow::Result<()> {
        let mut stmt = tx.prepare(
            "INSERT INTO blocks (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO NOTHING",
        )?;

        for block in blocks {
            stmt.execute((&block.hash, &block.data))?;
        }

        Ok(())
    }

    fn read_blocks(&self, hashes: Vec<String>) -> anyhow::Result<Vec<BlockRecord>> {
        let mut result: Vec<BlockRecord> = Vec::new();
        for hash in hashes {
            let block_data = self.sqlite_db.query_row(
                "SELECT value FROM blocks WHERE key = ?1",
                [&hash],
                |row| Ok(Some(row.get(0).expect("No value in row"))),
            )?;
            if block_data.is_none() {
                bail!(DBError::Error("No block with hash found".to_owned()))
            } else {
                result.push(BlockRecord {
                    hash,
                    data: block_data.unwrap(),
                })
            }
        }

        Ok(result)
    }

    fn write_commit(tx: &rusqlite::Transaction, commit: Commit) -> anyhow::Result<()> {
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
        ).context("Cannot insert commit object")?;

        Ok(())
    }

    fn read_commit(&self, hash: &str) -> anyhow::Result<Option<Commit>> {
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
        }))).context("Cannot read commit")
    }

    fn check_commit_exists(&self, hash: &str) -> anyhow::Result<bool> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT hash FROM commits WHERE hash = ?1")
            .context("Cannot create statement")?;

        let mut rows = stmt.query([hash]).context("Cannot query branch")?;

        let next = rows.next()?;

        Ok(next.is_some())
    }

    fn read_ancestors_of_commit(
        &self,
        starting_from_hash: &str,
    ) -> anyhow::Result<Vec<ShortCommitRecord>> {
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
            .context("Cannot prepare read commits query")?;

        let mut rows = stmt
            .query([starting_from_hash])
            .context("Cannot read commits")?;

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

    fn read_descendants_of_commit(&self, hash: &str) -> anyhow::Result<Vec<Commit>> {
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
            .context("Cannot prepare read commits query")?;

        let mut rows = stmt.query([hash]).context("Cannot read commits")?;

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

    fn read_current_branch_name(&self) -> anyhow::Result<String> {
        let current_branch_name = read_config_inner(&self.sqlite_db, &current_branch_name_key())?;
        if let Some(current_branch_name) = current_branch_name {
            Ok(current_branch_name)
        } else {
            bail!(DBError::Consistency(
                "Cannot read current branch name".to_owned(),
            ))
        }
    }

    fn write_current_branch_name(
        tx: &rusqlite::Transaction,
        brach_name: &str,
    ) -> anyhow::Result<()> {
        write_config_inner(tx, &current_branch_name_key(), brach_name)
    }

    fn read_all_branches(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT name FROM branches")
            .context("Cannot query branches")?;
        let mut rows = stmt.query([]).context("Cannot query branches")?;

        let mut result: Vec<String> = vec![];

        while let Ok(Some(data)) = rows.next() {
            let name = data.get(0).unwrap();

            result.push(name);
        }

        Ok(result)
    }

    fn read_branch_tip(&self, branch_name: &str) -> anyhow::Result<Option<String>> {
        let mut stmt = self
            .sqlite_db
            .prepare("SELECT tip FROM branches WHERE name = ?1")?;

        let mut rows = stmt.query([branch_name])?;

        let row = rows.next()?;

        if let Some(data) = row {
            Ok(Some(data.get(0).unwrap()))
        } else {
            Ok(None)
        }
    }

    fn write_branch_tip(
        tx: &rusqlite::Transaction,
        brach_name: &str,
        tip: &str,
    ) -> anyhow::Result<()> {
        tx.execute(
            "INSERT OR REPLACE INTO branches (name, tip) VALUES (?1, ?2)",
            [&brach_name, &tip],
        )
        .context(format!("Cannot create new branch {:?}", brach_name,))
        .map(|_| ())
    }

    fn read_current_commit_pointer(&self) -> anyhow::Result<String> {
        read_config_inner(&self.sqlite_db, &current_latest_commit_key()).and_then(|v| {
            if let Some(v) = v {
                Ok(v)
            } else {
                bail!(DBError::Consistency(
                    "Current commit pointer not set".to_owned(),
                ))
            }
        })
    }

    fn write_current_commit_pointer(tx: &rusqlite::Transaction, hash: &str) -> anyhow::Result<()> {
        write_config_inner(tx, &current_latest_commit_key(), hash)
            .context("Cannot write latest commit hash")
    }

    fn execute_in_transaction<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&rusqlite::Transaction) -> anyhow::Result<()>,
    {
        let tx = self
            .sqlite_db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .context("Cannot create transaction")?;

        f(&tx)?;

        tx.commit().context("Cannot commit transaction")
    }

    fn read_project_id(&self) -> anyhow::Result<String> {
        let project_id = read_config_inner(&self.sqlite_db, &project_id_key())
            .context("Cannot read project id")?;
        if let Some(project_id) = project_id {
            Ok(project_id)
        } else {
            bail!(DBError::Fundamental(
                "Current project key not set".to_owned(),
            ))
        }
    }

    fn write_project_id(tx: &rusqlite::Transaction, project_id: &str) -> anyhow::Result<()> {
        write_config_inner(tx, &project_id_key(), project_id)
    }

    fn delete_branch_with_commits(
        tx: &rusqlite::Transaction,
        branch_name: &str,
    ) -> anyhow::Result<()> {
        let mut delete_commits_stmt = tx
            .prepare(
                "
            DELETE FROM commits WHERE branch = ?1;
            ",
            )
            .context("Cannot prepare delete commits of branch query")?;

        let mut delete_branch_stmt = tx
            .prepare(
                "
            DELETE FROM branches WHERE name = ?1;
            ",
            )
            .context("Cannot prepare delete branch query")?;

        delete_commits_stmt
            .execute([branch_name])
            .context("Cannot execute delete commits query")?;

        delete_branch_stmt
            .execute([branch_name])
            .context("Cannot execute branch query")?;

        Ok(())
    }

    fn read_name(&self) -> anyhow::Result<Option<String>> {
        read_config_inner(&self.sqlite_db, &user_name_key())
    }

    fn write_name(tx: &rusqlite::Transaction, name: &str) -> anyhow::Result<()> {
        write_config_inner(tx, &user_name_key(), name)
    }

    fn read_last_modification_time(&self) -> anyhow::Result<Option<i64>> {
        let raw = read_config_inner(&self.sqlite_db, &last_mod_time_key())?;
        if let Some(raw) = raw {
            return raw
                .parse::<i64>()
                .map(Some)
                .context("Cannot parse timestamp from string");
        }

        Ok(None)
    }

    fn write_last_modifiction_time(
        tx: &rusqlite::Transaction,
        last_mod_time: i64,
    ) -> anyhow::Result<()> {
        write_config_inner(tx, &last_mod_time_key(), &last_mod_time.to_string())
    }
}
