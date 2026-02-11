# Codex Native GUI Plan

## Objective
Build a native desktop Codex GUI that is dramatically better than current agent UIs in:
- Speed and responsiveness
- Multi-agent orchestration
- Context transparency (compaction, token budgets, pinned prompt)
- Workspace intelligence (project memory, repo-aware tools)
- Visual customization and workflow ergonomics
- Companion mobile visibility and control on local network

Scope includes:
- Windows-first desktop execution quality with cross-platform viability.
- Android companion app over local network for real-time monitoring and interaction.
- Explicit coexistence with CLI/TUI (companions, not replacements).

## Inputs And References
- Existing `codex-rs` engine and TUI (already has export, project memory, screenshot, themes, notifications, transcript stream, subagent controls).
- `anomalyco/opencode` observations (dev branch):
  - Desktop app uses `Tauri v2` + Rust `src-tauri` + web frontend package.
  - Ships sidecar CLI binary and deep-link/update/notification plugins.
  - Explicit client/server architecture direction.
- `JetBrains/intellij-community` observations:
  - Platform-first architecture, plugin extensibility, long-lived action system.
  - Windows build/runtime discipline and predictable tooling conventions.

## Execution Tooling Policy
- This plan is implementation-tool agnostic and does not assume Claude usage.
- Subagents and Codex CLI are encouraged where they reduce risk or speed up delivery.

## Implementation Status
- In progress.
- Landed foundation milestone:
  - New `codex-hub` crate with local daemon lifecycle and control endpoints.
  - New CLI surface: `codex hub start|stop|status|pair`.

---

## 1) GUI Stack Decision Matrix

Scoring: 1 (poor) to 5 (excellent). Weighted for this repo.

### Weights
- Windows UX quality and packaging: 20%
- Reuse of existing Rust core: 20%
- UI velocity and hiring pool: 15%
- Runtime perf/memory footprint: 15%
- IPC/tooling maturity: 15%
- Extensibility/theming ecosystem: 10%
- Long-term lock-in risk: 5%

### Matrix
| Stack | Win UX | Rust Reuse | Velocity | Perf | IPC/Tooling | Extensibility | Lock-in | Weighted Score |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| **Tauri + React/TypeScript** | 4 | 5 | 5 | 4 | 5 | 5 | 4 | **4.6** |
| Electron + React | 4 | 3 | 5 | 2 | 5 | 5 | 3 | 3.8 |
| Pure Rust `egui` | 3 | 5 | 3 | 5 | 4 | 3 | 5 | 4.0 |
| GPUI (Rust) | 2 | 4 | 2 | 5 | 2 | 2 | 3 | 2.9 |
| Flutter Desktop | 4 | 2 | 4 | 4 | 3 | 4 | 3 | 3.5 |
| Qt (Rust bindings / C++) | 4 | 3 | 2 | 4 | 3 | 4 | 2 | 3.2 |

### Recommendation
Choose **Tauri v2 + React/TypeScript + Rust backend bridge**.

Why:
- Best blend of velocity and control for a feature-dense app.
- Preserves Rust core as product differentiator.
- Strong Windows installer/update/deeplink support.
- Faster UX iteration than pure Rust GUI frameworks for complex panels and timeline views.

Deliberate non-choice:
- Do **not** start with Electron due to memory/process overhead and weaker “native-feeling” baseline.
- Do **not** bet phase-1 delivery on GPUI maturity.

## Mobile Companion Stack Decision (Android)

### Options
| Stack | Android Velocity | Runtime Perf | Native Integration (notifications/background/network) | Reuse With Desktop UI | Weighted Fit |
|---|---:|---:|---:|---:|---:|
| **Kotlin + Jetpack Compose** | 4 | 5 | 5 | 2 | **4.3** |
| Flutter | 4 | 4 | 4 | 3 | 3.9 |
| React Native | 4 | 3 | 3 | 4 | 3.7 |

### Recommendation
Choose **Kotlin + Jetpack Compose** for Android companion v1.

Why:
- Best native behavior for LAN service discovery, foreground/background behavior, notifications, and battery discipline.
- Strong reliability for long-lived realtime streams and OS-level integration.
- Fastest path to a polished Android companion without desktop compromises.

---

## 2) Target Architecture

## Layered Architecture
1. **Core Engine (`codex-rs/core`, existing)**
- Session/thread state
- Tool execution and policy
- Compaction and context management
- Model/provider adapters

