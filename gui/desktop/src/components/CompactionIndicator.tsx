interface CompactionIndicatorProps {
  active: boolean;
  queueDepth: number;
}

export default function CompactionIndicator({ active, queueDepth }: CompactionIndicatorProps) {
  return (
    <div className="indicator-row">
      <span className={`indicator-dot ${active ? "online" : "idle"}`} />
      <span className="indicator-label">Compaction: {active ? "Active" : "Idle"}</span>
      <span className="indicator-meta">Queue {queueDepth}</span>
    </div>
  );
}
