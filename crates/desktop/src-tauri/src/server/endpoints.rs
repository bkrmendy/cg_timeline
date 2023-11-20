use actix_web::{
    get, post,
    web::{self, Json},
    HttpResponse, Responder,
};
use log::error;
use serde::{Deserialize, Serialize};
use timeline_lib::{
    api::{
        commit_command::create_new_commit, get_current_branch, get_latest_commit, init_command,
        list_branches_command::list_braches, log_checkpoints_command::list_checkpoints,
        new_branch_command::create_new_branch, restore_command::restore_checkpoint,
        switch_command::switch_branches,
    },
    db::db_ops::DBError,
};

use crate::serde_instances::DBErrorWrapper;

fn error_if_not_exists(db_path: &str) -> Result<(), DBError> {
    let exists = std::path::Path::new(db_path).exists();
    if !exists {
        return Err(DBError::Consistency("Database does not exist".to_owned()));
    }
    Ok(())
}

#[get("/healthcheck")]
pub async fn healthcheck() -> impl Responder {
    HttpResponse::Ok().json("Running")
}

#[derive(Deserialize)]
pub struct CommitPayload {
    db_path: String,
    file_path: String,
    message: String,
}

#[post("/commit")]
pub async fn commit(data: Json<CommitPayload>) -> impl Responder {
    let result = create_new_commit(
        &data.file_path,
        &data.db_path,
        Some(data.message.to_owned()),
    );

    match result {
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
        Ok(_) => HttpResponse::Ok().json("OK"),
    }
}

#[derive(Serialize)]
struct ShortCommitPayload {
    hash: String,
    message: String,
}

#[get("/checkpoints/{db_path}/{branch}")]
pub async fn checkpoints(path: web::Path<(String, String)>) -> impl Responder {
    let (db_path, branch_name) = path.into_inner();

    let result =
        error_if_not_exists(&db_path).and_then(|_| list_checkpoints(&db_path, &branch_name));

    match result {
        Ok(checkpoints) => HttpResponse::Ok().json(
            checkpoints
                .into_iter()
                .map(|checkpoint| ShortCommitPayload {
                    hash: checkpoint.hash,
                    message: checkpoint.message,
                })
                .collect::<Vec<ShortCommitPayload>>(),
        ),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[derive(Deserialize)]
pub struct RestorePayload {
    db_path: String,
    file_path: String,
    hash: String,
}

#[post("/restore")]
pub async fn restore(data: Json<RestorePayload>) -> impl Responder {
    let result = restore_checkpoint(&data.file_path, &data.db_path, &data.hash);

    match result {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[get("/branches/{db_path}")]
pub async fn branches(path: web::Path<(String,)>) -> impl Responder {
    let (db_path,) = path.into_inner();
    let result = error_if_not_exists(&db_path).and_then(|_| list_braches(&db_path));
    match result {
        Ok(branches) => HttpResponse::Ok().json(branches),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[derive(Deserialize)]
pub struct NewBranchPayload {
    db_path: String,
    branch_name: String,
}

#[post("/branches/new")]
pub async fn new_branch(data: Json<NewBranchPayload>) -> impl Responder {
    let result = create_new_branch(&data.db_path, &data.branch_name);
    match result {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[derive(Deserialize)]
pub struct SwitchBranchPayload {
    db_path: String,
    branch_name: String,
    file_path: String,
}

#[post("/branches/switch")]
pub async fn switch_branch(data: Json<SwitchBranchPayload>) -> impl Responder {
    let result = switch_branches(&data.db_path, &data.branch_name, &data.file_path);
    match result {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[get("/branches/current/{db_path}")]
pub async fn read_current_branch(path: web::Path<(String,)>) -> impl Responder {
    let (db_path,) = path.into_inner();
    let result = error_if_not_exists(&db_path)
        .and_then(|_| get_current_branch::get_current_branch(&db_path));
    match result {
        Ok(branch) => HttpResponse::Ok().json(branch),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[get("/commit/latest/{db_path}")]
pub async fn read_latest_commit_hash(path: web::Path<(String,)>) -> impl Responder {
    let (db_path,) = path.into_inner();
    let result =
        error_if_not_exists(&db_path).and_then(|_| get_latest_commit::get_latest_commit(&db_path));
    match result {
        Ok(branch) => HttpResponse::Ok().json(branch),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}

#[derive(Deserialize)]
pub struct ConnectDBPayload {
    db_path: String,
    file_path: String,
}

#[post("/connect")]
pub async fn connect(data: Json<ConnectDBPayload>) -> impl Responder {
    let already_exists = error_if_not_exists(&data.db_path).is_ok();
    if already_exists {
        return HttpResponse::Ok().finish();
    }

    let project_id = uuid::Uuid::new_v4().to_string();
    let result = init_command::init_db(&data.db_path, &project_id, &data.file_path);
    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            error!("{}", err);
            HttpResponse::BadRequest().json(DBErrorWrapper(err))
        }
    }
}