2. **App Orchestration Layer (`codex-rs/gui-host`, new Rust crate)**
- Stable API boundary for desktop clients
- Event fanout for high-frequency streaming
- Attachment pipeline (clipboard image, screenshot, local files)
- Persistence hooks (project memory, transcript stream, exports)
- Notification routing abstraction

3. **GUI Shell (`codex-gui`, Tauri app)**
- Window/workspace management
- React state/store and virtualized timelines
- Theme/layout system
- Rich interactions (drag/drop, multi-pane, keyboard command palette)

4. **Companion Gateway (`codex-rs/companion-gateway`, new Rust crate)**
- Local-network API endpoint for companion clients (Android first)
- Event multiplexing from core threads/agents to subscribed clients
- Pairing/auth/session management and client ACL enforcement
- LAN discovery support (mDNS) and secure channel bootstrap

5. **Companion Clients**
- Android app (Kotlin + Compose) for realtime watch + interact
- Future iOS/Web companions via same API contract

## API Strategy
Use **app-server v2 as canonical contract** and add GUI-specific v2 methods/notifications.
- Avoid bespoke ad-hoc Tauri command sprawl.
- Keep transport abstractable:
  - Local IPC for desktop shell
  - Local-network WebSocket/HTTP for companion clients
  - Optional remote/web later without contract breakage
- All clients consume the same event schema and operation semantics.
- Use explicit capability negotiation (`client/hello`) so desktop and mobile can evolve independently.

## Local Network Companion Model
- Default mode: loopback-only (`127.0.0.1`) and no LAN exposure.
- Companion mode (explicitly enabled):
  - Gateway listens on LAN interface and advertises via mDNS.
  - Pairing flow mints a client-scoped credential and stores it in OS keychain/secure storage.
  - All realtime traffic uses authenticated, encrypted channels.
- Companion access roles:
  - `observer` (read-only events/threads/agents)
  - `operator` (can submit prompts/control agents)
  - Role chosen at pair time and editable from desktop settings.

## Coexistence Model (CLI/TUI/GUI/Android)
- Core remains single source of truth for thread and agent state.
- CLI, TUI, Windows GUI, and Android are all peer clients connected via shared contract.
- Any client can observe realtime updates; write operations are policy-gated but semantically identical.
- No client-specific business logic forks for compaction, memory, or agent lifecycle.
- When multiple clients issue writes concurrently, server-side sequencing applies deterministic ordering by operation ID and timestamp.
- Clients render optimistic UI only for local affordance; authoritative state always comes from stream acknowledgements.

## Data Flow
- User action -> client command (CLI/TUI/GUI/Android) -> orchestration/gateway -> Core op -> event stream
- Event stream -> fanout bus -> normalized client state model -> incremental UI updates
- Persistent artifacts (exports, memory, transcript, snapshots) stored under `CODEX_HOME` with clear ownership.
- Mobile reconnect flow:
  - Android client sends last acknowledged event cursor.
  - Gateway replays missed events before switching to live stream.
  - Cursor checkpoints persisted per paired device.

---

## 3) Phased Roadmap (0/1/2/3)

## Phase 0: Architecture + Risk Burn-down (2-3 weeks)
### Deliverables
- Final stack ADR (Tauri+React) and package layout.
- API contract draft for GUI methods/events in app-server v2.
- Skeleton app window with live event stream from one thread.
- Performance baseline harness (cold start, stream fps, memory idle/active).
- Companion gateway spike with LAN discovery + pairing handshake prototype.

### Acceptance Criteria
- Launch to interactive window < 2.5s on reference Win dev machine.
- Stream 500+ timeline events without dropped ordering.
- Contract review approved (core + app-server + GUI owners).
- Android test harness can connect, authenticate, and receive live thread events on LAN.

### Risks
- Event protocol mismatch with existing TUI semantics.
- IPC throughput bottlenecks for heavy streaming.
- Pairing/security complexity delaying UI work.

### De-risking Spikes
- Event batching/coalescing prototype.
- Thread replay snapshot + incremental delta test harness.
- Certificate bootstrap + QR pairing experiment.

## Phase 1: Foundation GUI + Android Companion Alpha (4-6 weeks)
### Deliverables
- Main shell: chat pane, timeline pane, composer, command palette.
- Session/thread switching and subagent list basics.
- Model/personality/collab controls.
- Export to markdown + live transcript toggles surfaced in GUI.
- Windows notifications integration and settings.
- Android alpha app:
  - Device pairing and trusted-device list.
  - Realtime thread/agent observer views.
  - Prompt composer for existing primary thread.

