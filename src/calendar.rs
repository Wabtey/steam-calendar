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
) -> Event {
    /* -------------------------- retrieve achievements ------------------------- */
    let achievements = get_session_achievement(
        config.clone(),
        game_id,
        game_info,
        start_time.timestamp(),
        end_time.timestamp(),
    )
    .await;

    println!(
        "{}: Stopped playing at {}{} (for {})",
        Utc::now()
            .with_timezone(&config.timezone)
            .format("%H:%M:%S"),
        game_info,
        if achievements.is_empty() {
            String::new()
        } else {
            format!(", {} achievements unlocked", achievements.lines().count())
        },
        {
            let duration = end_time - start_time;
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() % 60;
            let seconds = duration.num_seconds() % 60;

            if hours > 0 {
                format!("{hours}h {minutes}m {seconds}s")
            } else if minutes > 0 {
                format!("{minutes}m {seconds}s")
            } else {
                format!("{seconds}s")
            }
        }
    );
    /* ---------------------------- retrieve calendar --------------------------- */
    let mut calendar = if Path::new(&config.calendar_path).exists() {
        let calendar_data = fs::read_to_string(config.calendar_path.clone())
            .unwrap_or("BEGIN:VCALENDAR\nEND:VCALENDAR".to_string());
        Calendar::from_str(&calendar_data).unwrap_or_default()
    } else {
        Calendar::new()
            .name("Game Sessions")
            // .version("2.0")
            .done()
    };

    /* ----------------------------- push new event ----------------------------- */
    // TODO: feat - add presence of steam friend
    let mut game_info_plus = game_info.to_string();
    let description = if achievements.is_empty() {
        "No achievements unlocked during this session".to_string()
    } else {
        game_info_plus = format!("{game_info} * ({})", achievements.lines().count());
        format!("Achievements:\n{achievements}")
    };

    let event = Event::new()
        .summary(&game_info_plus)
        .description(&description)
        .starts(start_time)
        // .class(Class::Confidential)
        .status(icalendar::EventStatus::Confirmed)
        .ends(end_time)
        .uid(&format!(
            "steam-{}-{}",
            game_info.to_lowercase().replace(' ', "-"),
            start_time.timestamp()
        ))
        .done();

    // println!("{event:#?}");

    // local save
    calendar.push(event.clone());

    // backup
    fs::write(
        backup_path(&config.calendar_path, &Utc::now().format("%Y%m%d_%H%M%S").to_string()).unwrap_or_else(|e| format!("Failed to set up a correct backup path: {e}")),
        calendar.to_string(),
    )
    .unwrap_or_else(|e| eprintln!("failed to write backup calendar: {e}"));

    fs::write(config.calendar_path, calendar.to_string())
        .unwrap_or_else(|e| eprintln!("failed to write calendar: {e}"));
    // println!("{calendar}");
    event
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
    let url = format!("{provider}/{uid}.ics");

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

/* ---------------------------------------------------------- */
/*                           Utils                            */
/* ---------------------------------------------------------- */

/// Returns the correct backup path.
///
/// The format, for a date `utc` = `"19700101_000101"` and
/// for a calendar_path including a parent directory as `"parent_dir/calendar.ics"`
/// it creates `"parent_dir/19700101_000101.calendar.ics.bak"`.
///
/// For a calendar_path with only a file stem as `"calendar.ics"` it creates
/// `"19700101_000101.calendar.ics.bak"`.
///
/// ## Notes
///
/// NOTE: this path doesn't work on Windows - replace / by \
fn backup_path(calendar_path: &str, utc: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(calendar_path);
    let parent = path.parent();
    let file_stem = path.file_stem();
    let ext = path.extension();

    match (parent, file_stem, ext) {
        (Some(parent), Some(file_stem), Some(ext)) => {
            let parent = parent.to_str().unwrap_or(".");
            let backup_path = if parent.is_empty() {
                format!("{utc}.{calendar_path}.bak")
            } else {
                format!(
                    "{}/{}.{}.{}.bak",
                    parent,
                    utc,
                    file_stem.to_str().unwrap_or("NO_FILE_STEM"),
                    ext.to_str().unwrap_or("UNKNOWN_EXTENSION"),
                )
            };

            Ok(backup_path)
        }

        (_, _, _) => Err(format!(
            "Failed to create a correct backup_path: calendar_path: {calendar_path}" // parent: {parent} file_stem: {file_stem} ext: {ext}
        )
        .into()),
    }
}

/* ---------------------------------------------------------- */
/*                           Tests                            */
/* ---------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_path_well_formed() {
        let calendar_short_path = "calendar.ics";
        let date = &DateTime::from_timestamp(61, 0).unwrap().format("%Y%m%d_%H%M%S").to_string();

        let backup_short_path = backup_path(calendar_short_path, date);
        assert!(backup_short_path.is_ok());
        assert_eq!(backup_short_path.unwrap(), "19700101_000101.calendar.ics.bak");

        let calendar_long_path = "parent_dir/calendar.ics";

        let backup_long_path = backup_path(calendar_long_path, date);
        assert!(backup_long_path.is_ok());
        assert_eq!(
            backup_long_path.unwrap(),
            "parent_dir/19700101_000101.calendar.ics.bak"
        );
    }
}
