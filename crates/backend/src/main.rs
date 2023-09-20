mod clone;
mod db;
mod login;
mod sync;
mod utils;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    web::{get, post, PayloadConfig},
    App, HttpServer,
};
use clone::clone_project;
use login::{authenticated_endpoint, login_complete, login_init, REDIS_URL};
use sync::v1_sync;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key = Key::generate();
    let store = RedisSessionStore::new(REDIS_URL).await.unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(SessionMiddleware::new(
                store.to_owned(),
                secret_key.to_owned(),
            ))
            .route("/login/init", post().to(login_init))
            .route("/login/complete", post().to(login_complete))
            .route("/check", get().to(authenticated_endpoint))
            .route("/v1/sync", post().to(v1_sync))
            .route("/v1/clone/{project_id}", get().to(clone_project))
            .app_data(PayloadConfig::new(1000000 * 250))
    })
    .bind("127.0.0.1:13337")?
    .run()
    .await
}
