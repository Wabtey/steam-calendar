use std::{fs, path::Path, str::FromStr};

use chrono::{DateTime, Duration, Utc};
use icalendar::{Calendar, Component, Event, EventLike};

use crate::{Config, session_achievements::get_session_achievement};

/// Append to the calendar the game session event.
pub async fn create_game_session_event(
    config: Config<'_>,
    game_id: &str,
    game_info: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<Calendar, Box<dyn std::error::Error>> {
    /* -------------------------- retrieve achievements ------------------------- */
    let achievements = get_session_achievement(
        config,
        game_id,
        game_info,
        start_time.timestamp(),
        end_time.timestamp(),
    )
    .await?;

    println!(
        "{}: Stopped playing at {}{} (for {})",
        Utc::now().with_timezone(config.timezone).format("%H:%M:%S"),
        game_info,
        if achievements.is_empty() {
            "".to_string()
        } else {
            format!(", {} achievements unlocked", achievements.len())
        },
        {
            let duration = end_time - start_time;
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() % 60; // NOTE: %60 needed?
            let seconds = duration.num_seconds() % 60;

            if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
            }
        }
    );
    /* ---------------------------- retrieve calendar --------------------------- */
    let mut calendar = if Path::new(config.calendar_path).exists() {
        let calendar_data = fs::read_to_string(config.calendar_path)?;
        Calendar::from_str(&calendar_data)?
    } else {
        Calendar::new()
            .name("Game Sessions")
            // .version("2.0")
            .done()
    };
    // TODO: feat - send event to provider
    // TODO: feat - ask one time for user credentials to connect to CalDAV

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

    fs::write(config.calendar_path, calendar.to_string())?;
    // println!("{calendar}");
    Ok(calendar)
}
