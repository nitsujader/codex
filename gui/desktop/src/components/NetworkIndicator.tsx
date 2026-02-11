interface NetworkIndicatorProps {
  connected: boolean;
  endpoint: string;
  latencyMs?: number;
}

export default function NetworkIndicator({ connected, endpoint, latencyMs }: NetworkIndicatorProps) {
  const label = connected ? "Connected" : "Disconnected";

  return (
    <div className="indicator-row">
      <span className={`indicator-dot ${connected ? "online" : "offline"}`} />
      <span className="indicator-label">Network: {label}</span>
      <span className="indicator-meta">{endpoint}</span>
      {typeof latencyMs === "number" ? <span className="indicator-meta">{latencyMs} ms</span> : null}
    </div>
  );
}
