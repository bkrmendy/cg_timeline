use rand::Rng;

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn gen_code() -> String {
    let mut rng = rand::thread_rng();
    let code: i32 = rng.gen_range(100_000..=999_999);
    format!("{}", code)
}
