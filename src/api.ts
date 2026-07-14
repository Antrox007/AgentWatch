// Bindung an das Tauri-Backend: initialer Snapshot + Live-Updates per Event.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppSettings, MonitorInfo, Snapshot } from "./types";

export async function getSnapshot(): Promise<Snapshot> {
  return await invoke<Snapshot>("get_snapshot");
}

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  await invoke("save_settings", { settings });
}

export async function onSnapshot(
  callback: (snapshot: Snapshot) => void,
): Promise<UnlistenFn> {
  return await listen<Snapshot>("snapshot", (event) => callback(event.payload));
}

/** Verfuegbare Monitore (fuer die Auswahl der Pill-Position). */
export async function listMonitors(): Promise<MonitorInfo[]> {
  return await invoke<MonitorInfo[]>("list_monitors");
}

/** Meldet dem Backend die gemessene Pill-Groesse (CSS-Pixel) zum Positionieren. */
export async function positionIsland(
  width: number,
  height: number,
): Promise<void> {
  await invoke("position_island", { width, height });
}

/** Dashboard-Flyout oben unter der Pill auf-/zuklappen (Klick auf die Pill). */
export async function toggleDashboardTop(): Promise<void> {
  await invoke("toggle_dashboard_top");
}

/** Dashboard-Flyout schließen (Esc). */
export async function hideDashboard(): Promise<void> {
  await invoke("hide_dashboard");
}
