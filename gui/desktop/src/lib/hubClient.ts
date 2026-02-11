import type { HubStatus, PairingRequest, PairingResponse } from "../types";

const HUB_BASE_URL = "http://127.0.0.1:46710";

async function hubRequest<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${HUB_BASE_URL}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
    ...init,
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Hub request failed (${response.status}): ${text}`);
  }

  return (await response.json()) as T;
}

export async function fetchHubStatus(): Promise<HubStatus> {
  return hubRequest<HubStatus>("/status");
}

export async function pairWithHub(payload: PairingRequest): Promise<PairingResponse> {
  return hubRequest<PairingResponse>("/pairing", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export { HUB_BASE_URL };
