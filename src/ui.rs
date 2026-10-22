use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike, Utc};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{page, parse_query_params},
    view::{View, component, view},
};

use crate::{
    auth,
    db::{self, App, Booking, Space},
};

#[derive(Default, Deserialize)]
#[serde(default)]
struct Query {
    date: Option<NaiveDate>,
    category: String,
    view: String,
    auth: String,
    notice: String,
    space: Option<i16>,
    booking: Option<i16>,
    start: Option<NaiveTime>,
}

fn date_label(date: NaiveDate) -> String {
    let days = [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ];
    let months = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];
    format!(
        "{}, {} de {}",
        days[date.weekday().num_days_from_monday() as usize],
        date.day(),
        months[date.month0() as usize]
    )
}

fn notice(code: &str) -> Option<(&'static str, bool)> {
    Some(match code {
        "welcome" => (
            "¡Qué bueno tenerte aquí! Ya puedes reservar un espacio.",
            true,
        ),
        "reserved" => (
            "Tu reserva está confirmada. ¡Nos vemos en la facultad!",
            true,
        ),
        "cancelled" => (
            "Reserva cancelada. El horario ya está disponible para otro estudiante.",
            true,
        ),
        "overlap" => (
            "Ese espacio acaba de ser reservado en ese horario. Elige otro horario disponible.",
            false,
        ),
        "student_overlap" => (
            "Ya tienes una reserva en ese horario. No puedes reservar dos espacios a la vez.",
            false,
        ),
        "date" => (
            "Elige un horario futuro, dentro de los próximos 30 días.",
            false,
        ),
        "hours" => (
            "Reserva de 08:00 a 22:00, en intervalos de 30 minutos y por un máximo de 3 horas.",
            false,
        ),
        "details" => (
            "Indica una actividad de 5 a 180 caracteres y al menos un participante.",
            false,
        ),
        "capacity" => (
            "El espacio no existe o el número de participantes supera su capacidad.",
            false,
        ),
        "credentials" => (
            "El código o la contraseña no coinciden. Inténtalo nuevamente.",
            false,
        ),
        "roster" => (
            "No encontramos ese código y correo en el padrón activo FISI. Consulta con la administración de tu facultad.",
            false,
        ),
        "password" => ("Usa una contraseña de entre 10 y 128 caracteres.", false),
        "registered" => (
            "Ya tienes una cuenta. Ingresa con tu código de estudiante.",
            false,
        ),
        "session" => ("Ingresa a tu cuenta para continuar con la reserva.", false),
        "cannot_cancel" => (
            "No se pudo cancelar: la reserva ya comenzó, fue cancelada o no te pertenece.",
            false,
        ),
        "server" => (
            "No pudimos guardar la reserva. Inténtalo nuevamente en un momento.",
            false,
        ),
        _ => return None,
    })
}

fn busy(
    bookings: &[Booking],
    space: i16,
    date: NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
) -> bool {
    let start = db::at(date, start);
    let end = db::at(date, end);
    bookings
        .iter()
        .any(|b| b.space_id == space && b.starts_at < end && b.ends_at > start)
}

