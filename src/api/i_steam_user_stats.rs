use serde::{Deserialize, Serialize};

/* ----------------- GetPlayerAchievements  ----------------- */

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlayerAchievementsResponse {
    pub playerstats: PlayerStats,
}

#[allow(non_snake_case)]
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlayerStats {
    pub steamID: String,
    pub gameName: String,
    pub achievements: Vec<Achievement>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Achievement {
    pub apiname: String,
    pub achieved: i32,
    pub unlocktime: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}

/* --------- GetGlobalAchievementPercentagesForApp  --------- */

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalAchievementPercentagesForApp {
    pub achievementpercentages: AchievementPercentages,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AchievementPercentages {
    pub achievements: Vec<AchievementPercentage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AchievementPercentage {
    pub name: String,
    pub percent: String,
}
