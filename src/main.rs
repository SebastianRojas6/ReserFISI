mod actions;
mod auth;
mod db;
mod ui;

use sqlx::postgres::PgPoolOptions;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, LayerFn, LayerFuture, Next, Router,
        content::{Css, Js},
        response::Response,
        route,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reserfisi=info".into()),
        )
        .init();
    let url = std::env::var("DATABASE_URL")
        .expect("Configura DATABASE_URL en .env. Consulta .env.example y README.md");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    let app = db::App {
        pool,
        secure_cookies: std::env::var("COOKIE_SECURE").is_ok_and(|value| value == "true"),
    };
    let router = Router::builder()
        .app_context(app)
        .page(ui::home)
        .route(styles)
        .route(script)
        .route(illustration)
        .route(faculty_photo)
        .route(courtyard_photo)
        .route(faculty_logo)
        .route(faculty_logo_vector)
        .route(health)
        .route(actions::register)
        .route(actions::login)
        .route(actions::logout)
        .route(actions::reserve)
        .route(actions::cancel)
        .layer(LayerFn::new(None::<&str>, response_headers))
        .build();
    tracing::info!("ReserFISI listo. HOST y PORT configurables; por defecto http://127.0.0.1:3000");
    topcoat::start(router).await?;
    Ok(())
}

fn response_headers<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    Box::pin(async move {
        let mut response = next.run(cx, body).await?;
        let headers = response.headers_mut();
        headers.insert("x-content-type-options", "nosniff".parse()?);
        headers.insert("referrer-policy", "same-origin".parse()?);
        headers.insert("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'".parse()?);
        if !headers.contains_key("cache-control") {
            headers.insert("cache-control", "no-store".parse()?);
        }
        Ok(response)
    })
}

#[route(GET "/assets/style.css")]
async fn styles() -> Result<Css<&'static str>> {
    Ok(Css(include_str!("../assets/style.css")))
}

#[route(GET "/assets/app.js")]
async fn script() -> Result<Js<&'static str>> {
    Ok(Js(include_str!("../assets/app.js")))
}

#[route(GET "/assets/spaces.svg")]
async fn illustration() -> Result<impl topcoat::router::response::IntoResponse> {
    Ok((
        [("content-type", "image/svg+xml")],
        include_str!("../assets/spaces.svg"),
    ))
}

#[route(GET "/health")]
async fn health(cx: &Cx) -> Result<&'static str> {
    let app: &db::App = topcoat::context::app_context(cx);
    sqlx::query("SELECT 1").execute(&app.pool).await?;
    Ok("ok")
}

fn image_response(content_type: &'static str, bytes: &'static [u8]) -> Result<Response> {
    Ok(Response::builder()
        .header("content-type", content_type)
        .header("cache-control", "public, max-age=3600")
        .body(Body::from(bytes))?)
}

#[route(GET "/assets/fisi.jpg")]
async fn faculty_photo() -> Result<Response> {
    image_response("image/jpeg", include_bytes!("../assets/fisi.jpg"))
}

#[route(GET "/assets/fisi2.jpg")]
async fn courtyard_photo() -> Result<Response> {
    image_response("image/jpeg", include_bytes!("../assets/fisi2.jpg"))
}

#[route(GET "/assets/logo-fisi.png")]
async fn faculty_logo() -> Result<Response> {
    image_response("image/png", include_bytes!("../assets/logo-fisi.png"))
}

#[route(GET "/assets/logo-fisi.svg")]
async fn faculty_logo_vector() -> Result<Response> {
    image_response("image/svg+xml", include_bytes!("../assets/logo-fisi.svg"))
}
