import type { ThemePreset } from "./theme/presets";

export type AgentState = "idle" | "working" | "offline" | "error";

export interface ThreadItem {
  id: string;
  title: string;
  lastActivity: string;
  unread: number;
}

export interface TimelineEvent {
  id: string;
  at: string;
  text: string;
}

export interface AgentCard {
  id: string;
  name: string;
  state: AgentState;
  task: string;
  compactQueueDepth: number;
}

export interface HubStatus {
  reachable: boolean;
  service: string;
  paired: boolean;
  activeAgents: number;
  latencyMs?: number;
}

export interface PairingRequest {
  pairingCode: string;
  deviceName?: string;
}

export interface PairingResponse {
  paired: boolean;
  message: string;
}

export interface StatusRailModel {
  endpoint: string;
  selectedTheme: ThemePreset;
  hubStatus: HubStatus;
  pairingFeedback: string;
}
