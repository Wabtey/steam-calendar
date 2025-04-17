use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
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
    // let mut end_time: DateTime<Utc>;

    loop {
        match reqwest::get(&player_summaries_url).await {
            Ok(response) => {
                if let Ok(player_summary) = response.json::<PlayerSummaryResponse>().await {
                    let player = &player_summary.response.players[0];
                    let game_currently_played = player.gameextrainfo.as_ref();

                    let start_of_session =
                        currently_playing.is_none() && game_currently_played.is_some();
                    let end_of_session =
                        currently_playing.is_some() && game_currently_played.is_none();

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
                        if let Err(e) = create_game_session_event(
                            config.clone(),
                            &game_id,
                            game_info,
                            start_time,
                            end_time,
                        )
                        .await
                        {
                            eprintln!("Error creating event: {}", e);
                        }
                    }
                    currently_playing = None; // = game_currently_played.clone();
                }
            }
            Err(e) => eprintln!("API request failed: {}", e),
        }

        wait_x_seconds(&config.timezone, config.delay).await;
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
        delay: 5 * 60,
        // TODO: feat - ask one time for user credentials to connect to CalDAV
        caldav: match (
            env::var("CALDAV_PROVIDER").ok(),
            env::var("CALDAV_USERNAME").ok(),
            env::var("CALDAV_PASSWORD").ok(),
        ) {
            (Some(p), Some(u), Some(pw)) => Some(CalDAV {
                provider: p,
                username: u,
                password: pw,
            }),
            _ => {
                println!(
                    "CalDAV credentials incomplete, publishing disabled (missing either in the .env the provider, username or password)"
                );
                None
            }
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
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

/* -------------------------------------------------------------------------- */
/*                                    Utils                                   */
/* -------------------------------------------------------------------------- */

async fn wait_x_seconds(timezone: &Tz, delay: u64) {
    let next_update = Utc::now() + Duration::seconds(delay as i64);
    println!(
        "Next update: {}",
        next_update.with_timezone(timezone).format("%H:%M:%S")
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
}
