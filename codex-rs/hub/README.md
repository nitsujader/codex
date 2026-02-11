# codex-hub

`codex-hub` provides the experimental local daemon runtime used by companion clients.

Current responsibilities:
- Persist daemon state under `CODEX_HOME/hub/state.json`
- Expose loopback control endpoints:
  - `GET /health`
  - `POST /pair/start` (Bearer token required)
  - `POST /admin/shutdown` (Bearer token required)
- Provide CLI-facing lifecycle helpers used by `codex hub start|stop|status|pair`
