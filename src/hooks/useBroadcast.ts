import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { runOnTerminalHandle } from "../lib/runOnTerminalHandle";
import type { HostId, TabMeta, Workspace } from "../lib/types";
import type { TerminalTabHandle } from "../components/TerminalTab";

export const SPLIT_PANE_ID = "split-pane";

interface UseBroadcastParams {
  tabs: TabMeta[];
  splitOpen: boolean;
  splitSource: "local" | HostId;
  workspace: Workspace | null;
  terminalRefs: RefObject<Map<string, TerminalTabHandle>>;
}

/** Broadcast/live-sync: send one command (or mirror keystrokes) to a chosen
 * set of open terminals, including the split view's second panel — which
 * lives outside the tab list, so it's added to the target list by hand.
 * Extracted from App.tsx, same "moves the code, not the coupling" spirit as
 * useTabs (needs tabs/split state/workspace passed in). */
export function useBroadcast({ tabs, splitOpen, splitSource, workspace, terminalRefs }: UseBroadcastParams) {
  const [broadcastMode, setBroadcastMode] = useState(false);
  const splitPaneLabel = splitSource === "local"
    ? "Terminal local (panneau 2)"
    : `${workspace?.hosts.find((h) => h.id === splitSource)?.label ?? "Hôte"} (panneau 2)`;
  // The split view's second panel is a standalone terminal outside the tab list, so
  // it has to be added to the broadcast/snippet target list by hand.
  const broadcastTargets: { id: string; label: string }[] = [
    ...tabs
      .filter((t) => (t.kind === "terminal" || t.kind === "local-terminal" || t.kind === "rdp-view") && t.status !== "placeholder")
      .map((t) => ({ id: t.id, label: t.label })),
    ...(splitOpen ? [{ id: SPLIT_PANE_ID, label: splitPaneLabel }] : []),
  ];
  const [broadcastSelected, setBroadcastSelected] = useState<Set<string>>(new Set());

  // Terminals that open *after* broadcast mode was turned on (a new tab, the split
  // view's second panel, …) join the selection automatically, and ones that close
  // (including the split view's panel) drop out of both the selection and the count.
  // A brand-new id is only auto-added the first time it's ever seen — reordering tabs
  // (or any other change that merely re-triggers this effect) must never resurrect a
  // target the user deliberately unchecked earlier.
  const knownTargetIdsRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    const currentIds = new Set(broadcastTargets.map((t) => t.id));
    const previouslyKnown = knownTargetIdsRef.current;
    if (broadcastMode) {
      setBroadcastSelected((prev) => {
        const next = new Set(prev);
        let changed = false;
        for (const id of prev) {
          if (!currentIds.has(id)) { next.delete(id); changed = true; }
        }
        for (const id of currentIds) {
          if (!previouslyKnown.has(id) && !next.has(id)) { next.add(id); changed = true; }
        }
        return changed ? next : prev;
      });
    }
    knownTargetIdsRef.current = currentIds;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [broadcastMode, tabs, splitOpen, splitSource]);

  // Not a useCallback: it needs the current render's `broadcastTargets` (which
  // depends on `tabs`, `splitOpen`, `splitSource`, `workspace`), and memoizing on
  // an incomplete dep list previously meant opening the split view (which doesn't
  // touch `tabs`) left this closure holding a stale target list — so turning
  // broadcast on right after would silently miss the split pane.
  const toggleBroadcastMode = () => {
    setBroadcastMode((v) => {
      const next = !v;
      if (next) setBroadcastSelected(new Set(broadcastTargets.map((t) => t.id)));
      else setLiveSyncMode(false);
      return next;
    });
  };

  const broadcastCommand = useCallback((command: string) => {
    for (const id of broadcastSelected) {
      const handle = terminalRefs.current.get(id);
      if (handle) runOnTerminalHandle(handle, command, tabs.find((t) => t.id === id)?.kind !== "rdp-view");
    }
  }, [broadcastSelected, tabs, terminalRefs]);

  // Live mode: keystrokes typed into whichever terminal has focus are mirrored, as
  // raw input, to every other selected terminal — unlike broadcastCommand above,
  // which sends one discrete command at a time to all of them.
  const [liveSyncMode, setLiveSyncMode] = useState(false);
  const mirrorInput = useCallback((sourceTabId: string, data: string) => {
    if (!liveSyncMode) return;
    for (const id of broadcastSelected) {
      if (id === sourceTabId) continue;
      terminalRefs.current.get(id)?.writeRaw(data);
    }
  }, [liveSyncMode, broadcastSelected, terminalRefs]);

  return {
    broadcastMode, setBroadcastMode,
    broadcastTargets, broadcastSelected, setBroadcastSelected,
    toggleBroadcastMode, broadcastCommand,
    liveSyncMode, setLiveSyncMode, mirrorInput,
  };
}