#[page("/")]
pub async fn home(cx: &Cx) -> Result<View> {
    let app: &App = app_context(cx);
    let query: Query = parse_query_params(cx)?;
    let today = db::today();
    let max_date = today + Duration::days(30);
    let date = query.date.unwrap_or(today).clamp(today, max_date);
    let student = auth::current(cx).await?;
    let spaces = db::spaces(&app.pool).await?;
    let bookings = db::day_bookings(&app.pool, date).await?;
    let mine = if let Some(student) = &student {
        db::my_bookings(&app.pool, student.id).await?
    } else {
        vec![]
    };
    let my_view = query.view == "my";
    let active_count = mine
        .iter()
        .filter(|b| b.cancelled_at.is_none() && b.ends_at > Utc::now())
        .count();
    let modal_space = query
        .booking
        .and_then(|id| spaces.iter().find(|s| s.id == id));
    let show_auth = query.auth == "login"
        || query.auth == "register"
        || (modal_space.is_some() && student.is_none());
    let message = notice(&query.notice);
    let first_name = student
        .as_ref()
        .and_then(|s| s.full_name.split_whitespace().next())
        .unwrap_or("estudiante");
    view! {
        <!DOCTYPE html>
        <html lang="es">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="theme-color" content="#F2FDFF">
                <meta name="description" content="Reserva la cancha de fútbol, el Aula Magna y el Auditorio de la Facultad de Ingeniería de Sistemas e Informática de la Universidad Nacional Mayor de San Marcos. FISI · UNMSM.">
                <title>"ReserFISI · FISI — UNMSM"</title>
                <link rel="icon" href="/assets/logo-fisi.png" type="image/png">
                <link rel="stylesheet" href="/assets/style.css?v=2">
                <script src="/assets/app.js?v=1" defer=""></script>
            </head>
            <body>
                <a class="skip-link" href="#contenido">"Ir al contenido"</a>
                <header class="site-header">
                    <div class="container header-inner">
                        <a class="brand" href="/" aria-label="ReserFISI · FISI UNMSM, inicio">
                            <img class="faculty-logo" src="/assets/logo-fisi.png" alt="Logo de la Facultad de Ingeniería de Sistemas e Informática de la UNMSM" width="195" height="176">
                            <span>"Reser"<strong>"FISI"</strong><small>"FISI · UNMSM"</small></span>
                        </a>
                        <nav class="main-nav" aria-label="Navegación principal">
                            <a href="/" if !my_view { aria-current="page" }>"Explorar espacios"</a>
                            <a href="/?view=my#mis-reservas" if my_view { aria-current="page" }>"Mis reservas" if active_count > 0 { <span class="nav-count">(active_count)</span> }</a>
                            <a href="/#como-funciona">"Cómo funciona"</a>
                        </nav>
                        if let Some(student) = &student {
                            <details class="profile-menu"><summary><span class="avatar">(first_name.chars().next().unwrap_or('E'))</span><span>(first_name)</span><span aria-hidden="true">"⌄"</span></summary>
                                <div class="profile-dropdown"><strong>(&student.full_name)</strong><small>"Estudiante · "(&student.student_code)</small><form method="post" action="/auth/logout"><input type="hidden" name="csrf" value=(&student.csrf_token)><button class="text-button" type="submit">"Cerrar sesión ↗"</button></form></div>
                            </details>
                        } else {
                            <a href="/?auth=login" class="button button-outline header-login">"Ingresar"<span aria-hidden="true">"↗"</span></a>
                        }
                    </div>
                </header>
                <main id="contenido" class="container">
                    if let Some((text, success)) = message {
                        <div class="notice" role="status" if !success { data-error="true" }><span aria-hidden="true">(if success { "✓" } else { "!" })</span><p>(text)</p><a href=(if my_view { "/?view=my" } else { "/" }) aria-label="Cerrar aviso">"×"</a></div>
                    }
                    if !my_view {
                        <section class="hero" aria-labelledby="hero-title">
                            <div class="hero-copy">
                                <p class="eyebrow"><span class="little-star" aria-hidden="true">"✳"</span>"SANMARQUINOS, ESTE ES NUESTRO LUGAR"</p>
                                <h1 id="hero-title">"Un espacio para"<br><span class="marker">"lo que nos une."</span></h1>
                                <p class="hero-faculty">"Facultad de Ingeniería de Sistemas e Informática"</p>
                                <p class="hero-description">"Una idea, un partido, una gran conversación. Encuentra tu espacio en la FISI de la Universidad Nacional Mayor de San Marcos."</p>
                                <div class="hero-actions"><a class="button button-primary" href="#espacios">"Encuentra tu espacio"<span aria-hidden="true">"↗"</span></a><span class="hero-note">"De estudiantes,"<br>"para estudiantes."</span></div>
                            </div>
                            <figure class="hero-art campus-photos">
                                <div class="campus-photo-main"><img src="/assets/fisi.jpg" alt="Fachada y entrada de la Facultad de Ingeniería de Sistemas e Informática de la UNMSM" width="617" height="324" fetchpriority="high"><span class="campus-photo-label">"NUESTRA FISI"<span class="little-star" aria-hidden="true">"✳"</span></span></div>
                                <div class="campus-photo-inset"><img src="/assets/fisi2.jpg" alt="Jardines y pabellón de la FISI en la Ciudad Universitaria de San Marcos" width="446" height="448"></div>
                                <figcaption class="campus-caption"><span class="campus-location">"UNMSM · Ciudad Universitaria, Lima"</span><span class="campus-handwritten">"Aquí pasan cosas buenas."<span aria-hidden="true">"↗"</span></span></figcaption>
                            </figure>
                        </section>
                        <div class="community-strip"><span class="community-label"><span class="small-spark" aria-hidden="true">"✳"</span>"El campus es de todos. Hagamos espacio."</span><span><strong>"03"</strong>" espacios, muchas posibilidades"</span><span class="strip-end"><span class="outline-check" aria-hidden="true">"✓"</span>"Sin cruces. Sin complicaciones."</span></div>
                        <section id="espacios" class="spaces-section" aria-labelledby="spaces-title">
                            <div class="section-heading"><div><p class="eyebrow">"ELIGE DÓNDE ENCONTRARNOS"</p><h2 id="spaces-title">"¿Qué tienes en mente?"</h2></div><span class="section-aside">"Un lugar para cada plan."<span aria-hidden="true">"↙"</span></span></div>
                            <div class="filters">
                                <div class="filter-chips" aria-label="Filtrar espacios por categoría">
                                    for (value, label) in [("", "Todos los espacios"), ("Deporte", "Deporte"), ("Académico", "Académico"), ("Cultural", "Cultural")] {
                                        <a href=(format!("/?date={date}&category={value}#espacios")) class=(if query.category == value { "chip selected" } else { "chip" }) if query.category == value { aria-current="true" }>(label)</a>
                                    }
                                </div>
                                <form method="get" action="/#espacios" class="date-filter"><input type="hidden" name="category" value=(&query.category)><label for="explore-date">"Para el"</label><input id="explore-date" type="date" name="date" value=(date.to_string()) min=(today.to_string()) max=(max_date.to_string()) required=""><button type="submit" class="date-submit" aria-label="Consultar fecha">"→"</button></form>
                            </div>
                            <div class="space-grid">
                                for space in spaces.iter().filter(|s| query.category.is_empty() || s.category == query.category) {
                                    space_card(space: space, date: date, bookings: &bookings)
                                }
                            </div>
                            if !query.category.is_empty() && !spaces.iter().any(|s| s.category == query.category) { <p class="empty-state">"No hay espacios en esta categoría. "<a href="/#espacios">"Ver todos los espacios →"</a></p> }
                        </section>
                        <section id="agenda" class="agenda-section" aria-labelledby="agenda-title">
                            <div class="section-heading"><div><p class="eyebrow">"UN VISTAZO A TU DÍA"</p><h2 id="agenda-title">"Hagamos lugar en la agenda."</h2></div><span class="timezone">"Hora de Lima · UTC−5"</span></div>
                            <div class="agenda-layout">
                                <div class="agenda-panel">
                                    <div class="agenda-toolbar"><div><strong>(date_label(date))</strong><span>"Elige un bloque para reservar"</span></div><div class="day-navigation">
                                        if date > today { <a href=(format!("/?date={}#agenda", date - Duration::days(1))) aria-label="Día anterior">"←"</a> }
                                        <a href="/#agenda" class="today-link">"Hoy"</a>
                                        if date < max_date { <a href=(format!("/?date={}#agenda", date + Duration::days(1))) aria-label="Día siguiente">"→"</a> }
                                    </div></div>
                                    <div class="schedule-scroll" data-today=(if date == today { "true" } else { "false" }) tabindex="0" role="region" aria-label="Horarios por espacio; desplázate horizontalmente en pantallas pequeñas">
                                        <table class="schedule"><thead><tr><th scope="col">"HORA"</th>for space in &spaces { <th scope="col">(&space.name)</th> }</tr></thead><tbody>
                                            for hour in 8..22 {
                                                <tr><th scope="row">(format!("{hour:02}:00"))</th>
                                                    for space in &spaces {
                                                        schedule_cell(space: space, date: date, hour: hour, bookings: &bookings)
                                                    }
                                                </tr>
                                            }
                                        </tbody></table>
                                    </div>
                                    <div class="schedule-legend"><span><i class="legend-free"></i>"Disponible"</span><span><i class="legend-busy"></i>"Reservado o parcialmente ocupado"</span></div>
                                </div>
                                <aside class="kind-note"><span class="note-star" aria-hidden="true">"✳"</span><p class="eyebrow">"UN PEQUEÑO RECORDATORIO"</p><h3>"Compartir también"<br>"es cuidar."</h3><p>"Detrás de cada reserva hay alguien con un plan. Si el tuyo cambia, libera tu espacio para que otro lo disfrute."</p><div class="note-divider"></div><p class="note-detail">"De 08:00 a 22:00"<br>"Hasta 3 horas por reserva"<br>"Con 30 días de anticipación"</p><a href="#como-funciona">"Conoce cómo funciona ↗"</a></aside>
                            </div>
                        </section>
                        <section class="how-section" id="como-funciona" aria-labelledby="how-title"><div class="section-heading"><div><p class="eyebrow">"MENOS TRÁMITES, MÁS ENCUENTROS"</p><h2 id="how-title">"Tu próximo plan, en tres pasos."</h2></div><span class="hand-star" aria-hidden="true">"✳"</span></div><div class="steps">
                            <article><span class="step-number">"01"</span><h3>"Encuentra tu lugar"</h3><p>"Explora los espacios y consulta los horarios disponibles para tu día."</p></article>
                            <article><span class="step-number">"02"</span><h3>"Hazlo tuyo por un rato"</h3><p>"Ingresa con tu cuenta FISI, elige tu horario y cuéntanos qué estás planeando."</p></article>
                            <article><span class="step-number">"03"</span><h3>"Nos vemos ahí"</h3><p>"Tu reserva se confirma al instante. Llega a tiempo, comparte y cuida el espacio."</p></article>
                        </div></section>
                        <section class="faq-section" aria-labelledby="faq-title"><h2 id="faq-title">"Por si te lo preguntabas…"</h2><div class="faq-list">
                            <details><summary>"¿Quién puede reservar?"<span aria-hidden="true">"+"</span></summary><p>"Cualquier estudiante activo de la Facultad de Ingeniería de Sistemas e Informática (FISI) de la Universidad Nacional Mayor de San Marcos (UNMSM) incluido en el padrón local. Crea tu cuenta con tu código y el correo registrado por la facultad. Si no apareces, contacta con la administración."</p></details>
                            <details><summary>"¿Cuánto tiempo puedo reservar?"<span aria-hidden="true">"+"</span></summary><p>"Entre 30 minutos y 3 horas, de 08:00 a 22:00. Puedes reservar hasta con 30 días de anticipación. Todos los horarios corresponden a Lima (UTC−5)."</p></details>
                            <details><summary>"¿Qué pasa si cambian mis planes?"<span aria-hidden="true">"+"</span></summary><p>"Puedes cancelar desde Mis reservas antes de que comience tu horario. El espacio queda libre inmediatamente. Para cambiar una reserva, cancela la anterior y elige un nuevo horario."</p></details>
                        </div></section>
                    } else {
                        <section class="my-section" id="mis-reservas"><p class="eyebrow">"TUS PRÓXIMOS ENCUENTROS"</p><div class="section-heading"><h1>"Tus planes tienen lugar."</h1><a class="button button-primary" href="/#espacios">"Nueva reserva ↗"</a></div>
                            if let Some(student) = &student {
                                <p class="muted my-intro">(format!("Hola, {first_name}. Aquí puedes consultar tus reservas y liberar un espacio si cambian tus planes."))</p>
                                if mine.is_empty() { <div class="empty-state"><span class="empty-star" aria-hidden="true">"✳"</span><h2>"El próximo encuentro lo decides tú."</h2><p>"Todavía no tienes reservas. Hay un espacio esperando tu próxima idea."</p><a href="/#espacios" class="button button-primary">"Explorar espacios ↗"</a></div> }
                                <div class="my-bookings">for booking in &mine { my_booking(booking: booking, csrf: &student.csrf_token) }</div>
                            } else {
                                <div class="empty-state"><span class="empty-star" aria-hidden="true">"✳"</span><h2>"Un lugar para tus planes."</h2><p>"Ingresa con tu cuenta de estudiante para ver y gestionar tus reservas."</p><a href="/?auth=login" class="button button-primary">"Ingresar a mi cuenta ↗"</a></div>
                            }
                        </section>
                    }
                </main>
                <footer class="site-footer"><div class="container footer-inner"><a class="footer-brand" href="/" aria-label="ReserFISI · FISI UNMSM, inicio"><img class="footer-logo" src="/assets/logo-fisi.png" alt="Logo FISI" width="195" height="176" loading="lazy"><span>"Reser"<strong>"FISI"</strong><small>"ESPACIOS QUE NOS UNEN"</small></span></a><p><strong>"Facultad de Ingeniería de Sistemas e Informática"</strong><span>"Universidad Nacional Mayor de San Marcos"</span><span class="footer-community">"Un lugar para aprender. Una comunidad para crecer."</span></p><span class="footer-note">"Hecho para encontrarnos."<span class="little-star" aria-hidden="true">"✳"</span></span></div></footer>
                if show_auth {
                    auth_dialog(register: query.auth == "register", message: message.map(|m| m.0))
                } else if let (Some(space), Some(student)) = (modal_space, &student) {
                    booking_dialog(space: space, date: date, start: query.start, csrf: &student.csrf_token, bookings: &bookings)
                }
            </body>
        </html>
    }
}

