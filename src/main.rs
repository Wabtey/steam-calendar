#![warn(clippy::pedantic)]

use actix_web::{App, HttpResponse, HttpServer, Responder, get, put, web};
use calendar::create_game_session_event;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::{Europe, Tz};
use dotenv::dotenv;
use icalendar::Calendar;
use std::{env, fs, path::Path, str::FromStr, sync::Arc};

use api::i_player_service::PlayerSummaryResponse;

mod api;
mod calendar;
mod session_achievements;

/* -------------------------------------------------------------------------- */
/*                                   Config                                   */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub steam_id: String,
    pub timezone: Tz,
    pub calendar_path: String,
    pub delay: u64,
    pub caldav: Option<CalDAV>,
}

#[derive(Clone, Debug)]
pub struct CalDAV {
    pub provider: String,
    pub username: String,
    /// FIXME: storing the password here doesn't seems right for some reason
    pub password: String,
}

struct AppState {
    config: Config,
}

// REFACTOR: write logs down

/* -------------------------------------------------------------------------- */
/*                                  Endpoints                                 */
/* -------------------------------------------------------------------------- */

#[get("/calendar.ics")]
async fn ics_endpoint(data: web::Data<Arc<AppState>>) -> impl Responder {
    let config = &data.config;

    let blank_calendar = Calendar::new()
        .name("Game Sessions")
        // .version("2.0")
        .done();
    let calendar = if Path::new(&config.calendar_path).exists() {
        let calendar_data = fs::read_to_string(config.calendar_path.clone()).unwrap_or_default();
        Calendar::from_str(&calendar_data).unwrap_or(blank_calendar)
    } else {
        blank_calendar
    };
    let ics_content = calendar.to_string();

    HttpResponse::Ok()
        .content_type("text/calendar; charset=utf-8")
        .body(ics_content)
}

