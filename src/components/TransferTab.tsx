import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { api } from "../lib/api";
import type { Entry, Host, PaneListed, PaneOpened, PaneSource, PaneState, Workspace } from "../lib/types";

type Side = "left" | "right";
type PanesState = Record<Side, PaneState>;
type SortKey = "name" | "modified" | "type" | "size";
type SortDir = "asc" | "desc";

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

function fileExt(entry: Entry): string {
  if (entry.isDir) return "";
  const dot = entry.name.lastIndexOf(".");
  return dot > 0 ? entry.name.slice(dot + 1).toLowerCase() : "";
}

function fileTypeLabel(entry: Entry): string {
  if (entry.isDir) return "Dossier";
  const ext = fileExt(entry);
  return ext ? ext.toUpperCase() : "Fichier";
}

function formatSize(bytes: number, isDir: boolean): string {
  if (isDir) return "—";
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} Mo`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} Go`;
}

function formatDate(ts?: number): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString("fr-FR", {
    day: "2-digit", month: "2-digit", year: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

function sortEntries(entries: Entry[], key: SortKey, dir: SortDir): Entry[] {
  const dirs = entries.filter((e) => e.isDir);
  const files = entries.filter((e) => !e.isDir);

  const cmp = (a: Entry, b: Entry): number => {
    let v = 0;
    switch (key) {
      case "name":     v = a.name.toLowerCase().localeCompare(b.name.toLowerCase()); break;
      case "modified": v = (a.modified ?? 0) - (b.modified ?? 0); break;
      case "type":     v = fileTypeLabel(a).localeCompare(fileTypeLabel(b)) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()); break;
      case "size":     v = a.size - b.size; break;
    }
    return dir === "asc" ? v : -v;
  };

  return [...dirs.sort(cmp), ...files.sort(cmp)];
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

function ColHeader({
  label, colKey, sortKey, sortDir, onSort, className,
}: {
  label: string; colKey: SortKey; sortKey: SortKey; sortDir: SortDir;
  onSort: (k: SortKey) => void; className?: string;
}) {
  const active = colKey === sortKey;
  return (
    <button
      onClick={() => onSort(colKey)}
      className={`flex items-center gap-0.5 whitespace-nowrap text-left text-[11px] font-medium transition-colors hover:text-slate-200 ${active ? "text-[var(--c-accent-text)]" : "text-slate-500"} ${className ?? ""}`}
    >
      {label}
      <span className="text-[9px] opacity-80">{active ? (sortDir === "asc" ? " ▲" : " ▼") : ""}</span>
    </button>
  );
}

function PaneView({ side, pane, workspace, onNavigate, onSourceChange, onCopy }: PaneViewProps) {
  const [gotoPath, setGotoPath] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const copyLabel = side === "left" ? "→" : "←";

  const handleSort = (key: SortKey) => {
    if (key === sortKey) setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    else { setSortKey(key); setSortDir("asc"); }
  };

  const sorted = useMemo(() => sortEntries(pane.entries, sortKey, sortDir), [pane.entries, sortKey, sortDir]);

  const sourceValue = pane.source.kind === "local" ? "local" : pane.source.hostId;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Source selector */}
      <div className="flex items-center gap-2 border-b border-[var(--c-border)] p-2">
        <select
          value={sourceValue}
          onChange={(e) => {
            const v = e.target.value;
            onSourceChange(side, v === "local" ? { kind: "local" } : { kind: "remote", hostId: v });
          }}
          className="rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
        >
          <option value="local">Local</option>
          {workspace.hosts.map((h) => (
            <option key={h.id} value={h.id}>{h.label}</option>
          ))}
        </select>
        {pane.status === "connecting" && <span className="text-xs text-slate-500">connexion…</span>}
      </div>

      {pane.status === "failed" && (
        <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-rose-300">
          Erreur : {pane.error}
        </div>
      )}

      {pane.status === "open" && (
        <>
          {/* Navigation bar */}
          <div className="flex items-center gap-2 border-b border-[var(--c-border)] px-2 py-1.5">
            <button
              onClick={() => onNavigate(side, parentPath(pane.cwd))}
              className="shrink-0 rounded px-2 py-0.5 text-sm text-slate-400 hover:bg-slate-700 hover:text-slate-100"
              title="Dossier parent"
            >
              ↑
            </button>
            <span className="min-w-0 flex-1 truncate font-mono text-xs text-slate-400" title={pane.cwd}>
              {pane.cwd}
            </span>
            <div className="flex shrink-0 items-center gap-1">
              <input
                value={gotoPath}
                onChange={(e) => setGotoPath(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") { onNavigate(side, gotoPath); setGotoPath(""); } }}
                placeholder="Aller à…"
                className="w-28 rounded-md bg-slate-800 px-2 py-0.5 text-xs text-slate-100 placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
              />
              <button
                onClick={() => { onNavigate(side, gotoPath); setGotoPath(""); }}
                className="rounded-md bg-slate-800 px-2 py-0.5 text-xs text-slate-400 hover:bg-slate-700 hover:text-slate-100"
              >
                OK
              </button>
            </div>
          </div>

          {/* Column headers */}
          <div className="flex items-center gap-1 border-b border-[var(--c-border)] bg-[var(--c-bg3)]/60 px-2 py-1">
            <ColHeader label="Nom"          colKey="name"     sortKey={sortKey} sortDir={sortDir} onSort={handleSort} className="flex-1" />
            <ColHeader label="Modifié"      colKey="modified" sortKey={sortKey} sortDir={sortDir} onSort={handleSort} className="w-32 hidden sm:flex" />
            <ColHeader label="Type"         colKey="type"     sortKey={sortKey} sortDir={sortDir} onSort={handleSort} className="w-12" />
            <ColHeader label="Taille"       colKey="size"     sortKey={sortKey} sortDir={sortDir} onSort={handleSort} className="w-14 text-right justify-end" />
            <div className="w-6 shrink-0" />
          </div>

          {/* File list */}
          <div className="min-h-0 flex-1 overflow-y-auto">
            {sorted.map((entry) => (
              <div
                key={entry.name}
                className="group flex items-center gap-1 px-2 py-[3px] text-xs hover:bg-[var(--c-bg2)]"
              >
                {/* Name */}
                <button
                  onClick={() => entry.isDir && onNavigate(side, joinPath(pane.cwd, entry.name))}
                  className={`flex min-w-0 flex-1 items-center gap-1.5 truncate text-left ${
                    entry.isDir ? "font-medium text-[var(--c-accent-text)]" : "text-slate-200"
                  } ${entry.isDir ? "cursor-pointer" : "cursor-default"}`}
                >
                  <span className="shrink-0 text-[13px]">{entry.isDir ? "📁" : "📄"}</span>
                  <span className="truncate">{entry.name}</span>
                </button>

                {/* Modified */}
                <span className="hidden w-32 shrink-0 text-slate-500 tabular-nums sm:block">
                  {formatDate(entry.modified)}
                </span>

                {/* Type */}
                <span className="w-12 shrink-0 text-slate-500">{fileTypeLabel(entry)}</span>

                {/* Size */}
                <span className="w-14 shrink-0 text-right tabular-nums text-slate-500">
                  {formatSize(entry.size, entry.isDir)}
                </span>

                {/* Copy button */}
                <button
                  onClick={() => onCopy(side, entry)}
                  title={entry.isDir ? "Copier le dossier vers l'autre panneau" : "Copier vers l'autre panneau"}
                  className="w-6 shrink-0 rounded px-0.5 text-center text-slate-600 opacity-0 hover:bg-[var(--c-accent)] hover:text-white group-hover:opacity-100"
                >
                  {copyLabel}
                </button>
              </div>
            ))}
            {pane.entries.length === 0 && (
              <p className="px-2 py-6 text-center text-xs text-slate-500">Dossier vide</p>
            )}
          </div>
        </>
      )}
    </div>
  );
}
