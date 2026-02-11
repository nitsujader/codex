import type { TimelineEvent } from "../types";

interface TimelinePaneProps {
  events: TimelineEvent[];
}

export default function TimelinePane({ events }: TimelinePaneProps) {
  return (
    <section className="pane timeline-pane">
      <h2>Timeline</h2>
      <ol className="timeline-list">
        {events.map((event) => (
          <li key={event.id} className="timeline-event">
            <span className="event-time">{event.at}</span>
            <span>{event.text}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
