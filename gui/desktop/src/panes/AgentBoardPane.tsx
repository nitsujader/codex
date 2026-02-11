import AgentStatusBadge from "../components/AgentStatusBadge";
import CompactionIndicator from "../components/CompactionIndicator";
import type { AgentCard } from "../types";

interface AgentBoardPaneProps {
  agents: AgentCard[];
}

export default function AgentBoardPane({ agents }: AgentBoardPaneProps) {
  return (
    <section className="pane board-pane">
      <h2>Agent Board</h2>
      <div className="agent-grid">
        {agents.map((agent) => (
          <article key={agent.id} className="agent-card">
            <div className="agent-card-head">
              <strong>{agent.name}</strong>
              <AgentStatusBadge status={agent.state} />
            </div>
            <p className="agent-task">{agent.task}</p>
            <CompactionIndicator active={agent.compactQueueDepth > 0} queueDepth={agent.compactQueueDepth} />
          </article>
        ))}
      </div>
    </section>
  );
}
