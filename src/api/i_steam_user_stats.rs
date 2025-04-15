use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerAchievementsResponse {
    pub playerstats: PlayerStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerStats {
    pub steamid: String,
    pub gamename: String,
    pub achievements: Vec<Achievement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Achievement {
    pub apiname: String,
    pub achieved: i32,
    pub unlocktime: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}
