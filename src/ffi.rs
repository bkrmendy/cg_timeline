use std::{error::Error, fmt::Display};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::api::{
    blend_file_from_timeline_command, create_new_checkpoint_command::create_new_checkpoint,
    delete_branch, get_current_branch::get_current_branch, get_current_commit::get_current_commit,
    init_command::init_db, list_branches_command::list_braches,
    log_checkpoints_command::list_checkpoints, new_branch_command::create_new_branch,
    restore_command, switch_command::switch_branches,
};

#[derive(Serialize)]
pub struct ConnectResponse {
    pub branches: Vec<String>,
    pub current_branch_name: String,
    pub checkpoints_on_this_branch: Vec<(String, String)>,
    pub current_checkpoint_hash: String,
}

#[derive(Serialize)]
pub struct CreateCheckpointResponse {
    pub checkpoints_on_this_branch: Vec<(String, String)>,
    pub current_checkpoint_hash: String,
}

#[derive(Serialize)]
pub struct RestoreCheckpointResponse {
    pub current_checkpoint_hash: String,
}

#[derive(Serialize)]
pub struct SwitchToNewBranchResponse {
    pub branches: Vec<String>,
    pub current_branch_name: String,
}

#[derive(Serialize)]
pub struct SwitchBranchResponse {
    pub current_branch_name: String,
    pub checkpoints_on_this_branch: Vec<(String, String)>,
    pub current_checkpoint_hash: String,
}

#[derive(Serialize)]
pub struct BlendFileFromTimelineResponse {
    pub restored_file_path: String,
}

#[derive(Serialize)]
pub struct DeleteBranchResponse {
    pub branches: Vec<String>,
}

#[derive(Debug)]
pub enum FFIError {
    MalformedMessage(String),
    InternalError(String),
    SerializationError,
}

impl Error for FFIError {}

impl Display for FFIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FFIError::MalformedMessage(msg) => write!(f, "Malformed message: {}", msg),
            FFIError::SerializationError => write!(f, "Serialization error"),
            FFIError::InternalError(msg) => write!(f, "Internal error, {}", msg),
        }
    }
}

struct DBPath<'a>(&'a str);
struct PathToBlend<'a>(&'a str);

fn connect_command(db_path: DBPath, path_to_blend: PathToBlend) -> anyhow::Result<ConnectResponse> {
    let exists = std::path::Path::new(&db_path.0).exists();

    if !exists {
        let project_id = uuid::Uuid::new_v4().to_string();

        init_db(db_path.0, &project_id, path_to_blend.0)?;
    }

    let branches = list_braches(db_path.0)?;
    let current_branch_name = get_current_branch(db_path.0)?;
    let checkpoints_on_this_branch = list_checkpoints(db_path.0, &current_branch_name)
        .map(|commits| commits.into_iter().map(|c| (c.hash, c.message)).collect())?;
    let current_checkpoint_hash = get_current_commit(db_path.0)?;

    Ok(ConnectResponse {
        branches,
        current_branch_name,
        checkpoints_on_this_branch,
        current_checkpoint_hash,
    })
}

fn create_checkpoint(
    db_path: DBPath,
    path_to_blend: PathToBlend,
    message: &str,
) -> anyhow::Result<CreateCheckpointResponse> {
    create_new_checkpoint(path_to_blend.0, db_path.0, Some(message.to_string()))?;

    let current_branch_name = get_current_branch(db_path.0)?;
    let checkpoints_on_this_branch = list_checkpoints(db_path.0, &current_branch_name)
        .map(|commits| commits.into_iter().map(|c| (c.hash, c.message)).collect())?;
    let current_checkpoint_hash = get_current_commit(db_path.0)?;

    Ok(CreateCheckpointResponse {
        checkpoints_on_this_branch,
        current_checkpoint_hash,
    })
}

fn restore_checkpoint(
    db_path: DBPath,
    path_to_blend: PathToBlend,
    hash: &str,
) -> anyhow::Result<RestoreCheckpointResponse> {
    restore_command::restore_checkpoint(path_to_blend.0, db_path.0, hash)?;
    let current_checkpoint_hash = get_current_commit(db_path.0)?;
    Ok(RestoreCheckpointResponse {
        current_checkpoint_hash,
    })
}

