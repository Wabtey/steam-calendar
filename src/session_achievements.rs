use chrono::DateTime;
use std::collections::HashMap;

use crate::{
    Config,
    api::i_steam_user_stats::{
        AchievementPercentages, GlobalAchievementPercentagesForApp, PlayerAchievementsResponse,
        PlayerStats,
    },
};

/// Fetch achievements for the current game
pub async fn get_session_achievement(
    config: Config,
    appid: &str,
    game_name: &str,
    start_time: i64,
    end_time: i64,
) -> String {
    // println!("game id: {appid}");
    let achievements_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?appid={}&key={}&steamid={}&l=english",
        appid, config.api_key, config.steam_id
    );

    let default_response = PlayerAchievementsResponse {
        playerstats: PlayerStats {
            steamID: config.steam_id.to_string(),
            gameName: game_name.to_string(),
            achievements: vec![],
        },
    };

    let achievements: PlayerAchievementsResponse = match reqwest::get(&achievements_url).await {
        Ok(achievements_response) => achievements_response.json().await.unwrap_or_else(|_| {
            eprintln!("JSON parsing error");
            default_response
        }),
        Err(_) => {
            eprintln!("No achievements available for {game_name}");
            default_response
        }
    };

    let filtered_achievements = achievements
        .playerstats
        .achievements
        .iter()
        .filter(|a| a.achieved == 1)
        .filter(|a| a.unlocktime >= start_time && a.unlocktime <= end_time);

    let mut percentages = HashMap::new();
    for a in filtered_achievements.clone() {
        let percentage = get_achievement_percentage(config.clone(), appid, &a.apiname).await;
        percentages.insert(&a.apiname, format!("{percentage}%"));
    }

    // calendar event description
    let achievements: String = filtered_achievements
        .map(|a| {
            format!(
                "- {}: {}{} ({})",
                a.name.as_ref().unwrap_or(&a.apiname),
                percentages.get(&a.apiname).unwrap(),
                {
                    let description = a
                        .description
                        .clone()
                        .unwrap_or(" - no description".to_string());
                    if description.is_empty() {
                        "no description".to_string()
                    } else {
                        description
                    }
                },
                DateTime::from_timestamp(a.unlocktime, 0)
                    .map(|dt| dt
                        .with_timezone(&config.timezone)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string())
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    achievements
}

pub async fn get_achievement_percentage(
    config: Config,
    gameid: &str,
    achievement_name: &str,
) -> String {
    let achievements_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}&key={}&l=english",
        gameid, config.api_key
    );

    let achievements_response = reqwest::get(&achievements_url).await;
    let percentage = match achievements_response {
        Err(e) => {
            eprintln!("{e}");
            "101.".to_string()
        }
        Ok(achievements_response) => {
            let achievement_percentages: GlobalAchievementPercentagesForApp =
                match achievements_response.json().await {
                    Ok(data) => data,
                    Err(_) => {
                        println!("No achievement stats available for {gameid}");
                        GlobalAchievementPercentagesForApp {
                            achievementpercentages: AchievementPercentages {
                                achievements: vec![],
                            },
                        }
                    }
                };

            achievement_percentages
                .achievementpercentages
                .achievements
                .iter()
                .find(|achievement| achievement.name.eq(achievement_name))
                .map(|achievement| achievement.percent.clone())
                .unwrap_or_else(|| {
                    eprintln!("{achievement_name} not found in {gameid}");
                    "101.".to_string()
                })
        }
    };

    percentage
}