#[component]
async fn space_card(space: &Space, date: NaiveDate, bookings: &[Booking]) -> Result<View> {
    let free = (8..22)
        .filter(|&h| {
            let start = NaiveTime::from_hms_opt(h, 0, 0).unwrap();
            db::at(date, start) > Utc::now()
                && !busy(
                    bookings,
                    space.id,
                    date,
                    start,
                    NaiveTime::from_hms_opt(h + 1, 0, 0).unwrap(),
                )
        })
        .count();
    view! {
        <article class="space-card">
            <div class=(format!("card-art art-{}", space.illustration))><span class="category-label">(&space.category)</span><img src=(format!("/assets/spaces.svg#{}", space.illustration)) alt=(format!("Ilustración de {}", space.name)) width="500" height="260" loading="lazy"></div>
            <div class="card-body"><div class="card-title-row"><h3>(&space.name)</h3><span class="card-number">(format!("0{}", space.id))</span></div><p class="space-description">(&space.description)</p><div class="space-meta"><span><span aria-hidden="true">"♧"</span>(format!("Hasta {} personas", space.capacity))</span><span><span aria-hidden="true">"⌖"</span>(&space.location)</span><span><span aria-hidden="true">"◇"</span>(&space.amenity)</span></div>
                <div class="card-bottom"><span class="availability"><i></i>(if free > 0 { format!("{free} bloques de 1 h libres") } else { "Consulta bloques de 30 min".into() })</span><a class="card-reserve" href=(format!("/?date={date}&booking={}", space.id)) aria-label=(format!("Reservar {}", space.name))>"Reservar"<span aria-hidden="true">"↗"</span></a></div>
            </div>
        </article>
    }
}

