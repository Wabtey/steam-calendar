use calendar::create_game_session_event;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::{Europe, Tz};
use dotenv::dotenv;
use std::env;

use api::i_player_service::PlayerSummaryResponse;

mod api;
mod calendar;
mod session_achievements;

#[derive(Clone, Debug)]
pub struct Config<'a> {
    pub api_key: &'a str,
    pub steam_id: &'a str,
    pub timezone: &'a Tz,
    pub calendar_path: &'a str,
    pub caldav: Option<CalDAV>,
}

#[derive(Clone, Debug)]
pub struct CalDAV {
    pub provider: String,
    pub username: String,
    /// FIXME: storing the password here doesn't seems right for some reason
    pub password: String,
}

// REFACTOR: write logs down

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /* ------------------------- Config ------------------------- */
    dotenv().ok();
    let api_key = &env::var("STEAM_API_KEY")
        .expect("STEAM_API_KEY must be set in environment variables (`/.env`)");
    let steam_id = &env::var("STEAM_USER_ID")
        .expect("STEAM_USER_ID must be set in environment variables (`/.env`)");
    // TODO: feat - ask for/detect timezone
    let timezone = &Europe::Paris;
    // TODO: feat - ask delay frequency
    let delay: u64 = 5 * 60;
    let calendar_path = "game_sessions.ics";
    // TODO: feat - ask one time for user credentials to connect to CalDAV
    let caldav_provider = env::var("CALDAV_PROVIDER").ok();
    let caldav_username = env::var("CALDAV_USERNAME").ok();
    let caldav_password = env::var("CALDAV_PASSWORD").ok();

    let caldav = if let (Some(provider), Some(username), Some(password)) =
        (caldav_provider, caldav_username, caldav_password)
    {
        Some(CalDAV {
            provider,
            username,
            password,
        })
    } else {
        println!(
            "publication is omitted. (missing either in the .env the provider, username or password)"
        );
        None
    };

    let config = Config {
        api_key,
        steam_id,
        timezone,
        calendar_path,
        caldav,
    };
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

    let player_summaries_url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
        config.api_key, config.steam_id
    );
    // TOTEST: switching while being on between two games
    let mut currently_playing: Option<String> = None;
    let mut game_id = String::new();
    let mut start_time: DateTime<Utc> = Utc::now();
    let mut end_time: DateTime<Utc>;

    let mut watching = true;
    while watching {
        watching = true;
        /* ------------------------------- API request ------------------------------ */
        let response = reqwest::get(&player_summaries_url).await?;
        let player_summary: PlayerSummaryResponse = response.json().await?;
        let game_currently_played = &player_summary.response.players[0].gameextrainfo;

        let start_of_session = currently_playing.is_none() && game_currently_played.is_some();
        let end_of_session = currently_playing.is_some() && game_currently_played.is_none();

        if start_of_session {
            start_time = Utc::now();
            currently_playing = game_currently_played.clone();
            game_id = player_summary.response.players[0]
                .gameid
                .as_ref()
                .expect("No game ID found")
                .to_string();
            println!(
                "{}: Started playing at {}",
                Utc::now().with_timezone(timezone).format("%H:%M:%S"),
                currently_playing.as_ref().unwrap()
            );
        }

        if !end_of_session {
            wait_x_seconds(timezone, delay).await;
            continue;
        }

        end_time = Utc::now();
        /* -------------------------------------------------------------------------- */
        /*                 Create the Event at the end of the session                 */
        /* -------------------------------------------------------------------------- */
        let game_info = currently_playing.clone().unwrap();
        let _event =
            create_game_session_event(config.clone(), &game_id, &game_info, start_time, end_time)
                .await?;
        if let Some(_caldav) = config.clone().caldav {
            // publish_to_nextcloud(&event, &caldav.provider, &caldav.username, &caldav.password)
            //     .await?;
        }

        currently_playing = None; // = game_currently_played.clone();

        wait_x_seconds(timezone, delay).await;
    }

    Ok(())
}

async fn wait_x_seconds(timezone: &Tz, delay: u64) {
    let next_update = Utc::now() + Duration::seconds(delay as i64);
    println!(
        "Next update: {}",
        next_update.with_timezone(timezone).format("%H:%M:%S")
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
}
