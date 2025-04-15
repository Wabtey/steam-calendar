use chrono::{Duration, Utc};
use dotenv::dotenv;
use icalendar::{Calendar, Class, Component, Event, EventLike};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerSummaryResponse {
    response: PlayerSummaries,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerSummaries {
    players: Vec<Player>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    steamid: String,
    personaname: String,
    profileurl: String,
    avatar: String,
    avatarmedium: String,
    avatarfull: String,
    personastate: i32,
    communityvisibilitystate: i32,
    profilestate: Option<i32>,
    lastlogoff: Option<i64>,
    commentpermission: Option<i32>,

    // Private data fields (`Node` if profile is private)
    realname: Option<String>,
    primaryclanid: Option<String>,
    timecreated: Option<i64>,
    gameid: Option<String>,
    gameserverip: Option<String>,
    // what we need
    gameextrainfo: Option<String>,
    loccountrycode: Option<String>,
    locstatecode: Option<String>,
    loccityid: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    /* ------------------------------- API request ------------------------------ */
    let api_key = env::var("STEAM_API_KEY")
        .expect("STEAM_API_KEY must be set in environment variables (`/.env`)");
    let user_id = env::var("STEAM_USER_ID")
        .expect("STEAM_USER_ID must be set in environment variables (`/.env`)");

    // TODO: feat - request every 5minutes
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={api_key}&steamids={user_id}",
    );

    let response = reqwest::get(&url).await?;
    let player_summary: PlayerSummaryResponse = response.json().await?;
    println!("{:?}", player_summary.response.players[0].gameextrainfo);

    if let Some(game_info) = &player_summary.response.players[0].gameextrainfo {
        // TODO: feat - fetch the achievements obtained
        /* ---------------------------- retrieve calendar --------------------------- */
        let calendar_path = "calendar.ics";
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
        // TODO: feat - ask for user credentials to connect to CalDAV

        /* ----------------------------- push new event ----------------------------- */
        let event = Event::new()
            .summary(game_info)
            .description("Achievements:\n- 1% Finish the base game")
            .starts(Utc::now())
            .class(Class::Confidential)
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