#[component]
async fn schedule_cell(
    space: &Space,
    date: NaiveDate,
    hour: u32,
    bookings: &[Booking],
) -> Result<View> {
    let start = NaiveTime::from_hms_opt(hour, 0, 0).unwrap();
    let end = NaiveTime::from_hms_opt(hour + 1, 0, 0).unwrap();
    let occupied = busy(bookings, space.id, date, start, end);
    let past = db::at(date, start) <= Utc::now();
    view! {
        <td>
            if past { <span class="slot slot-past">"Finalizado"</span> }
            else if occupied { <span class="slot slot-busy">"Reservado"</span> }
            else { <a class="slot slot-free" href=(format!("/?date={date}&booking={}&start={hour:02}:00", space.id)) aria-label=(format!("Reservar {} a las {hour}:00", space.name))><span>"Disponible"</span><span aria-hidden="true">"+"</span></a> }
        </td>
    }
}

#[component]
async fn my_booking(booking: &Booking, csrf: &str) -> Result<View> {
    let cancelled = booking.cancelled_at.is_some();
    let past = booking.ends_at <= Utc::now();
    let can_cancel = !cancelled && booking.starts_at > Utc::now();
    view! {
        <article class="my-booking"><div class="booking-date"><strong>(booking.starts_at.with_timezone(&db::lima()).format("%d").to_string())</strong><span>(booking.starts_at.with_timezone(&db::lima()).format("%m / %Y").to_string())</span></div><div class="booking-info"><span class="booking-status">(if cancelled { "Cancelada" } else if past { "Finalizada" } else { "Confirmada" })</span><h3>(&booking.space_name)</h3><p>(&booking.purpose)</p><span class="muted">(format!("{} – {} · {} participantes", db::time_label(booking.starts_at), db::time_label(booking.ends_at), booking.attendees))</span><small class="booking-reference">(format!("Referencia: {}", booking.id))</small></div>
            if can_cancel { <form method="post" action="/reservations/cancel" data-confirm="¿Cancelar esta reserva? El horario quedará libre para otro estudiante."><input type="hidden" name="csrf" value=(csrf)><input type="hidden" name="id" value=(booking.id.to_string())><button class="button button-outline" type="submit">"Cancelar reserva"</button></form> }
        </article>
    }
}

