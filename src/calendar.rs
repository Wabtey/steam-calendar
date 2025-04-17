use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event, EventLike};
use reqwest::{Client, header};
use std::{error::Error, fs, path::Path, str::FromStr};

use crate::{Config, session_achievements::get_session_achievement};

/// Append to the calendar the game session event.
pub async fn create_game_session_event(
    config: Config,
    game_id: &str,
    game_info: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<Event, Box<dyn Error>> {
    /* -------------------------- retrieve achievements ------------------------- */
    let achievements = get_session_achievement(
        config.clone(),
        game_id,
        game_info,
        start_time.timestamp(),
        end_time.timestamp(),
    )
    .await?;

    println!(
        "{}: Stopped playing at {}{} (for {})",
        Utc::now()
            .with_timezone(&config.timezone)
            .format("%H:%M:%S"),
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
    let mut calendar = if Path::new(&config.calendar_path).exists() {
        let calendar_data = fs::read_to_string(config.calendar_path.clone())?;
        Calendar::from_str(&calendar_data)?
    } else {
        Calendar::new()
            .name("Game Sessions")
            // .version("2.0")
            .done()
    };

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
        .starts(start_time)
        // .class(Class::Confidential)
        .status(icalendar::EventStatus::Confirmed)
        .ends(end_time)
        .uid(&format!(
            "steam-{}-{}",
            game_info.to_lowercase().replace(" ", "-"),
            start_time.timestamp()
        ))
        .done();

    // println!("{event:#?}");
    // local save
    calendar.push(event.clone());

    fs::write(config.calendar_path, calendar.to_string())?;
    // println!("{calendar}");
    Ok(event)
}

/* ---------------------------------------------------------- */
/*                           CalDAV                           */
/* ---------------------------------------------------------- */

#[allow(unused)]
pub async fn publish_to_nextcloud(
    event: &Event,
    provider: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn Error>> {
    let uid = event.get_uid().ok_or("Event has no UID")?;
    let ical_data = event.to_string();
    let url = format!("{}/{}.ics", provider, uid);

    let client = Client::new();
    let response = client
        .put(&url)
        .basic_auth(username, Some(password))
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .body(ical_data)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to upload event: {} {}",
            response.status(),
            response.text().await?
        )
        .into())
    }
}
