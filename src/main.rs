use chrono::{Duration, Utc};
use chrono_tz::Europe;
use dotenv::dotenv;
use icalendar::{Calendar, Component, Event, EventLike};
use session_achievements::get_session_achievement;
use std::{env, fs, path::Path, str::FromStr};

use api::i_player_service::PlayerSummaryResponse;

mod api;
mod session_achievements;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("STEAM_API_KEY")
        .expect("STEAM_API_KEY must be set in environment variables (`/.env`)");
    let user_id = env::var("STEAM_USER_ID")
        .expect("STEAM_USER_ID must be set in environment variables (`/.env`)");

    /* ------------------------------- API request ------------------------------ */
    // TODO: feat - request every 5minutes
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={api_key}&steamids={user_id}",
    );

    let response = reqwest::get(&url).await?;
    let player_summary: PlayerSummaryResponse = response.json().await?;
    println!("{:?}", player_summary.response.players[0].gameextrainfo);

    if let Some(game_info) = &player_summary.response.players[0].gameextrainfo {
        // REFACTOR: create event after the session ended
        /* -------------------------- retrieve achievements ------------------------- */
        let game_id = player_summary.response.players[0]
            .gameid
            .as_ref()
            .expect("No game ID found");
        let achievements = get_session_achievement(
            api_key,
            user_id,
            game_id.to_string(),
            game_info.to_string(),
            (Utc::now() - Duration::days(359)).timestamp(),
            Utc::now().timestamp(),
            &Europe::Paris,
        )
        .await?;

        /* ---------------------------- retrieve calendar --------------------------- */
        let calendar_path = "game_sessions.ics";
        let mut calendar = if Path::new(calendar_path).exists() {
            let calendar_data = fs::read_to_string(calendar_path)?;
            Calendar::from_str(&calendar_data)?
        } else {
            Calendar::new()
                .name("Game Sessions")
                // .version("2.0")
                .done()
        };
        // TODO: feat - send event to provider
        // TODO: feat - ask one time for user credentials to connect to CalDAV
        // TODO: feat - ask for/detect timezone

        /* ----------------------------- push new event ----------------------------- */
        // TODO: feat - add presence of steam friend
        let description = if achievements.is_empty() {
            "No achievements unlocked during this session".to_string()
        } else {
            format!("Achievements:\n{achievements}")
        };

        let event = Event::new()
            .summary(game_info)
            .description(&description)
            .starts(Utc::now())
            // .class(Class::Confidential)
            .ends(Utc::now() + Duration::minutes(20))
            .uid(&format!(
                "steam-{}-{}",
                game_info.to_lowercase().replace(" ", "-"),
                Utc::now().timestamp()
            ))
            .done();

        // println!("{event:#?}");
        calendar.push(event);

        fs::write(calendar_path, calendar.to_string())?;
        println!("{calendar}");
    }

    Ok(())
}
