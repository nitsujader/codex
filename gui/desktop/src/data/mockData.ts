import type { AgentCard, ThreadItem, TimelineEvent } from "../types";

export const THREADS: ThreadItem[] = [
  { id: "th-101", title: "Product Sync", lastActivity: "09:12", unread: 2 },
  { id: "th-102", title: "Platform Migration", lastActivity: "08:49", unread: 0 },
  { id: "th-103", title: "Inbox Triage", lastActivity: "08:20", unread: 4 },
  { id: "th-104", title: "Design Review", lastActivity: "Yesterday", unread: 0 },
];

export const TIMELINE_EVENTS: TimelineEvent[] = [
  { id: "ev-1", at: "09:12", text: "User follow-up drafted for Product Sync." },
  { id: "ev-2", at: "09:10", text: "Hub heartbeat acknowledged by local service." },
  { id: "ev-3", at: "09:05", text: "Compaction queue processed 3 context chunks." },
  { id: "ev-4", at: "08:56", text: "Agent board refreshed from pairing metadata." },
  { id: "ev-5", at: "08:49", text: "Platform migration notes attached to thread th-102." },
];

export const AGENTS: AgentCard[] = [
  {
    id: "ag-11",
    name: "Planner",
    state: "working",
    task: "Breaking roadmap into execution slices",
    compactQueueDepth: 1,
  },
  {
    id: "ag-12",
    name: "Reviewer",
    state: "idle",
    task: "Waiting for diff to review",
    compactQueueDepth: 0,
  },
  {
    id: "ag-13",
    name: "Operator",
    state: "offline",
    task: "Desktop bridge disconnected",
    compactQueueDepth: 0,
  },
];
