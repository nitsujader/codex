import type { AgentState } from "../types";

interface AgentStatusBadgeProps {
  status: AgentState;
}

const STATUS_LABELS: Record<AgentState, string> = {
  idle: "Idle",
  working: "Working",
  offline: "Offline",
  error: "Error",
};

export default function AgentStatusBadge({ status }: AgentStatusBadgeProps) {
  return (
    <span className={`agent-status-badge agent-status-${status}`}>
      {STATUS_LABELS[status]}
    </span>
  );
}
