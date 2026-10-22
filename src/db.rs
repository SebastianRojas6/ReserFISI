use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use serde::Deserialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct App {
    pub pool: PgPool,
    pub secure_cookies: bool,
}

#[derive(FromRow)]
pub struct Space {
    pub id: i16,
    pub name: String,
    pub category: String,
    pub description: String,
    pub capacity: i32,
    pub location: String,
    pub amenity: String,
    pub illustration: String,
}

#[derive(FromRow)]
pub struct Booking {
    pub id: Uuid,
    pub space_id: i16,
    pub space_name: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub purpose: String,
    pub attendees: i32,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct BookingForm {
    pub csrf: String,
    pub space_id: i16,
    pub date: NaiveDate,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub purpose: String,
    pub attendees: i32,
}

pub fn lima() -> FixedOffset {
    FixedOffset::west_opt(5 * 3600).expect("UTC-5 es válido")
}

pub fn today() -> NaiveDate {
    Utc::now().with_timezone(&lima()).date_naive()
}

pub fn at(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    lima()
        .from_local_datetime(&date.and_time(time))
        .single()
        .expect("offset fijo")
        .with_timezone(&Utc)
}

pub fn time_label(time: DateTime<Utc>) -> String {
    time.with_timezone(&lima()).format("%H:%M").to_string()
}

pub fn validate_booking(
    form: &BookingForm,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), &'static str> {
    let start = at(form.date, form.start);
    let end = at(form.date, form.end);
    if start <= now || form.date > now.with_timezone(&lima()).date_naive() + Duration::days(30) {
        return Err("date");
    }
    if end <= start
        || end - start > Duration::hours(3)
        || form.start.hour() < 8
        || form.end > NaiveTime::from_hms_opt(22, 0, 0).unwrap()
        || ![form.start, form.end]
            .iter()
            .all(|t| t.minute() % 30 == 0 && t.second() == 0 && t.nanosecond() == 0)
    {
        return Err("hours");
    }
    if !(5..=180).contains(&form.purpose.trim().chars().count()) || form.attendees < 1 {
        return Err("details");
    }
    Ok((start, end))
}

pub async fn spaces(pool: &PgPool) -> Result<Vec<Space>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM spaces ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn day_bookings(pool: &PgPool, date: NaiveDate) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as("SELECT r.*, s.name AS space_name FROM reservations r JOIN spaces s ON s.id = r.space_id WHERE r.starts_at >= $1 AND r.starts_at < $2 AND r.cancelled_at IS NULL ORDER BY r.starts_at")
        .bind(at(date, NaiveTime::MIN)).bind(at(date + Duration::days(1), NaiveTime::MIN)).fetch_all(pool).await
}

pub async fn my_bookings(pool: &PgPool, student: Uuid) -> Result<Vec<Booking>, sqlx::Error> {
    sqlx::query_as("SELECT r.*, s.name AS space_name FROM reservations r JOIN spaces s ON s.id = r.space_id WHERE student_id = $1 ORDER BY starts_at DESC LIMIT 100")
        .bind(student).fetch_all(pool).await
}

pub async fn reserve(
    pool: &PgPool,
    student: Uuid,
    form: &BookingForm,
) -> Result<Uuid, &'static str> {
    let (start, end) = validate_booking(form, Utc::now())?;
    let capacity: Option<i32> = sqlx::query_scalar("SELECT capacity FROM spaces WHERE id = $1")
        .bind(form.space_id)
        .fetch_optional(pool)
        .await
        .map_err(log_error)?;
    if capacity.is_none_or(|capacity| form.attendees > capacity) {
        return Err("capacity");
    }
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO reservations (id, student_id, space_id, starts_at, ends_at, purpose, attendees) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(id).bind(student).bind(form.space_id).bind(start).bind(end).bind(form.purpose.trim()).bind(form.attendees)
        .execute(pool).await;
    match result {
        Ok(_) => Ok(id),
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23P01") => {
            Err(if error.constraint() == Some("no_student_overlap") {
                "student_overlap"
            } else {
                "overlap"
            })
        }
        Err(error) => Err(log_error(error)),
    }
}

pub fn log_error(error: sqlx::Error) -> &'static str {
    tracing::error!(%error, "Error de PostgreSQL");
    "server"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_lima_hours_duration_and_booking_window() {
        let now = DateTime::parse_from_rfc3339("2026-10-22T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut form = BookingForm {
            csrf: String::new(),
            space_id: 1,
            date: NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
            start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            purpose: "Partido de estudiantes".into(),
            attendees: 12,
        };
        let (start, _) = validate_booking(&form, now).unwrap();
        assert_eq!(start.hour(), 13);
        form.end = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert_eq!(validate_booking(&form, now), Err("hours"));
        form.end = NaiveTime::from_hms_opt(8, 15, 0).unwrap();
        assert_eq!(validate_booking(&form, now), Err("hours"));
        form.end = form.start;
        assert_eq!(validate_booking(&form, now), Err("hours"));
        form.date = today() + Duration::days(1000);
        assert_eq!(validate_booking(&form, now), Err("date"));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore = "Requiere DATABASE_URL y permiso CREATEDB; ver README"]
    async fn database_prevents_races_and_allows_adjacent_or_cancelled_slots(pool: PgPool) {
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();
        for (id, code, email) in [
            (alice, "20260001", "alice@fisi.example"),
            (bob, "20260002", "bob@fisi.example"),
        ] {
            sqlx::query("INSERT INTO student_roster (student_code, full_name, email) VALUES ($1, 'Estudiante de prueba', $2)").bind(code).bind(email).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO students (id, student_code, password_hash) VALUES ($1, $2, 'test-only')").bind(id).bind(code).execute(&pool).await.unwrap();
        }
        let mut form = BookingForm {
            csrf: String::new(),
            space_id: 1,
            date: today() + Duration::days(1),
            start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            purpose: "Prueba concurrente".into(),
            attendees: 10,
        };
        let (first, second) =
            tokio::join!(reserve(&pool, alice, &form), reserve(&pool, bob, &form));
        assert_ne!(
            first.is_ok(),
            second.is_ok(),
            "Solo una solicitud simultánea puede ganar"
        );
        let (winner, loser, id) = if let Ok(id) = first {
            assert_eq!(second, Err("overlap"));
            (alice, bob, id)
        } else {
            assert_eq!(first, Err("overlap"));
            (bob, alice, second.unwrap())
        };

        form.space_id = 2;
        assert_eq!(reserve(&pool, winner, &form).await, Err("student_overlap"));
        assert!(
            reserve(&pool, loser, &form).await.is_ok(),
            "Otro estudiante sí puede usar otro espacio"
        );

        form.space_id = 1;
        form.start = NaiveTime::from_hms_opt(11, 0, 0).unwrap();
        form.end = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
        assert!(
            reserve(&pool, winner, &form).await.is_ok(),
            "Se permiten horarios adyacentes"
        );

        sqlx::query("UPDATE reservations SET cancelled_at = now() WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        form.start = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        form.end = NaiveTime::from_hms_opt(11, 0, 0).unwrap();
        assert!(
            reserve(&pool, winner, &form).await.is_ok(),
            "Cancelar libera el espacio"
        );
        form.attendees = 23;
        assert_eq!(reserve(&pool, winner, &form).await, Err("capacity"));
    }
}