### Acceptance Criteria
- Complete end-to-end turn execution parity with current TUI for core flow.
- No blocking UI jank > 100ms during normal streaming.
- Crash-free rate > 99.5% on internal dogfood cohort.
- Two concurrent clients (desktop + Android) can observe and interact with same thread without state drift.

### Risks
- Frontend store complexity growth.
- Inconsistent behavior between GUI and TUI pathways.
- Mobile lifecycle interruptions (backgrounding/network changes) causing stale cursors.

### De-risking Spikes
- Shared adapter library for event normalization.
- Golden trace replay tests (same trace rendered by GUI state reducers).
- Mobile reconnect simulator with forced network interruptions.

## Phase 2: “Epic” Feature Set (6-8 weeks)
### Deliverables
- Multi-agent control center (spawn/focus/interrupt/close + health).
- Conversation timeline with compaction visualization and rewind points.
- Context budget meter with pinned prompt inspector.
- Workspace memory panel + inline editing + provenance.
- Screenshot/image paste studio (preview/crop/annotate/attach queue).
- Theme studio (preset + custom token editing + layout presets).
- Android command deck:
  - Agent list with live status and quick controls.
  - Push alerts for completion/failure/mention.
  - “Follow selected agent” stream mode.

### Acceptance Criteria
- Users can understand “why context changed” without reading raw logs.
- Multi-agent task orchestration reduces completion time on benchmark tasks by >=20%.
- At least 3 production-ready themes + persistent layout profiles.
- Mobile actions (`prompt send`, `agent interrupt`) have >=99% delivery success on stable LAN.

### Risks
- UX complexity could overwhelm first-time users.
- Compaction introspection may expose too much low-level noise.
- Notification overload across desktop/mobile channels.

### De-risking Spikes
- Progressive disclosure UX prototype with novice/power modes.
- Instrumented usability sessions on compaction/timeline clarity.
- Notification policy presets (`minimal`, `balanced`, `verbose`) with default tuned to `balanced`.

## Phase 3: Platform And Extension Moat (6-10 weeks)
### Deliverables
- Plugin/extension API (read-only v1 + controlled command actions).
- Workflow automations (macros, triggers, session recipes).
- Team features: shareable workspace profiles, export bundles, policy presets.
- Performance hardening and large-repo stress profile.
- Cross-client continuity improvements (desktop handoff to mobile and back).

### Acceptance Criteria
- Third-party extension can add panel + commands without fork.
- 100k event timeline sessions remain interactable.
- SLOs met across Windows/macOS/Linux release channels.
- Companion reconnect after sleep/resume recovers to live state in < 5s P95.

---

## 4) Killer GUI Feature Spec

## A) Multi-Agent Control Center
- Agent grid/list with status, current operation, token burn rate, and queue depth.
- Action bar: `Spawn`, `Route task`, `Interrupt`, `Pause`, `Close`, `Promote to primary`.
- Visual dependency graph for delegated tasks.
- Fast “follow mode” to pin viewport to selected agent output.

## B) Conversation Timeline + Compaction Lens
- Unified timeline of user/assistant/tool/compaction/system events.
- Compaction events rendered as explicit checkpoints with:
  - Before/after context size
  - Summary payload
  - Pinned original prompt verification
  - “Recent reviewed” excerpts
- Time travel read mode: inspect prior context snapshots without mutating current state.

## C) Prompt Pinning + Context Budget Meter
- Fixed panel showing pinned prompt text and mutation history.
- Live budget ring:
  - Total window, used, reserved, projected post-turn usage.
  - Risk indicator (“safe”, “tight”, “compaction likely”).
- Click-through to top contributors by token footprint (files/messages/tools).

## D) Rich Reasoning/Thinking Surfaces
- Streamed “thinking note” rail with dedup-aware variety and per-theme flavor.
- Expandable reasoning summaries and confidence markers.
- Optional “strict mode” hiding speculative/low-value updates.

## E) Workspace Memory Panel
- Per-project memory with sections (Goals, Constraints, Preferences, Gotchas).
- Edit with version history and compaction-assisted refresh suggestions.
- “Applied in this turn” indicator to make memory usage explicit.

