use std::{fmt::Display, path::Path};

use timeline_lib::db::structs::{BlockRecord, Commit};

#[derive(Debug)]
pub enum ServerDBError {
    Fundamental(String),
    Consistency(String),
    Error(String),
}

impl Display for ServerDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerDBError::Fundamental(msg) => write!(f, "Fundamental error: {}", msg),
            ServerDBError::Consistency(msg) => write!(f, "Consistency error: {}", msg),
            ServerDBError::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

pub struct DB {
    relational: rusqlite::Connection,
    kv: rocksdb::DB,
}

pub fn open_db(path: &str) -> Result<DB, ServerDBError> {
    let relational_path = Path::new(path).join("commits.sqlite");
    let kv_path = Path::new(path).join("blobs.rocks");

    let relational_db = rusqlite::Connection::open(relational_path)
        .map_err(|e| ServerDBError::Fundamental(format!("Cannot open DB: {:?}", e)))?;

    relational_db
        .execute(
            "CREATE TABLE IF NOT EXISTS commits (
        hash TEXT PRIMARY KEY,
        prev_commit_hash TEXT,
        project_id TEXT,
        branch TEXT,
        message TEXT,
        author TEXT,
        date INTEGER,
        header BLOB
    )",
            [],
        )
        .map_err(|e| ServerDBError::Fundamental(format!("Cannot create commits table: {:?}", e)))?;

    let rocks_db = rocksdb::DB::open_default(kv_path)
        .map_err(|e| ServerDBError::Fundamental(format!("Cannot open RocksDB: {:?}", e)))?;

    Ok(DB {
        relational: relational_db,
        kv: rocks_db,
    })
}

#[inline]
fn block_hash_key(key: &str) -> String {
    format!("block-hash-{:?}", key)
}

pub fn write_blocks(db: &DB, blocks: Vec<BlockRecord>) -> Result<(), ServerDBError> {
    for block in blocks {
        db.kv
            .put(block_hash_key(&block.hash), &block.data)
            .map_err(|e| ServerDBError::Error(format!("Cannot write block: {:?}", e)))?;
    }

    Ok(())
}

pub fn read_blocks(db: &DB, hashes: Vec<String>) -> Result<Vec<BlockRecord>, ServerDBError> {
    let mut result: Vec<BlockRecord> = Vec::new();
    for hash in hashes {
        let block_data = db
            .kv
            .get(block_hash_key(&hash))
            .map_err(|e| ServerDBError::Error(format!("Error reading block: {:?}", e)))?
            .ok_or(ServerDBError::Error("No block with hash found".to_owned()))?;

        result.push(BlockRecord {
            hash,
            data: block_data,
        })
    }

    Ok(result)
}

pub fn write_commits(db: &DB, commits: Vec<Commit>) -> Result<(), ServerDBError> {
    for commit in commits {
        db.kv
            .put(working_dir_key(&commit.hash), commit.blocks)
            .map_err(|_| ServerDBError::Error("Cannot write working dir blocks".to_owned()))?;

        let hash = commit.hash.clone();

        db.relational.execute(
            "INSERT INTO commits (hash, prev_commit_hash, project_id, branch, message, author, date, header) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                commit.hash,
                commit.prev_commit_hash,
                commit.project_id,
                commit.branch,
                commit.message,
                commit.author,
                commit.date,
                commit.header,
            ),
        )
        .map_err(|e| ServerDBError::Error(format!("Cannot insert commit object: {:?}", e)))?;

        println!("wrote commit with hash {}", hash);
    }

    Ok(())
}

#[inline]
fn working_dir_key(key: &str) -> String {
    format!("working-dir-{:?}", key)
}

fn get_blocks_by_hash(rocks_db: &rocksdb::DB, hash: &str) -> Result<String, ServerDBError> {
    rocks_db
        .get(working_dir_key(hash))
        .map_err(|e| ServerDBError::Error(format!("Cannot read working dir key: {:?}", e)))?
        .map(|bs| String::from_utf8(bs).unwrap())
        .ok_or(ServerDBError::Consistency(
            "No working dir found".to_owned(),
        ))
}

pub fn read_descendants_of_commit(db: &DB, hash: &str) -> Result<Vec<Commit>, ServerDBError> {
    let mut stmt = db
        .relational
        .prepare(
            "
            WITH RECURSIVE ancestor_commits(hash, prev_commit_hash, project_id, branch, message, author, date, header) AS (
                SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header FROM commits WHERE hash = ?1
                UNION ALL
                SELECT c.hash, c.prev_commit_hash, c.project_id, c.branch, c.message, c.author, c.date, c.header FROM commits c
                JOIN ancestor_commits a ON c.prev_commit_hash = a.hash
            )
            SELECT hash, prev_commit_hash, project_id, branch, message, author, date, header FROM ancestor_commits ORDER BY date ASC;
            ",
        )
        .map_err(|e| {
            ServerDBError::Fundamental(format!("Cannot prepare read commits query: {:?}", e))
        })?;

    let mut rows = stmt
        .query([hash])
        .map_err(|e| ServerDBError::Error(format!("Cannot read commits: {:?}", e)))?;

    let mut result: Vec<Commit> = vec![];

    while let Ok(Some(data)) = rows.next() {
        let hash: String = data
            .get::<usize, String>(0)
            .expect("No hash found in row")
            .to_string();

        let blocks = get_blocks_by_hash(&db.kv, &hash)?;

        result.push(Commit {
            hash,
            prev_commit_hash: data.get(1).expect("No prev_commit_hash found in row"),
            project_id: data.get(2).expect("No project_id found in row"),
            branch: data.get(3).expect("No branch found in row"),
            message: data.get(4).expect("No message found in row"),
            author: data.get(5).expect("No author found in row"),
            date: data.get(6).expect("No date found in row"),
            header: data.get(7).expect("No header found in row"),
            blocks,
        })
    }

    Ok(result)
}

pub fn read_commits_with_project_id(
    db: &DB,
    project_id: &str,
) -> Result<Vec<Commit>, ServerDBError> {
    let mut stmt = db
        .relational
        .prepare("SELECT * from commits where project_id = ?1;")
        .map_err(|e| {
            ServerDBError::Fundamental(format!("Cannot prepare read commits query: {:?}", e))
        })?;

    let mut rows = stmt
        .query([project_id])
        .map_err(|e| ServerDBError::Error(format!("Cannot read commits: {:?}", e)))?;

    let mut result: Vec<Commit> = vec![];

    while let Ok(Some(data)) = rows.next() {
        let hash: String = data
            .get::<usize, String>(0)
            .expect("No hash found in row")
            .to_string();

        let blocks = get_blocks_by_hash(&db.kv, &hash)?;

        result.push(Commit {
            hash,
            prev_commit_hash: data.get(1).expect("No prev_commit_hash found in row"),
            project_id: data.get(2).expect("No project_id found in row"),
            branch: data.get(3).expect("No branch found in row"),
            message: data.get(4).expect("No message found in row"),
            author: data.get(5).expect("No author found in row"),
            date: data.get(6).expect("No date found in row"),
            header: data.get(7).expect("No header found in row"),
            blocks,
        })
    }

    Ok(result)
}
