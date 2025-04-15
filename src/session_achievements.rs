use crate::api::i_steam_user_stats::{PlayerAchievementsResponse, PlayerStats};

/// Fetch achievements for the current game
pub async fn get_session_achievement(
    api_key: String,
    steamid: String,
    appid: String,
    game_name: String,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("{appid}");
    let achievements_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?appid={}&key={}&steamid={}",
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
        .map(|a| {
            format!(
                "- {} ({})",
                // REFACTOR: instead of using localized name/description fetch game info to complete
                a.name.as_ref().unwrap_or(&a.apiname),
                // a.description
                //     .as_ref()
                //     .unwrap_or(&"no description".to_string()),
                // FIXME: use custom timezone (summer hour, time late by an hour)
                chrono::DateTime::from_timestamp(a.unlocktime, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(achievements)
}