## F) Screenshot + Image Paste Workflow
- Clipboard ingest queue with thumbnails and metadata.
- Capture tools: full screen, region, window (Windows-first).
- Lightweight annotate/crop before attach.
- Attachment manifest visible in composer.

## G) Notification Center
- In-app notification inbox + OS notifications.
- Rule engine: notify by event type, severity, agent, or mention.
- Retry/failure diagnostics surfaced with quick actions.

## H) Theme + Layout Studio
- Presets: Default, Fallout, Cyberpunk, Matrix + user-defined themes.
- Theme tokens editable (colors, typography, spacing, animation density).
- Dockable panes and saved workspace layouts.
- “Streamer mode” and “focus mode” profiles.

## I) Android Companion Command Deck
- Realtime cards for active threads and agents.
- One-tap actions: `Send prompt`, `Interrupt`, `Pause`, `Resume`.
- Readable mobile timeline with compaction checkpoints and pinned prompt badge.
- Local-only status banner showing connection, role, and security state.

---

## 5) Technical Implementation Details

## IPC/API Contracts (v2-first)
Add/extend v2 methods (illustrative):
- `client/hello` (capability and version negotiation)
- `thread/exportMarkdown`
- `thread/streamTranscriptSet`
- `thread/contextBudgetRead`
- `thread/compactionHistoryRead`
- `thread/pinnedPromptRead`
- `memory/readProject`
- `memory/writeProject`
- `agent/list`
- `agent/spawn`
- `agent/control` (interrupt/pause/close)
- `media/captureScreenshot`
- `ui/themeSet`
- `ui/layoutSet`
- `companion/modeSet` (loopback-only vs LAN)
- `companion/pairInit`
- `companion/pairConfirm`
- `companion/pairList`
- `companion/pairRevoke`
- `companion/sessionList`
- `companion/sessionAttach`
- `companion/roleSet`

Notifications:
- `thread/eventNotification`
- `thread/compactionNotification`
- `thread/tokenUsageNotification`
- `agent/statusNotification`
- `media/captureNotification`
- `ui/notificationRaised`
- `companion/deviceStatusNotification`
- `companion/pairingNotification`
- `companion/connectionHealthNotification`

## Companion Transport + Security
- Transport:
  - Desktop IPC: existing local app-server channel.
  - Mobile: WebSocket stream + HTTP command channel on same local gateway.
- Authentication:
  - Explicit pairing only; no anonymous LAN access.
  - Short-lived pairing challenge plus long-lived device credential.
- Encryption:
  - TLS required for LAN mode; desktop-generated cert pinned by companion app at pair time.
- Authorization:
  - Role-based permissions (`observer`, `operator`) enforced server-side on each command.
- Safety defaults:
  - LAN mode disabled by default.
  - Optional “prompt write disabled from mobile” policy for safety-sensitive users.

## State Model
- Event-sourced client store with normalized entities:
  - `threads`, `turns`, `events`, `agents`, `attachments`, `memories`, `ui`, `clients`.
- Deterministic reducers for replay and regression testing.
- Coalescing strategy for high-frequency deltas (message/reasoning/tool output).
- Per-client cursor tracking for replay-on-reconnect correctness.

## Plugin/Extension Model (v1)
- Manifest-declared capabilities and permissions.
- Sandboxed extension runtime (JS workers) with explicit API surface.
- Allowed operations:
  - Read timeline slices
  - Register custom panels/commands
  - Emit non-privileged suggestions/actions
- Privileged tool execution remains core-controlled.

## Telemetry + Perf Budgets
### Product KPIs
- Task completion time
- Multi-agent throughput
- User interaction friction (cancel/backtrack rates)
- Feature activation and retention
- Cross-client interaction success rate (desktop to mobile and mobile to desktop)
- Companion command latency and reconnect reliability

### Performance Budgets
- Cold start: <= 2.5s (P50), <= 4.0s (P95)
- Input-to-paint: <= 50ms P95
- Stream render throughput: >= 120 events/sec sustained without visible stutter
- Memory: <= 700MB steady-state typical session
- Companion end-to-end event lag: <= 300ms P95 on same Wi-Fi LAN
- Companion reconnect catch-up: <= 5s P95 for <= 5k missed events

