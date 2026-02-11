import type { ThreadItem } from "../types";

interface ThreadsPaneProps {
  threads: ThreadItem[];
  selectedThreadId: string;
  onSelectThread: (threadId: string) => void;
}

export default function ThreadsPane({ threads, selectedThreadId, onSelectThread }: ThreadsPaneProps) {
  return (
    <section className="pane threads-pane">
      <h2>Threads</h2>
      <ul className="thread-list">
        {threads.map((thread) => {
          const selected = selectedThreadId === thread.id;
          return (
            <li key={thread.id}>
              <button
                type="button"
                className={`thread-button ${selected ? "selected" : ""}`}
                onClick={() => onSelectThread(thread.id)}
              >
                <div className="thread-title-row">
                  <span>{thread.title}</span>
                  <span className="thread-time">{thread.lastActivity}</span>
                </div>
                <div className="thread-meta">Unread: {thread.unread}</div>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