fn switch_to_new_branch(
    db_path: DBPath,
    branch_name: &str,
) -> anyhow::Result<SwitchToNewBranchResponse> {
    create_new_branch(db_path.0, branch_name)?;
    let branches = list_braches(db_path.0)?;
    let current_branch_name = get_current_branch(db_path.0)?;

    Ok(SwitchToNewBranchResponse {
        branches,
        current_branch_name,
    })
}

fn switch_to_branch(
    db_path: DBPath,
    path_to_blend: PathToBlend,
    branch_name: &str,
) -> anyhow::Result<SwitchBranchResponse> {
    switch_branches(db_path.0, branch_name, path_to_blend.0)?;
    let current_branch_name = get_current_branch(db_path.0)?;
    let checkpoints_on_this_branch = list_checkpoints(db_path.0, &current_branch_name)
        .map(|commits| commits.into_iter().map(|c| (c.hash, c.message)).collect())?;
    let current_checkpoint_hash = get_current_commit(db_path.0)?;

    Ok(SwitchBranchResponse {
        current_branch_name,
        checkpoints_on_this_branch,
        current_checkpoint_hash,
    })
}

fn blend_file_from_timeline(db_path: DBPath) -> anyhow::Result<BlendFileFromTimelineResponse> {
    let restored_file_path = blend_file_from_timeline_command::blend_file_from_timeline(db_path.0)?;
    Ok(BlendFileFromTimelineResponse { restored_file_path })
}

fn delete_branch(db_path: DBPath, branch_name: &str) -> anyhow::Result<DeleteBranchResponse> {
    delete_branch::delete_branch(db_path.0, branch_name)?;
    let branches = list_braches(db_path.0)?;
    Ok(DeleteBranchResponse { branches })
}

type JsonObject = Map<String, Value>;

fn get_string_value<'a>(value: &'a JsonObject, key: &'a str) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(|c| c.as_str())
        .ok_or(FFIError::MalformedMessage(format!("{} not in object", key)).into())
}

pub fn error_json(error: FFIError) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "error".to_string(),
        serde_json::Value::String(format!("Error: {}", error)),
    );
    serde_json::Value::Object(object)
}

pub fn do_command(value: Value) -> anyhow::Result<String> {
    let value = value.as_object().ok_or(FFIError::MalformedMessage(
        "Payload should be an object".to_string(),
    ))?;
    let command_name = get_string_value(value, "command")?;

    match command_name {
        "connect" => {
            let db_path = get_string_value(value, "db_path")?;
            let path_to_blend = get_string_value(value, "path_to_blend")?;

            let result = connect_command(DBPath(db_path), PathToBlend(path_to_blend))?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "create-checkpoint" => {
            let db_path = get_string_value(value, "db_path")?;
            let path_to_blend = get_string_value(value, "path_to_blend")?;
            let message = get_string_value(value, "message")?;

            let result = create_checkpoint(DBPath(db_path), PathToBlend(path_to_blend), message)?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "restore-checkpoint" => {
            let db_path = get_string_value(value, "db_path")?;
            let path_to_blend = get_string_value(value, "path_to_blend")?;
            let hash = get_string_value(value, "hash")?;

            let result = restore_checkpoint(DBPath(db_path), PathToBlend(path_to_blend), hash)?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "switch-to-new-branch" => {
            let db_path = get_string_value(value, "db_path")?;
            let branch_name = get_string_value(value, "branch_name")?;

            let result = switch_to_new_branch(DBPath(db_path), branch_name)?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "switch-to-branch" => {
            let db_path = get_string_value(value, "db_path")?;
            let path_to_blend = get_string_value(value, "path_to_blend")?;
            let branch_name = get_string_value(value, "branch_name")?;

            let result =
                switch_to_branch(DBPath(db_path), PathToBlend(path_to_blend), branch_name)?;

            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "blend-file-from-timeline" => {
            let db_path = get_string_value(value, "db_path")?;
            let result = blend_file_from_timeline(DBPath(db_path))?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        "delete-branch" => {
            let db_path = get_string_value(value, "db_path")?;
            let branch_name = get_string_value(value, "branch_name")?;
            let result = delete_branch(DBPath(db_path), branch_name)?;
            let s = serde_json::to_string(&result)?;
            Ok(s)
        }

        c => {
            let resp = serde_json::to_string(&error_json(FFIError::InternalError(format!(
                "Command {} not implemented",
                c
            ))))?;
            Ok(resp)
        }
    }
}
