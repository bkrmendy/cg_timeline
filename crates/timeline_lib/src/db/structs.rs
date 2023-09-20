use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct BlockRecord {
    pub hash: String,
    pub data: Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct Commit {
    pub hash: String,
    pub prev_commit_hash: String,
    pub project_id: String,
    pub branch: String,
    pub message: String,
    pub author: String,
    pub date: u64,
    pub header: Vec<u8>,
    pub blocks: String,
}
