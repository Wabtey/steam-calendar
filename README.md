# Add your (steam) game sessions to your calendar

<!-- DOC: License it -->
<!-- DOC: Code coverage -->

## Quickstart

- create the `.env` file at the root of the project and include your [steam api key](https://steamcommunity.com/dev/apikey)
  - `cp .example.env .env`
  
  ```toml
  STEAM_API_KEY=XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
  STEAM_USER_ID=76561197960435530 # Robin Walker
  CALDAV_PROVIDER="https://myNextCloudserver.org/remote.php/dav/calendars/user"
  CALDAV_USERNAME=User
  CALDAV_PASSWORD="password"
  ```

- `cargo run` locally run your server and query steam api
- `ngrok http http://localhost:8080` to publish your local server using ngrok
- append `/calendar.ics` to your ngrok url and use it do have a dynamic (change with the server) read only calendar

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

<!-- DOC: add to the repo an example of .ics generated -->
