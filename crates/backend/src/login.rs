use actix_session::Session;
use actix_web::{cookie::time::Duration, web, HttpResponse, Responder};

use redis::Commands;
use serde::Deserialize;
use uuid::Uuid;

use crate::utils::{gen_code, e500};



pub const REDIS_URL: &str = "redis://127.0.0.1/";
const EXPIRATION_SECS: usize = 300;

#[derive(Deserialize)]
pub struct LoginInitInfo {
    email: String,
}

#[derive(Deserialize)]
pub struct LoginCompleteInfo {
    email: String,
    activation_id: String,
    code: String,
}

pub async fn login_init(data: web::Json<LoginInitInfo>) -> impl Responder {
    let activation_id = format!("{}", Uuid::new_v4());
    let code = gen_code();

    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut conn = client.get_connection().unwrap();

    println!("activation id: {}, code: {}", activation_id, code);

    let redis_result: Result<(), redis::RedisError> = {
        conn.set(activation_id.clone(), code.clone())
            .and_then(|_: ()| conn.expire(activation_id.clone(), EXPIRATION_SECS))
    };

    match redis_result {
        Err(_) => HttpResponse::InternalServerError().json("Internal error"),
        Ok(_) => HttpResponse::Ok().json(code),
    }
}

const EMAIL_KEY: &str = "email";

pub async fn login_complete(
    session: Session,
    data: web::Json<LoginCompleteInfo>,
) -> impl Responder {
    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut conn = client.get_connection().unwrap();

    let value: Result<String, redis::RedisError> = conn.get(&data.activation_id);

    if value.is_err() {
        return HttpResponse::Unauthorized().json("no login session in progress");
    }

    let code = value.unwrap();

    if code != data.code {
        return HttpResponse::Unauthorized().json("incorrect code");
    }

    let delete_result: Result<(), redis::RedisError> = conn.del(&data.activation_id);
    if delete_result.is_err() {
        return HttpResponse::InternalServerError().json("Internal error");
    }

    session.insert(EMAIL_KEY, &data.email).unwrap();

    let cookie = actix_web::cookie::Cookie::build(EMAIL_KEY, &data.email)
        .max_age(Duration::days(7))
        .finish();

    HttpResponse::Ok().cookie(cookie).finish()
}

pub async fn authenticated_endpoint(session: Session) -> Result<HttpResponse, actix_web::Error> {
    if let Some(email) = session.get::<String>(EMAIL_KEY).map_err(e500)? {
        Ok(HttpResponse::Ok().json(format!("Welcome to the club, {:?}", email)))
    } else {
        Ok(HttpResponse::Forbidden().json("Not authenticated"))
    }
}
