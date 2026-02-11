import NetworkIndicator from "../components/NetworkIndicator";
import { THEME_PRESETS, type ThemePreset } from "../theme/presets";
import type { StatusRailModel } from "../types";

interface StatusRailPaneProps {
  model: StatusRailModel;
  pairingCode: string;
  onThemeChange: (theme: ThemePreset) => void;
  onPairingCodeChange: (code: string) => void;
  onPair: () => void;
}

export default function StatusRailPane({
  model,
  pairingCode,
  onThemeChange,
  onPairingCodeChange,
  onPair,
}: StatusRailPaneProps) {
  return (
    <aside className="pane status-pane">
      <h2>Status Rail</h2>
      <NetworkIndicator
        connected={model.hubStatus.reachable}
        endpoint={model.endpoint}
        latencyMs={model.hubStatus.latencyMs}
      />

      <div className="status-row">
        <span className="status-label">Service</span>
        <span>{model.hubStatus.service}</span>
      </div>
      <div className="status-row">
        <span className="status-label">Paired</span>
        <span>{model.hubStatus.paired ? "Yes" : "No"}</span>
      </div>
      <div className="status-row">
        <span className="status-label">Active Agents</span>
        <span>{model.hubStatus.activeAgents}</span>
      </div>

      <label className="field-label" htmlFor="theme-select">
        Theme preset
      </label>
      <select
        id="theme-select"
        value={model.selectedTheme}
        onChange={(event) => onThemeChange(event.target.value as ThemePreset)}
      >
        {THEME_PRESETS.map((preset) => (
          <option key={preset} value={preset}>
            {preset}
          </option>
        ))}
      </select>

      <label className="field-label" htmlFor="pair-code">
        Pairing code
      </label>
      <input
        id="pair-code"
        type="text"
        value={pairingCode}
        onChange={(event) => onPairingCodeChange(event.target.value)}
        placeholder="Enter pairing code"
      />
      <button type="button" onClick={onPair}>
        Pair with Hub
      </button>
      <p className="status-feedback">{model.pairingFeedback}</p>
    </aside>
  );
}
