import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { api } from "../lib/api";
import type { Entry, Host, PaneListed, PaneOpened, PaneSource, PaneState, Workspace } from "../lib/types";

type Side = "left" | "right";
type PanesState = Record<Side, PaneState>;

type Action =
  | { type: "opening"; side: Side; source: PaneSource }
  | { type: "opened"; side: Side; result: PaneOpened }
  | { type: "failed"; side: Side; error: string }
  | { type: "listed"; side: Side; result: PaneListed };

function reducer(state: PanesState, action: Action): PanesState {
  const pane = state[action.side];
  switch (action.type) {
    case "opening":
      return { ...state, [action.side]: { source: action.source, status: "connecting", paneId: null, cwd: "", entries: [] } };
    case "opened":
      return { ...state, [action.side]: { ...pane, status: "open", paneId: action.result.paneId, cwd: action.result.cwd, entries: action.result.entries } };
    case "failed":
      return { ...state, [action.side]: { ...pane, status: "failed", error: action.error } };
    case "listed":
      return { ...state, [action.side]: { ...pane, cwd: action.result.cwd, entries: action.result.entries } };
  }
}

interface TransferTabProps {
  host: Host;
  workspace: Workspace;
  onError: (message: string) => void;
}

export function TransferTab({ host, workspace, onError }: TransferTabProps) {
  const [leftPercent, setLeftPercent] = useState(50);
  const dividerDragRef = useRef<{ startX: number; startPercent: number; containerWidth: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!dividerDragRef.current) return;
      const { startX, startPercent, containerWidth } = dividerDragRef.current;
      const delta = e.clientX - startX;
      const pct = startPercent + (delta / containerWidth) * 100;
      setLeftPercent(Math.max(20, Math.min(80, pct)));
    };
    const onUp = () => {
      if (dividerDragRef.current) {
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        dividerDragRef.current = null;
      }
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const onDividerDrag = useCallback((e: React.MouseEvent) => {
    const container = containerRef.current;
    if (!container) return;
    dividerDragRef.current = { startX: e.clientX, startPercent: leftPercent, containerWidth: container.clientWidth };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    e.preventDefault();
  }, [leftPercent]);

  const [state, dispatch] = useReducer(reducer, undefined, (): PanesState => ({
    left: { source: { kind: "local" }, status: "connecting", paneId: null, cwd: "", entries: [] },
    right: { source: { kind: "remote", hostId: host.id }, status: "connecting", paneId: null, cwd: "", entries: [] },
  }));
  const paneIds = useRef<Record<Side, string | null>>({ left: null, right: null });

  const openPaneFor = async (side: Side, source: PaneSource) => {
    dispatch({ type: "opening", side, source });
    try {
      const result = await api.openPane(source);
      paneIds.current[side] = result.paneId;
      dispatch({ type: "opened", side, result });
    } catch (e) {
      dispatch({ type: "failed", side, error: String(e) });
    }
  };

  useEffect(() => {
    openPaneFor("left", { kind: "local" });
    openPaneFor("right", { kind: "remote", hostId: host.id });
    return () => {
      if (paneIds.current.left) api.closePane(paneIds.current.left).catch(() => {});
      if (paneIds.current.right) api.closePane(paneIds.current.right).catch(() => {});
    };
    // Only re-run if the tab's own identity changes (it never does after mount).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const navigate = async (side: Side, path: string) => {
    const paneId = paneIds.current[side];
    if (!paneId) return;
    try {
      const result = await api.listPane(paneId, path);
      dispatch({ type: "listed", side, result });
    } catch (e) {
      onError(String(e));
    }
  };

  const changeSource = async (side: Side, source: PaneSource) => {
    const oldId = paneIds.current[side];
    if (oldId) api.closePane(oldId).catch(() => {});
    paneIds.current[side] = null;
    await openPaneFor(side, source);
  };

  const copy = async (side: Side, entry: Entry) => {
    const destSide: Side = side === "left" ? "right" : "left";
    const sourceId = paneIds.current[side];
    const destId = paneIds.current[destSide];
    if (!sourceId || !destId) return;
    try {
      const result = await api.copyEntry(sourceId, state[side].cwd, entry, destId, state[destSide].cwd);
      dispatch({ type: "listed", side: destSide, result });
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <div ref={containerRef} className="flex min-h-0 flex-1">
      <div style={{ width: `${leftPercent}%` }} className="flex min-h-0 shrink-0 flex-col overflow-hidden">
        <PaneView side="left" pane={state.left} workspace={workspace} onNavigate={navigate} onSourceChange={changeSource} onCopy={copy} />
      </div>
      <div
        onMouseDown={onDividerDrag}
        className="group relative flex w-1 shrink-0 cursor-col-resize items-center justify-center"
      >
        <div className="h-full w-px bg-slate-800 transition-colors group-hover:bg-[var(--c-accent)]" />
      </div>
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <PaneView side="right" pane={state.right} workspace={workspace} onNavigate={navigate} onSourceChange={changeSource} onCopy={copy} />
      </div>
    </div>
  );
}

function parentPath(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

function joinPath(base: string, segment: string): string {
  return base.endsWith("/") ? `${base}${segment}` : `${base}/${segment}`;
}

interface PaneViewProps {
  side: Side;
  pane: PaneState;
  workspace: Workspace;
  onNavigate: (side: Side, path: string) => void;
  onSourceChange: (side: Side, source: PaneSource) => void;
  onCopy: (side: Side, entry: Entry) => void;
}

function PaneView({ side, pane, workspace, onNavigate, onSourceChange, onCopy }: PaneViewProps) {
  const [gotoPath, setGotoPath] = useState("");
  const copyLabel = side === "left" ? "→" : "←";

  const sourceValue = pane.source.kind === "local" ? "local" : pane.source.hostId;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-2 border-b border-[var(--c-border)] p-2">
        <select
          value={sourceValue}
          onChange={(e) => {
            const value = e.target.value;
            onSourceChange(side, value === "local" ? { kind: "local" } : { kind: "remote", hostId: value });
          }}
          className="rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
        >
          <option value="local">Local</option>
          {workspace.hosts.map((h) => (
            <option key={h.id} value={h.id}>
              {h.label}
            </option>
          ))}
        </select>
        {pane.status === "connecting" && <span className="text-xs text-slate-500">connexion…</span>}
      </div>

      {pane.status === "failed" && <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-rose-300">Erreur : {pane.error}</div>}

      {pane.status === "open" && (
        <>
          <div className="flex items-center gap-2 border-b border-[var(--c-border)] p-2">
            <button onClick={() => onNavigate(side, parentPath(pane.cwd))} className="rounded-md bg-slate-800 px-2 py-1 text-sm hover:bg-slate-700">
              ↑
            </button>
            <span className="min-w-0 flex-1 truncate text-xs text-slate-400" title={pane.cwd}>
              {pane.cwd}
            </span>
            <input
              value={gotoPath}
              onChange={(e) => setGotoPath(e.target.value)}
              placeholder="Aller à…"
              className="w-32 rounded-md bg-slate-800 px-2 py-1 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
            />
            <button onClick={() => onNavigate(side, gotoPath)} className="rounded-md bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700">
              Aller
            </button>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto">
            {pane.entries.map((entry) => (
              <div key={entry.name} className="flex items-center gap-2 px-2 py-1 text-sm hover:bg-[var(--c-bg2)]">
                <button
                  onClick={() => entry.isDir && onNavigate(side, joinPath(pane.cwd, entry.name))}
                  className={`min-w-0 flex-1 truncate text-left ${entry.isDir ? "font-medium text-[var(--c-accent-text)]" : "text-slate-200"}`}
                >
                  {entry.isDir ? "📁 " : "📄 "}
                  {entry.name}
                </button>
                {!entry.isDir && <span className="shrink-0 text-xs text-slate-500">{entry.size} o</span>}
                <button onClick={() => onCopy(side, entry)} title={entry.isDir ? "Copier le dossier vers l'autre panneau" : "Copier vers l'autre panneau"} className="shrink-0 rounded-md bg-slate-800 px-1.5 py-0.5 text-xs hover:bg-[var(--c-accent)]">
                  {copyLabel}
                </button>
              </div>
            ))}
            {pane.entries.length === 0 && <p className="px-2 py-4 text-center text-xs text-slate-500">Dossier vide</p>}
          </div>
        </>
      )}
    </div>
  );
}
