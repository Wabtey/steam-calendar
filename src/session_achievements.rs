use crate::api::i_steam_user_stats::{PlayerAchievementsResponse, PlayerStats};
use chrono_tz::Tz;

/// Fetch achievements for the current game
pub async fn get_session_achievement(
    api_key: String,
    steamid: String,
    appid: String,
    game_name: String,
    start_time: i64,
    end_time: i64,
    timezone: &Tz,
) -> Result<String, Box<dyn std::error::Error>> {
    // println!("game id: {appid}");
    let achievements_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?appid={}&key={}&steamid={}&l=english",
        appid, api_key, steamid
    );

    let achievements_response = reqwest::get(&achievements_url).await?;
    let achievements: PlayerAchievementsResponse = match achievements_response.json().await {
        Ok(data) => data,
        Err(_) => {
            println!("No achievements available for this game");
            PlayerAchievementsResponse {
                playerstats: PlayerStats {
                    steamID: steamid.clone(),
                    gameName: game_name.clone(),
                    achievements: vec![],
                },
            }
        }
    };

    // calendar event description
    let achievements: String = achievements
        .playerstats
        .achievements
        .iter()
        .filter(|a| a.achieved == 1)
        // TODO: filter only those between the game time
        .filter(|a| a.unlocktime >= start_time && a.unlocktime <= end_time)
        .map(|a| {
            format!(
                "- {}: {} ({})",
                a.name.as_ref().unwrap_or(&a.apiname),
                a.description
                    .as_ref()
                    .unwrap_or(&"no description".to_string()),
                chrono::DateTime::from_timestamp(a.unlocktime, 0)
                    .map(|dt| dt
                        .with_timezone(timezone)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string())
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(achievements)
}