#[put("/calendar.ics")]
async fn write_ics_endpoint(data: web::Data<Arc<AppState>>, body: web::Bytes) -> impl Responder {
    let config = &data.config;

    /* --------------------------------- backup --------------------------------- */
    if Path::new(&config.calendar_path).exists() {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("{}.{}.bak", config.calendar_path, timestamp);
        if let Err(e) = fs::copy(&config.calendar_path, &backup_path) {
            eprintln!("Failed to create backup: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    /* ---------------------------- incoming calendar --------------------------- */
    let incoming_cal = match String::from_utf8(body.to_vec()) {
        Ok(s) => match Calendar::from_str(&s) {
            Ok(cal) => cal,
            Err(_) => return HttpResponse::BadRequest().body("Invalid calendar format"),
        },
        Err(_) => return HttpResponse::BadRequest().body("Invalid UTF-8"),
    };

    /* ---------------------------- existing calendar --------------------------- */
    let existing_cal = if Path::new(&config.calendar_path).exists() {
        match fs::read_to_string(&config.calendar_path) {
            Ok(data) => Calendar::from_str(&data).unwrap_or_else(|_| Calendar::new().done()),
            Err(_) => Calendar::new().done(),
        }
    } else {
        Calendar::new().done()
    };

    /* ---------------------------------- merge --------------------------------- */
    let mut merged_cal = Calendar::new();
    for component in existing_cal.components {
        merged_cal.push(component);
    }
    for component in incoming_cal.components {
        merged_cal.push(component);
    }
    let merged_cal = merged_cal.done();

    /* ---------------------------------- save ---------------------------------- */
    if let Err(e) = fs::write(&config.calendar_path, merged_cal.to_string()) {
        eprintln!("Failed to write calendar: {e}");
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok()
        .content_type("text/calendar; charset=utf-8")
        .body(merged_cal.to_string())
}

/* -------------------------------------------------------------------------- */
/*                                  Services                                  */
/* -------------------------------------------------------------------------- */

async fn steam_tracking_loop(app_state: Arc<AppState>) {
    let config = &app_state.config;
    let player_summaries_url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
        config.api_key, config.steam_id
    );

    // TOTEST: switching while being on between two games
    let mut currently_playing: Option<String> = None;
    let mut game_id = String::new();
    let mut start_time: DateTime<Utc> = Utc::now();
    let mut skip_waiting = false;

    loop {
        match reqwest::get(&player_summaries_url).await {
            Err(e) => eprintln!("API request failed: {e}"),
            Ok(response) => {
                if let Ok(player_summary) = response.json::<PlayerSummaryResponse>().await {
                    if player_summary.response.players.is_empty() {
                        eprintln!("API request failed: No players returned.");
                        continue;
                    }
                    let player = &player_summary.response.players[0];
                    let game_currently_played = player.gameextrainfo.as_ref();

                    let start_of_session =
                        currently_playing.is_none() && game_currently_played.is_some();
                    let end_of_session = currently_playing.is_some()
                        && (game_currently_played.is_none() || {
                            let past_game = currently_playing.clone().unwrap();
                            let current_game = game_currently_played.unwrap().as_ref();
                            // println!("past: {past_game}, current: {current_game}");
                            let switched_game = past_game != current_game;
                            skip_waiting = switched_game;
                            switched_game
                        });
                    // println!("start: {start_of_session}, end: {end_of_session}");

                    if start_of_session {
                        start_time = Utc::now();
                        currently_playing = game_currently_played.cloned();
                        game_id = player
                            .gameid
                            .clone()
                            .unwrap_or("No game ID found".to_string());
                        println!(
                            "{}: Started playing at {}",
                            start_time
                                .with_timezone(&config.timezone)
                                .format("%H:%M:%S"),
                            currently_playing.as_ref().unwrap()
                        );
                    }

                    if !end_of_session {
                        wait_x_seconds(&config.timezone, config.delay).await;
                        continue;
                    }

                    let end_time = Utc::now();
                    /* -------------------------------------------------------------------------- */
                    /*                 Create the Event at the end of the session                 */
                    /* -------------------------------------------------------------------------- */
                    if let Some(game_info) = &currently_playing {
                        let _event = create_game_session_event(
                            config.clone(),
                            &game_id,
                            game_info,
                            start_time,
                            end_time,
                        )
                        .await;
                    }
                    currently_playing = None; // = game_currently_played.clone();
                }
            }
        }

        if !skip_waiting {
            wait_x_seconds(&config.timezone, config.delay).await;
            skip_waiting = true;
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Server                                   */
/* -------------------------------------------------------------------------- */

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config = Config {
        api_key: env::var("STEAM_API_KEY").expect("STEAM_API_KEY missing"),
        steam_id: env::var("STEAM_USER_ID").expect("STEAM_USER_ID missing"),
        // TODO: feat - ask for/detect timezone
        timezone: Europe::Paris,
        calendar_path: "game_sessions.ics".to_string(),
        // TODO: feat - ask delay frequency
        delay: 60,
        // TODO: feat - ask one time for user credentials to connect to CalDAV
        caldav: if let (Some(p), Some(u), Some(pw)) = (
            env::var("CALDAV_PROVIDER").ok(),
            env::var("CALDAV_USERNAME").ok(),
            env::var("CALDAV_PASSWORD").ok(),
        ) {
            Some(CalDAV {
                provider: p,
                username: u,
                password: pw,
            })
        } else {
            println!(
                "CalDAV credentials incomplete, publishing disabled (missing either in the .env the provider, username or password)"
            );
            None
        },
    };

    let app_state = Arc::new(AppState { config });

    /* ---------------------------------------------------------- */
    // println!("steam-gunfire-reborn-{}", Utc::now().timestamp());
    // let event = Event::new()
    //     .summary("Gunfire Reborn")
    //     .description("This is a test\nhaha")
    //     .starts(Utc::now())
    //     // .class(Class::Confidential)
    //     .ends(Utc::now() + Duration::minutes(90))
    //     .uid(&format!("steam-gunfire-reborn-{}", Utc::now().timestamp()))
    //     .done();
    // let caldav = config.clone().caldav.unwrap();
    // publish_to_nextcloud(&event, &caldav.provider, &caldav.username, &caldav.password).await?;
    /* ---------------------------------------------------------- */

    let bg_state = app_state.clone();
    tokio::spawn(async move {
        steam_tracking_loop(bg_state).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(ics_endpoint)
            .service(write_ics_endpoint)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

/* -------------------------------------------------------------------------- */
/*                                    Utils                                   */
/* -------------------------------------------------------------------------- */

/// TODO: random delay?
async fn wait_x_seconds(timezone: &Tz, delay: u64) {
    let next_update = Utc::now() + Duration::seconds(delay as i64);
    println!(
        "Next update: {}",
        next_update.with_timezone(timezone).format("%H:%M:%S")
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
}
