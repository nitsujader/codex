# Android Companion Scaffold

This folder contains a standalone Android Studio project scaffold for a Codex companion app.

## What is included

- Kotlin + Jetpack Compose app skeleton
- Screens:
  - Pairing screen (host/token/code)
  - Session list screen
  - Live thread placeholder (prompt send + interrupt/pause/resume buttons)
  - Settings screen (connection state + role)
- Networking skeleton:
  - Ktor WebSocket client with reconnect loop placeholder logic
- Persistence skeleton:
  - Room database + entities/DAOs for paired devices and session cursors
- Discovery abstraction:
  - mDNS interface + stub implementation with TODO markers

## Local network assumptions

- Companion and host are on the same LAN or reachable private network.
- Host can expose a WebSocket endpoint, default expected path is `/ws` when only `host:port` is entered.
- Cleartext networking is currently allowed for local development.
- mDNS discovery is a stub right now and does not perform real discovery yet.

## Open in Android Studio

1. Open Android Studio.
2. Select `Open` and choose the `gui/android` directory.
3. Let Gradle sync complete.

## Run from terminal

From `gui/android`:

```bash
./gradlew assembleDebug
./gradlew installDebug
```

Windows PowerShell:

```powershell
.\gradlew.bat assembleDebug
.\gradlew.bat installDebug
```

## Next implementation steps

- Replace WebSocket placeholder payload formatting with typed protocol models.
- Implement actual mDNS discovery via Android NSD APIs.
- Add repository-backed session/thread loading from host APIs.
- Add tests for view model and database migration strategy.
