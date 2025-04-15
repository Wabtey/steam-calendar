# Add your (steam) game sessions to your calendar

<!-- DOC: License it -->
<!-- DOC: Code coverage -->

## Quickstart

- create the `.env` file at the root of the project and include your [steam api key](https://steamcommunity.com/dev/apikey)
  - `cp .sos.env .env`
  
  ```toml
  STEAM_API_KEY=XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
  STEAM_USER_ID=76561197960435530 # Robin Walker
  ```

- `cargo run`

## Event Format

```ics
BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTAMP:20250415T002909Z
CLASS:CONFIDENTIAL
DESCRIPTION:Achievements:\n- 1% Finish the base game
DTEND:20250415T004909Z
DTSTART:20250415T002909Z
SUMMARY:Vampire Survivors
UID:steam-vampire-survivors-1744676949
END:VEVENT
END:VCALENDAR
```
