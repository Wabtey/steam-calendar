use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerSummaryResponse {
    pub response: PlayerSummaries,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerSummaries {
    pub players: Vec<Player>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub steamid: String,
    pub personaname: String,
    pub profileurl: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub personastate: i32,
    pub communityvisibilitystate: i32,
    pub profilestate: Option<i32>,
    pub lastlogoff: Option<i64>,
    pub commentpermission: Option<i32>,

    // Private data fields (`Node` if profile is private)
    pub realname: Option<String>,
    pub primaryclanid: Option<String>,
    pub timecreated: Option<i64>,
    pub gameid: Option<String>,
    pub gameserverip: Option<String>,
    /// what we need
    pub gameextrainfo: Option<String>,
    pub loccountrycode: Option<String>,
    pub locstatecode: Option<String>,
    pub loccityid: Option<i32>,
}