#[component]
async fn auth_dialog(register: bool, message: Option<&str>) -> Result<View> {
    view! {
        <div class="modal-backdrop"><section class="modal auth-modal" role="dialog" aria-modal="true" aria-labelledby="auth-title" tabindex="-1"><a href="/" class="modal-close" aria-label="Cerrar">"×"</a><img class="auth-logo" src="/assets/logo-fisi.png" alt="Logo de la FISI de la UNMSM" width="195" height="176"><p class="eyebrow">"TU COMUNIDAD FISI · UNMSM"</p><h2 id="auth-title">(if register { "Aquí comienza tu próximo plan." } else { "Qué bueno verte de nuevo." })</h2><p class="muted">(if register { "Crea tu cuenta con los datos del padrón de estudiantes de la FISI de San Marcos." } else { "Ingresa con tu código de estudiante sanmarquino. Tu próximo encuentro está cerca." })</p>
            if let Some(message) = message { <p class="form-message" role="status">(message)</p> }
            <form method="post" action=(if register { "/auth/register" } else { "/auth/login" }) class="stack-form"><label for="student-code">"Código de estudiante"</label><input id="student-code" name="student_code" inputmode="numeric" pattern="[0-9]{6,12}" minlength="6" maxlength="12" autocomplete="username" placeholder="Tu código de estudiante, ejemplo:3" required="">
                if register { <label for="email">"Correo registrado en el padrón"</label><input id="email" name="email" type="email" autocomplete="email" maxlength="254" placeholder="Tu correo de estudiante" required=""> }
                <label for="password">"Contraseña"</label><input id="password" name="password" type="password" autocomplete=(if register { "new-password" } else { "current-password" }) if register { minlength="10" } maxlength="128" placeholder=(if register { "Al menos 10 caracteres" } else { "Tu contraseña" }) required="">
                <button class="button button-primary full-width" type="submit">(if register { "Crear mi cuenta" } else { "Ingresar" })<span aria-hidden="true">"↗"</span></button>
            </form><p class="auth-switch">(if register { "¿Ya tienes cuenta? " } else { "¿Primera vez por aquí? " })<a href=(if register { "/?auth=login" } else { "/?auth=register" })>(if register { "Ingresa" } else { "Crea tu cuenta" })</a></p><p class="form-footnote">"Facultad de Ingeniería de Sistemas e Informática"<br>"Universidad Nacional Mayor de San Marcos"<br>"Solo para estudiantes activos de la FISI."</p>
        </section></div>
    }
}

