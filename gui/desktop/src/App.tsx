import { useEffect, useMemo, useState } from "react";
import { AGENTS, THREADS, TIMELINE_EVENTS } from "./data/mockData";
import { HUB_BASE_URL, fetchHubStatus, pairWithHub } from "./lib/hubClient";
import AgentBoardPane from "./panes/AgentBoardPane";
import PromptComposerPane from "./panes/PromptComposerPane";
import StatusRailPane from "./panes/StatusRailPane";
import ThreadsPane from "./panes/ThreadsPane";
import TimelinePane from "./panes/TimelinePane";
import { DEFAULT_THEME, type ThemePreset } from "./theme/presets";
import type { HubStatus } from "./types";
import "./app.css";

const THEME_KEY = "desktop-companion-theme";

const FALLBACK_HUB_STATUS: HubStatus = {
  reachable: false,
  service: "offline",
  paired: false,
  activeAgents: 0,
};

function loadSavedTheme(): ThemePreset {
  const fromStorage = localStorage.getItem(THEME_KEY);
  if (
    fromStorage === "default" ||
    fromStorage === "fallout" ||
    fromStorage === "cyberpunk" ||
    fromStorage === "matrix"
  ) {
    return fromStorage;
  }
  return DEFAULT_THEME;
}

export default function App() {
  const [selectedThreadId, setSelectedThreadId] = useState<string>(THREADS[0].id);
  const [promptValue, setPromptValue] = useState<string>("");
  const [theme, setTheme] = useState<ThemePreset>(loadSavedTheme);
  const [hubStatus, setHubStatus] = useState<HubStatus>(FALLBACK_HUB_STATUS);
  const [pairingCode, setPairingCode] = useState<string>("");
  const [pairingFeedback, setPairingFeedback] = useState<string>("Ready");

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem(THEME_KEY, theme);
  }, [theme]);

  useEffect(() => {
    let cancelled = false;

    const refreshStatus = async () => {
      const started = performance.now();
      try {
        const next = await fetchHubStatus();
        const latencyMs = Math.round(performance.now() - started);
        if (!cancelled) {
          setHubStatus({ ...next, latencyMs });
        }
      } catch {
        if (!cancelled) {
          setHubStatus(FALLBACK_HUB_STATUS);
        }
      }
    };

    void refreshStatus();
    const interval = window.setInterval(() => {
      void refreshStatus();
    }, 12000);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  const statusModel = useMemo(
    () => ({
      endpoint: HUB_BASE_URL,
      selectedTheme: theme,
      hubStatus,
      pairingFeedback,
    }),
    [hubStatus, pairingFeedback, theme],
  );

  const handlePair = async () => {
    if (!pairingCode.trim()) {
      setPairingFeedback("Enter a pairing code first.");
      return;
    }

    try {
      const response = await pairWithHub({
        pairingCode: pairingCode.trim(),
        deviceName: "desktop-companion",
      });
      setPairingFeedback(response.message);
      setPairingCode("");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Pairing failed.";
      setPairingFeedback(message);
    }
  };

  return (
    <div className="app-shell">
      <header className="top-bar">
        <h1>Desktop Companion</h1>
        <span className="top-subtitle">Thread-aware local control surface</span>
      </header>

      <main className="app-grid">
        <ThreadsPane
          threads={THREADS}
          selectedThreadId={selectedThreadId}
          onSelectThread={setSelectedThreadId}
        />
        <TimelinePane events={TIMELINE_EVENTS} />
        <AgentBoardPane agents={AGENTS} />
        <PromptComposerPane
          value={promptValue}
          onValueChange={setPromptValue}
          onSubmit={() => setPromptValue("")}
        />
        <StatusRailPane
          model={statusModel}
          pairingCode={pairingCode}
          onThemeChange={setTheme}
          onPairingCodeChange={setPairingCode}
          onPair={handlePair}
        />
      </main>
    </div>
  );
}
