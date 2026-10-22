use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Body, content::Form, response::Response, route},
};
use uuid::Uuid;

use crate::{
    auth,
    db::{self, App, BookingForm},
};

pub fn see_other(location: &str, cookie: Option<String>) -> Result<Response> {
    let mut response = Response::builder()
        .status(303)
        .header("location", location)
        .header("cache-control", "no-store");
    if let Some(cookie) = cookie {
        response = response.header("set-cookie", cookie);
    }
    Ok(response.body(Body::empty())?)
}

#[derive(Deserialize)]
pub struct RegisterForm {
    student_code: String,
    email: String,
    password: String,
}

#[route(POST "/auth/register")]
pub async fn register(cx: &Cx, Form(form): Form<RegisterForm>) -> Result<Response> {
    let app: &App = app_context(cx);
    if !(10..=128).contains(&form.password.chars().count()) {
        return see_other("/?auth=register&notice=password", None);
    }
    let code: Option<String> = sqlx::query_scalar(
        "SELECT student_code FROM student_roster WHERE student_code = $1 AND email = $2 AND active",
    )
    .bind(form.student_code.trim())
    .bind(form.email.trim().to_lowercase())
    .fetch_optional(&app.pool)
    .await?;
    let Some(code) = code else {
        return see_other("/?auth=register&notice=roster", None);
    };
    let hash = auth::hash_password(form.password).await?;
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO students (id, student_code, password_hash) VALUES ($1, $2, $3) ON CONFLICT (student_code) DO NOTHING")
        .bind(id).bind(code).bind(hash).execute(&app.pool).await?;
    if result.rows_affected() == 0 {
        return see_other("/?auth=login&notice=registered", None);
    }
    see_other(
        "/?notice=welcome",
        Some(auth::create_session(app, id).await?),
    )
}

#[derive(Deserialize)]
pub struct LoginForm {
    student_code: String,
    password: String,
}

#[route(POST "/auth/login")]
pub async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<Response> {
    let app: &App = app_context(cx);
    if form.password.len() > 512 {
        return see_other("/?auth=login&notice=credentials", None);
    }
    let student: Option<(Uuid, String)> = sqlx::query_as("SELECT s.id, s.password_hash FROM students s JOIN student_roster r ON r.student_code = s.student_code WHERE s.student_code = $1 AND r.active")
        .bind(form.student_code.trim()).fetch_optional(&app.pool).await?;
    if let Some((id, hash)) = student
        && auth::verify_password(form.password, hash).await?
    {
        if let Some(old) = auth::session_token(cx) {
            sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
                .bind(auth::token_hash(old))
                .execute(&app.pool)
                .await?;
        }
        return see_other(
            "/?notice=welcome",
            Some(auth::create_session(app, id).await?),
        );
    }
    see_other("/?auth=login&notice=credentials", None)
}

#[derive(Deserialize)]
pub struct CsrfForm {
    csrf: String,
}

#[route(POST "/auth/logout")]
pub async fn logout(cx: &Cx, Form(form): Form<CsrfForm>) -> Result<Response> {
    auth::require(cx, &form.csrf).await?;
    let app: &App = app_context(cx);
    if let Some(token) = auth::session_token(cx) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(auth::token_hash(token))
            .execute(&app.pool)
            .await?;
    }
    see_other("/", Some(auth::cookie(app, "", 0)))
}

#[route(POST "/reservations")]
pub async fn reserve(cx: &Cx, Form(form): Form<BookingForm>) -> Result<Response> {
    let student = auth::require(cx, &form.csrf).await?;
    let app: &App = app_context(cx);
    match db::reserve(&app.pool, student.id, &form).await {
        Ok(_) => see_other("/?view=my&notice=reserved#mis-reservas", None),
        Err(message) => see_other(
            &format!(
                "/?date={}&space={}&notice={message}#agenda",
                form.date, form.space_id
            ),
            None,
        ),
    }
}

#[derive(Deserialize)]
pub struct CancelForm {
    csrf: String,
    id: Uuid,
}

#[route(POST "/reservations/cancel")]
pub async fn cancel(cx: &Cx, Form(form): Form<CancelForm>) -> Result<Response> {
    let student = auth::require(cx, &form.csrf).await?;
    let app: &App = app_context(cx);
    let result = sqlx::query("UPDATE reservations SET cancelled_at = now() WHERE id = $1 AND student_id = $2 AND cancelled_at IS NULL AND starts_at > now()")
        .bind(form.id).bind(student.id).execute(&app.pool).await?;
    see_other(
        if result.rows_affected() == 1 {
            "/?view=my&notice=cancelled#mis-reservas"
        } else {
            "/?view=my&notice=cannot_cancel#mis-reservas"
        },
        None,
    )
}