## Testing Strategy
- Unit: reducers, selectors, IPC marshalling, theme token logic.
- Integration: GUI-host + core event contract tests.
- E2E: Playwright desktop smoke paths (new session, agent spawn, compaction inspect, export).
- Snapshot/visual: timeline cards, compaction blocks, theme variants.
- Replay tests: golden event traces from real sessions to ensure deterministic UI state.
- Android instrumentation tests for pairing, stream subscribe, prompt send, and reconnect.
- Network chaos tests (packet loss, connection flap, AP switch) for companion reliability.

---

## 6) Migration Strategy (TUI -> Shared Engine + GUI)

## Principles
- No forked business logic.
- TUI and GUI are clients over shared orchestration and contract.
- Move logic downward into reusable crates before adding GUI-only hacks.
- CLI remains first-class and fully operable with no GUI dependency.
- Mobile companion is additive control/visibility, never a required path.

## Migration Steps
1. Extract shared “client-facing” adapters from TUI into reusable Rust modules.
2. Promote GUI-relevant behavior into app-server v2 contract.
3. Add companion gateway behind feature flag with loopback-only default.
4. Keep TUI as reference client and regression oracle during GUI build-out.
5. Add trace replay parity tests across CLI/TUI/GUI/mobile reducers for core workflows.
6. Gradually shift advanced UX features to GUI while preserving CLI/TUI reliability.

## Duplication Guardrails
- One source of truth for:
  - Compaction semantics
  - Project memory pathing and update rules
  - Transcript export and redaction options
  - Agent lifecycle semantics
  - Event IDs, ordering, and replay cursor semantics

---

## 7) One-Page Epic For Kickoff

## Epic Name
**Codex Command Center (Native GUI)**

## Problem
Current agent interfaces limit power-user velocity and transparency in multi-agent, long-context, and highly visual workflows. We need a desktop experience with a companion mobile surface so users can monitor and steer work from anywhere on their local network.

## Outcome
A Windows-first, cross-platform native GUI plus Android companion that:
- Executes and visualizes complex multi-agent workflows.
- Makes context state (token budget, compaction, pinned prompt) transparent.
- Integrates media and project memory workflows natively.
- Delivers customizable, high-performance UX with extension hooks.
- Syncs realtime state over LAN with secure pairing and policy-controlled interaction.

## Non-Goals (v1)
- Full cloud collaboration suite.
- Arbitrary untrusted code execution inside extensions.
- Replacing CLI/TUI; they remain supported first-class clients.
- WAN/internet remote control (local network only for v1).

## Users
- Primary: power users, staff engineers, infra/reliability engineers.
- Secondary: advanced individual developers, AI workflow creators.

## Success Metrics
- >=20% faster completion time on benchmark multi-step coding tasks.
- >=30% reduction in context-related user confusion events.
- >=50% weekly active usage of at least one advanced GUI feature (timeline lens, agent center, theme/layout customization).

## Milestones
- M0 (2-3 weeks): architecture + perf baseline + skeleton stream + pairing spike.
- M1 (4-6 weeks): desktop parity foundation + Android observer alpha.
- M2 (6-8 weeks): killer feature set + Android interactive command deck in dogfood.
- M3 (6-10 weeks): extension model, scale hardening, and cross-client continuity polish.

## Team Shape (initial)
- 1 Tech Lead (Rust/core + contract)
- 2 Desktop/UI engineers (Tauri + React)
- 1 Backend/orchestration engineer
- 1 Android engineer (Kotlin + Compose + networking/notifications)
- 1 Design engineer (UX systems + theming)
- 1 QA/SDET focusing on replay + e2e automation

## Top Risks
- Scope explosion from feature-rich UI surface.
- Contract instability between GUI and core.
- Performance regressions in large timelines.
- LAN companion security and trust model mistakes.

## Mitigations
- Contract-first development and replay-based regression suite.
- Aggressive phase gates and perf SLO enforcement.
- Progressive disclosure UX to keep complexity manageable.
- Default-deny LAN mode with explicit pairing, cert pinning, and revocation tooling.

---

## Immediate Next Actions (This Week)
1. Create ADR selecting Tauri+React and define package layout.
2. Draft app-server v2 contract PR including companion pairing/session methods.
3. Build Phase-0 skeleton window with live thread event stream.
4. Build Android proof-of-connectivity app (pair + subscribe + render events).
5. Capture baseline perf metrics on Windows reference machines and LAN event lag.
6. Define dogfood cohort and benchmark task suite (desktop-only and cross-device flows).