#[component]
async fn booking_dialog(
    space: &Space,
    date: NaiveDate,
    start: Option<NaiveTime>,
    csrf: &str,
    bookings: &[Booking],
) -> Result<View> {
    let today = db::today();
    let max_date = today + Duration::days(30);
    let requested_start = start.filter(|t| {
        *t >= NaiveTime::from_hms_opt(8, 0, 0).unwrap()
            && *t < NaiveTime::from_hms_opt(22, 0, 0).unwrap()
            && t.minute() % 30 == 0
            && t.second() == 0
            && t.nanosecond() == 0
    });
    let suggested_start = (16..44)
        .map(|half| NaiveTime::from_hms_opt(half / 2, (half % 2) * 30, 0).unwrap())
        .find(|&time| {
            db::at(date, time) > Utc::now()
                && !busy(bookings, space.id, date, time, time + Duration::minutes(30))
        });
    let start = requested_start
        .or(suggested_start)
        .unwrap_or(NaiveTime::from_hms_opt(8, 0, 0).unwrap());
    let hour_end = start + Duration::hours(1);
    let end = if hour_end <= NaiveTime::from_hms_opt(22, 0, 0).unwrap()
        && !busy(bookings, space.id, date, start, hour_end)
    {
        hour_end
    } else {
        start + Duration::minutes(30)
    };
    view! {
        <div class="modal-backdrop"><section class="modal booking-modal" role="dialog" aria-modal="true" aria-labelledby="booking-title" tabindex="-1"><a href=(format!("/?date={date}#agenda")) class="modal-close" aria-label="Cerrar">"×"</a><p class="eyebrow">"HAGAMOS REALIDAD ESE PLAN"</p><h2 id="booking-title">"Reserva tu espacio."</h2><div class="selected-space"><img src=(format!("/assets/spaces.svg#{}", space.illustration)) alt="" width="100" height="60"><div><strong>(&space.name)</strong><small>(format!("{} · Hasta {} personas", space.location, space.capacity))</small></div></div>
            if suggested_start.is_none() { <p class="form-message">"No quedan bloques disponibles para este día. Elige otra fecha para tu encuentro."</p> }
            <form method="post" action="/reservations" class="stack-form"><input type="hidden" name="csrf" value=(csrf)><input type="hidden" name="space_id" value=(space.id.to_string())><label for="booking-date">"¿Qué día nos encontramos?"</label><input id="booking-date" type="date" name="date" value=(date.to_string()) min=(today.to_string()) max=(max_date.to_string()) required="">
                <div class="form-row"><div><label for="start">"Desde"</label><select id="start" name="start" required="">for half in 16..44 { <option value=(format!("{:02}:{:02}", half / 2, (half % 2) * 30)) if NaiveTime::from_hms_opt(half / 2, (half % 2) * 30, 0).unwrap() == start { selected="" }>(format!("{:02}:{:02}", half / 2, (half % 2) * 30))</option> }</select></div><div><label for="end">"Hasta"</label><select id="end" name="end" required="">for half in 17..=44 { <option value=(format!("{:02}:{:02}", half / 2, (half % 2) * 30)) if NaiveTime::from_hms_opt(half / 2, (half % 2) * 30, 0).unwrap() == end { selected="" }>(format!("{:02}:{:02}", half / 2, (half % 2) * 30))</option> }</select></div></div>
                <p class="input-hint">"Hora de Lima · Entre 30 minutos y 3 horas por reserva."</p>
                <label for="purpose">"¿Qué estás planeando?"</label><input id="purpose" name="purpose" minlength="5" maxlength="180" placeholder="Ej. Encuentro de estudiantes de primer ciclo" required="">
                <label for="attendees">"¿Cuántas personas participarán?"</label><input id="attendees" name="attendees" type="number" min="1" max=(space.capacity) value="1" required="">
                <p class="booking-promise"><span aria-hidden="true">"✓"</span>"Al confirmar, el horario queda reservado para ti. Si alguien se adelanta, te avisaremos para que elijas otro."</p><button class="button button-primary full-width" type="submit">"Confirmar mi reserva"<span aria-hidden="true">"↗"</span></button>
            </form>
        </section></div>
    }
}
