use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        error::{forbidden, redirect},
        request::headers,
    },
};
use uuid::Uuid;

use crate::db::App;

#[derive(FromRow)]
pub struct Student {
    pub id: Uuid,
    pub student_code: String,
    pub full_name: String,
    pub csrf_token: String,
}

pub fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub fn session_token(cx: &Cx) -> Option<&str> {
    headers(cx)
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("reserfisi_session="))
}

pub async fn current(cx: &Cx) -> Result<Option<Student>> {
    let Some(token) = session_token(cx) else {
        return Ok(None);
    };
    let app: &App = app_context(cx);
    Ok(sqlx::query_as("SELECT s.id, s.student_code, r.full_name, se.csrf_token FROM sessions se JOIN students s ON s.id = se.student_id JOIN student_roster r ON r.student_code = s.student_code WHERE se.token_hash = $1 AND se.expires_at > now() AND r.active")
        .bind(token_hash(token)).fetch_optional(&app.pool).await?)
}

pub async fn require(cx: &Cx, csrf: &str) -> Result<Student> {
    let student = current(cx)
        .await?
        .ok_or_else(|| redirect("/?auth=login&notice=session"))?;
    if csrf != student.csrf_token {
        return Err(forbidden().into());
    }
    Ok(student)
}

pub async fn create_session(app: &App, student: Uuid) -> Result<String> {
    // Dos UUID aleatorios: 244 bits de entropía. Solo el hash llega a PostgreSQL.
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let csrf = Uuid::new_v4().to_string();
    sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(&app.pool)
        .await?;
    sqlx::query("INSERT INTO sessions (token_hash, student_id, csrf_token, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(token_hash(&token)).bind(student).bind(csrf).bind(Utc::now() + Duration::days(7)).execute(&app.pool).await?;
    Ok(cookie(app, &token, 7 * 24 * 3600))
}

pub fn cookie(app: &App, token: &str, max_age: u32) -> String {
    format!(
        "reserfisi_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",
        if app.secure_cookies { "; Secure" } else { "" }
    )
}

pub async fn hash_password(password: String) -> Result<String> {
    Ok(tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|hash| hash.to_string())
            .map_err(|error| std::io::Error::other(error.to_string()))
    })
    .await??)
}

pub async fn verify_password(password: String, hash: String) -> Result<bool> {
    Ok(tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await?)
}
