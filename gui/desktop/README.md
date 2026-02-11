# Desktop Companion Scaffold

Windows-first desktop companion scaffold built with Tauri 2, React, TypeScript, and Vite-style layout.

## Run

```bash
cd gui/desktop
npm install
npm run dev
```

Run the desktop shell with Tauri:

```bash
npm run tauri:dev
```

## Build

```bash
npm run build
npm run tauri:build
```

## Scripts

- `npm run dev`: starts Vite dev server.
- `npm run build`: type-checks and builds frontend.
- `npm run lint`: lints TypeScript and TSX files.
- `npm run preview`: previews built frontend assets.
- `npm run tauri:dev`: starts Tauri app in development mode.
- `npm run tauri:build`: creates desktop build artifacts.

## Architecture Notes

- `src/` contains the React app with a pane-based companion UI:
  - `ThreadsPane`
  - `TimelinePane`
  - `AgentBoardPane`
  - `PromptComposerPane`
  - `StatusRailPane`
- Theme presets are CSS-variable driven and switched through `data-theme` on the document root (`default`, `fallout`, `cyberpunk`, `matrix`).
- Visual indicators are reusable components:
  - `AgentStatusBadge`
  - `NetworkIndicator`
  - `CompactionIndicator`
- `src/lib/hubClient.ts` targets local hub APIs at `http://127.0.0.1:46710` with methods for `/status` and `/pairing`.
- `src-tauri/` includes the Rust shell and placeholder commands for clipboard paste, screenshot, and notifications.
