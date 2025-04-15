use calendar::create_game_session_event;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::{Europe, Tz};
use dotenv::dotenv;
use std::env;

use api::i_player_service::PlayerSummaryResponse;

mod api;
mod calendar;
mod session_achievements;

#[derive(Clone, Copy, Debug)]
pub struct Config<'a> {
    pub api_key: &'a str,
    pub steam_id: &'a str,
    pub timezone: &'a Tz,
    pub calendar_path: &'a str,
}

// REFACTOR: write logs down

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = &env::var("STEAM_API_KEY")
        .expect("STEAM_API_KEY must be set in environment variables (`/.env`)");
    let steam_id = &env::var("STEAM_USER_ID")
        .expect("STEAM_USER_ID must be set in environment variables (`/.env`)");
    let calendar_path = "game_sessions.ics";
    // TODO: feat - ask for/detect timezone
    let timezone = &Europe::Paris;
    // TODO: ask delay frequency
    let delay: u64 = 5 * 60;

    let config = Config {
        api_key,
        steam_id,
        timezone,
        calendar_path,
    };

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
        let _calendar =
            create_game_session_event(config, &game_id, &game_info, start_time, end_time).await?;
        // publish_game_session_calendar();

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
