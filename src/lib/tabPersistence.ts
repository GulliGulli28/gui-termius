import type { TabMeta } from "./types";

const STORAGE_KEY = "gui-termius-tabs";

export interface PersistedTab {
  kind: TabMeta["kind"];
  label: string;
  hostId?: string;
  dockerContainerId?: string;
}

/** Persists only enough to redraw placeholder tabs — never a live session id. */
export function saveTabs(tabs: TabMeta[]): void {
  const trimmed: PersistedTab[] = tabs.map((t) => ({
    kind: t.kind,
    label: t.label,
    hostId: t.kind === "terminal" || t.kind === "transfer" ? t.hostId : undefined,
    dockerContainerId: t.kind === "terminal" ? t.dockerContainerId : undefined,
  }));
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch { /* ignore */ }
}

export function loadTabs(): PersistedTab[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return [];
}
