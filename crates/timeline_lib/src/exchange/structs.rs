use crate::db::structs::{BlockRecord, Commit};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Debug)]
pub struct Exchange {
    pub commits: Vec<Commit>,
    pub blocks: Vec<BlockRecord>,
}

pub fn encode_exchange(exchange: &Exchange) -> Result<Vec<u8>, String> {
    bincode::serialize(exchange).map_err(|e| format!("Cannot encode exchange: {:?}", e))
}

pub fn decode_exchange(data: &[u8]) -> Result<Exchange, String> {
    bincode::deserialize(data).map_err(|e| format!("Cannot decode exchange: {:?}", e))
}

#[derive(Serialize, Deserialize)]
pub struct Sync {
    // tips of local branches
    // the descendants of these will be returned
    pub local_tips: Vec<String>,
    // data that will be sent "up"
    pub exchange: Exchange,
}

pub fn decode_sync(data: &[u8]) -> Result<Sync, String> {
    bincode::deserialize(data).map_err(|e| format!("Cannot decode sync: {:?}", e))
}

pub fn encode_sync(sync: &Sync) -> Result<Vec<u8>, String> {
    bincode::serialize(sync).map_err(|e| format!("Cannot encode sync: {:?}", e))
}

#[cfg(test)]
mod test {
    use crate::{
        api::init_command::MAIN_BRANCH_NAME,
        db::structs::{BlockRecord, Commit},
        exchange::structs::decode_exchange,
    };

    use super::{encode_exchange, Exchange};

    #[test]
    fn test_round_trip_serialize_deserialize() {
        let original_exchange = Exchange {
            commits: vec![
                Commit {
                    hash: String::from("abc123"),
                    prev_commit_hash: String::from("def456"),
                    project_id: String::from("proj789"),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: String::from("Initial commit"),
                    author: String::from("John Doe"),
                    date: 1632870400, // Unix timestamp
                    header: vec![1, 2, 3, 4, 5],
                    blocks: String::from("blocks data 1"),
                },
                Commit {
                    hash: String::from("qwe234"),
                    prev_commit_hash: String::from("abc123"),
                    project_id: String::from("proj78"),
                    branch: String::from(MAIN_BRANCH_NAME),
                    message: String::from("Next commit"),
                    author: String::from("John Doe too"),
                    date: 1632870410, // Unix timestamp
                    header: vec![1, 2, 3, 4, 5],
                    blocks: String::from("blocks data 2"),
                },
            ],
            blocks: vec![
                BlockRecord {
                    hash: String::from("aaaabbbb"),
                    data: vec![1, 2, 3, 4],
                },
                BlockRecord {
                    hash: String::from("ccccdddd"),
                    data: vec![5, 6, 7, 8],
                },
            ],
        };

        let serialized = encode_exchange(&original_exchange).unwrap();
        assert_eq!(serialized.len(), 342);

        let deserialized = decode_exchange(&serialized).unwrap();
        assert_eq!(deserialized, original_exchange);
    }
}
