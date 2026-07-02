import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./lib/api";
import type { GroupId, Host, HostId, Workspace } from "./lib/types";
import { Sidebar, type SidebarPanelKind } from "./components/Sidebar";
import { HostForm } from "./components/HostForm";
import { TabBar } from "./components/TabBar";
import { TerminalTab, type TerminalTabHandle } from "./components/TerminalTab";
import { LocalTerminalTab } from "./components/LocalTerminalTab";
import { TransferTab } from "./components/TransferTab";
import { TitleBar } from "./components/TitleBar";
import { type AppPreferences, ACCENT_COLORS, BG_THEMES, loadPreferences, savePreferences } from "./lib/preferences";
import { SplitPane } from "./components/SplitPane";
import { GroupForm, type GroupFormData } from "./components/GroupForm";
import { IconTerminal, IconClose } from "./components/ui-icons";

type TabMeta =
  | { id: string; kind: "terminal" | "transfer"; hostId: HostId; label: string }
  | { id: string; kind: "local-terminal"; label: string; initialCommand?: string };

let nextTabId = 0;

export default function App() {
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [sidebarPanel, setSidebarPanel] = useState<SidebarPanelKind>("hosts");
  const [editingHost, setEditingHost] = useState<Host | "new" | null>(null);
  const [editingGroup, setEditingGroup] = useState<GroupFormData | null>(null);
  const [newHostDefaultGroupId, setNewHostDefaultGroupId] = useState<GroupId | null>(null);
  const [tabs, setTabs] = useState<TabMeta[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [preferences, setPreferences] = useState<AppPreferences>(loadPreferences);
  const [splitOpen, setSplitOpen] = useState(false);
  const terminalRefs = useRef<Map<string, TerminalTabHandle>>(new Map());

  // ── Resizable panels ─────────────────────────────────────────────────────
  const [sidebarWidth, setSidebarWidth] = useState(320);
  const [rightPanelWidth, setRightPanelWidth] = useState(420);
  const [splitPercent, setSplitPercent] = useState(50);
  const [isDragging, setIsDragging] = useState(false);
  const sidebarDragData = useRef<{ startX: number; startWidth: number } | null>(null);
  const rightDragData = useRef<{ startX: number; startWidth: number } | null>(null);
  const splitDragData = useRef<{ startX: number; startPercent: number; containerWidth: number } | null>(null);
  const splitContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (sidebarDragData.current) {
        const delta = e.clientX - sidebarDragData.current.startX;
        setSidebarWidth(Math.max(200, Math.min(600, sidebarDragData.current.startWidth + delta)));
      }
      if (rightDragData.current) {
        const delta = rightDragData.current.startX - e.clientX;
        setRightPanelWidth(Math.max(280, Math.min(700, rightDragData.current.startWidth + delta)));
      }
      if (splitDragData.current) {
        const { startX, startPercent, containerWidth } = splitDragData.current;
        const delta = e.clientX - startX;
        const pct = startPercent + (delta / containerWidth) * 100;
        setSplitPercent(Math.max(20, Math.min(80, pct)));
      }
    };
    const onUp = () => {
      if (sidebarDragData.current || rightDragData.current || splitDragData.current) {
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        sidebarDragData.current = null;
        rightDragData.current = null;
        splitDragData.current = null;
        setIsDragging(false);
      }
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const onSidebarDragStart = useCallback((e: React.MouseEvent) => {
    sidebarDragData.current = { startX: e.clientX, startWidth: sidebarWidth };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    setIsDragging(true);
    e.preventDefault();
  }, [sidebarWidth]);

  const onRightDragStart = useCallback((e: React.MouseEvent) => {
    rightDragData.current = { startX: e.clientX, startWidth: rightPanelWidth };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    setIsDragging(true);
    e.preventDefault();
  }, [rightPanelWidth]);

  const onSplitDragStart = useCallback((e: React.MouseEvent) => {
    const container = splitContainerRef.current;
    if (!container) return;
    splitDragData.current = { startX: e.clientX, startPercent: splitPercent, containerWidth: container.clientWidth };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    setIsDragging(true);
    e.preventDefault();
  }, [splitPercent]);

  // ── Preferences ──────────────────────────────────────────────────────────
  const updatePreferences = useCallback((p: AppPreferences) => {
    savePreferences(p);
    setPreferences(p);
  }, []);

  useEffect(() => {
    const colors = ACCENT_COLORS[preferences.uiAccent ?? "indigo"];
    if (!colors) return;
    const root = document.documentElement;
    root.style.setProperty("--c-accent", colors.c600);
    root.style.setProperty("--c-accent-hover", colors.c500);
    root.style.setProperty("--c-accent-text", colors.c300);
    root.style.setProperty("--c-accent-dim", colors.dim);
  }, [preferences.uiAccent]);

  useEffect(() => {
    const bg = BG_THEMES[preferences.uiBg ?? "slate"];
    if (!bg) return;
    const root = document.documentElement;
    root.style.setProperty("--c-bg", bg.bg);
    root.style.setProperty("--c-bg2", bg.bg2);
    root.style.setProperty("--c-bg3", bg.bg3);
    root.style.setProperty("--c-border", bg.border);
  }, [preferences.uiBg]);

  useEffect(() => {
    api.getWorkspace().then(setWorkspace).catch((e) => setStatus(String(e)));
  }, []);

  const refreshWorkspace = useCallback((next: Workspace) => setWorkspace(next), []);

  // ── Tab management ───────────────────────────────────────────────────────
  const openTab = useCallback((kind: "terminal" | "transfer", host: Host) => {
    const id = `tab-${nextTabId++}`;
    const label = kind === "terminal" ? host.label : `Transfert : ${host.label}`;
    setTabs((prev) => [...prev, { id, kind, hostId: host.id, label }]);
    setActiveTabId(id);
  }, []);

  const openLocalTerminal = useCallback((initialCommand?: string) => {
    const id = `tab-${nextTabId++}`;
    const label = initialCommand ? `ssh ${initialCommand.replace(/^ssh\s+/, "")}` : "Terminal local";
    setTabs((prev) => [...prev, { id, kind: "local-terminal", label, initialCommand }]);
    setActiveTabId(id);
  }, []);

  const toggleSplit = useCallback(() => setSplitOpen((v) => !v), []);

  const closeTab = useCallback((id: string) => {
    terminalRefs.current.get(id)?.dispose();
    terminalRefs.current.delete(id);
    setTabs((prev) => {
      const next = prev.filter((t) => t.id !== id);
      setActiveTabId((current) => (current === id ? (next.length > 0 ? next[next.length - 1].id : null) : current));
      return next;
    });
  }, []);

  const runSnippetOnActiveTerminal = useCallback((command: string) => {
    if (!activeTabId) { setStatus("Aucun terminal actif pour exécuter ce snippet"); return; }
    const handle = terminalRefs.current.get(activeTabId);
    if (!handle) { setStatus("L'onglet actif n'est pas un terminal"); return; }
    handle.runCommand(command);
  }, [activeTabId]);

  if (!workspace) {
    return (
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-[var(--c-bg)] text-slate-100">
        <TitleBar sidebarVisible={sidebarVisible} onToggleSidebar={() => setSidebarVisible((v) => !v)} />
        <div className="flex flex-1 items-center justify-center text-slate-400">Chargement…</div>
      </div>
    );
  }

  const showRightPanel = !!(editingHost || editingGroup);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-[var(--c-bg)] text-slate-100">
      {/* Transparent overlay during any drag — prevents xterm canvas from stealing mouse events */}
      {isDragging && <div className="fixed inset-0 z-[9999] cursor-col-resize" />}
      <TitleBar sidebarVisible={sidebarVisible} onToggleSidebar={() => setSidebarVisible((v) => !v)} />

      {status && (
        <div className="flex shrink-0 items-center justify-between bg-amber-900/60 px-4 py-2 text-sm text-amber-100">
          <span>{status}</span>
          <button className="flex items-center justify-center rounded p-1 hover:bg-amber-800" onClick={() => setStatus(null)} aria-label="Fermer">
            <IconClose size={12} />
          </button>
        </div>
      )}

      {/* Full-width tab bar — spans above sidebar + content, immune to sidebar resizing */}
      {tabs.length > 0 && (
        <TabBar
          tabs={tabs}
          activeTabId={activeTabId}
          splitOpen={splitOpen}
          onSelect={setActiveTabId}
          onClose={closeTab}
          onToggleSplit={toggleSplit}
        />
      )}

      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Sidebar */}
        <div
          style={{ width: sidebarVisible ? sidebarWidth : 0 }}
          className={`flex shrink-0 overflow-hidden ${isDragging ? "" : "transition-[width] duration-200 ease-in-out"}`}
        >
          <Sidebar
            workspace={workspace}
            panel={sidebarPanel}
            onPanelChange={setSidebarPanel}
            onConnect={(host) => openTab("terminal", host)}
            onOpenTransfer={(host) => openTab("transfer", host)}
            onOpenLocalTerminal={() => openLocalTerminal()}
            onQuickSSH={(cmd) => openLocalTerminal(cmd)}
            onNewHost={() => { setEditingHost("new"); setNewHostDefaultGroupId(null); setEditingGroup(null); }}
            onEditHost={(host) => { setEditingHost(host); setEditingGroup(null); }}
            onNewGroup={() => { setEditingGroup({ id: null, name: "", parentId: null, icon: null }); setEditingHost(null); }}
            onNewHostInGroup={(groupId) => { setEditingHost("new"); setNewHostDefaultGroupId(groupId); setEditingGroup(null); }}
            onNewGroupUnder={(parentId) => { setEditingGroup({ id: null, name: "", parentId, icon: null }); setEditingHost(null); }}
            onEditGroup={(group) => { setEditingGroup({ id: group.id, name: group.name, parentId: group.parentId ?? null, icon: group.icon ?? null }); setEditingHost(null); }}
            onWorkspaceUpdate={refreshWorkspace}
            onAddSnippet={(name, command) => api.addSnippet(name, command).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onDeleteSnippet={(id) => api.deleteSnippet(id).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onRunSnippet={runSnippetOnActiveTerminal}
            onAddForward={(input) => api.addForward(input).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onDeleteForward={(id) => api.deleteForward(id).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onAddKey={(name, path, passphrase) => api.addPrivateKey(name, path, passphrase).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onDeleteKey={(id) => api.deletePrivateKey(id).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onRenameKey={(id, name) => api.renamePrivateKey(id, name).then(refreshWorkspace).catch((e) => setStatus(String(e)))}
            onError={setStatus}
            preferences={preferences}
            onPreferencesChange={updatePreferences}
          />
        </div>

        {/* Sidebar resize handle */}
        {sidebarVisible && (
          <div
            onMouseDown={onSidebarDragStart}
            className="group relative flex w-1 shrink-0 cursor-col-resize items-center justify-center"
          >
            <div className="h-full w-px bg-[var(--c-border)] transition-colors group-hover:bg-[var(--c-accent)]" />
          </div>
        )}

        {/* Main content */}
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
          {tabs.length === 0 ? (
            <div className="flex flex-1 select-none flex-col items-center justify-center gap-4">
              <div className="flex h-14 w-14 items-center justify-center rounded-2xl border border-[var(--c-border)] bg-[var(--c-bg2)]">
                <IconTerminal size={28} className="text-slate-700" />
              </div>
              <div className="text-center">
                <p className="text-sm text-slate-500">Aucun terminal ouvert</p>
                <p className="mt-0.5 text-xs text-slate-600">Choisissez un hôte dans la barre latérale</p>
              </div>
            </div>
          ) : (
            <div ref={splitContainerRef} className="flex min-h-0 flex-1">
              {/* Primary pane */}
              <div
                className="relative min-w-0"
                style={{ width: splitOpen ? `${splitPercent}%` : "100%" }}
              >
                {tabs.map((tab) => {
                  const isActive = tab.id === activeTabId;
                  if (tab.kind === "local-terminal") {
                    return (
                      <div key={tab.id} className={isActive ? "absolute inset-0 flex flex-col" : "hidden"}>
                        <LocalTerminalTab
                          isActive={isActive}
                          preferences={preferences}
                          initialCommand={tab.initialCommand}
                          onDisconnect={() => closeTab(tab.id)}
                          ref={(handle) => {
                            if (handle) terminalRefs.current.set(tab.id, handle);
                            else terminalRefs.current.delete(tab.id);
                          }}
                        />
                      </div>
                    );
                  }
                  const host = workspace.hosts.find((h) => h.id === tab.hostId);
                  if (!host) return null;
                  return (
                    <div key={tab.id} className={isActive ? "absolute inset-0 flex flex-col" : "hidden"}>
                      {tab.kind === "terminal" ? (
                        <TerminalTab
                          host={host}
                          isActive={isActive}
                          preferences={preferences}
                          onDisconnect={() => closeTab(tab.id)}
                          ref={(handle) => {
                            if (handle) terminalRefs.current.set(tab.id, handle);
                            else terminalRefs.current.delete(tab.id);
                          }}
                        />
                      ) : (
                        <TransferTab host={host} workspace={workspace} onError={setStatus} />
                      )}
                    </div>
                  );
                })}
              </div>

              {/* Split pane resize handle + secondary pane */}
              {splitOpen && (
                <>
                  <div
                    onMouseDown={onSplitDragStart}
                    className="group relative flex w-1 shrink-0 cursor-col-resize items-center justify-center"
                  >
                    <div className="h-full w-px bg-[var(--c-border)] transition-colors group-hover:bg-[var(--c-accent)]" />
                  </div>
                  <SplitPane workspace={workspace} preferences={preferences} />
                </>
              )}
            </div>
          )}
        </main>

        {/* Right panel resize handle */}
        {showRightPanel && (
          <div
            onMouseDown={onRightDragStart}
            className="group relative flex w-1 shrink-0 cursor-col-resize items-center justify-center"
          >
            <div className="h-full w-px bg-[var(--c-border)] transition-colors group-hover:bg-[var(--c-accent)]" />
          </div>
        )}

        {/* Right edit panel */}
        <div
          style={{ width: showRightPanel ? rightPanelWidth : 0 }}
          className={`flex shrink-0 flex-col overflow-hidden bg-[var(--c-bg)] ${isDragging ? "" : "transition-[width] duration-200 ease-in-out"}`}
        >
          {editingHost && (
            <HostForm
              workspace={workspace}
              host={editingHost === "new" ? null : editingHost}
              defaultGroupId={editingHost === "new" ? newHostDefaultGroupId : null}
              onCancel={() => setEditingHost(null)}
              onSave={(input) => {
                api.saveHost(input)
                  .then((ws) => { refreshWorkspace(ws); setEditingHost(null); })
                  .catch((e) => setStatus(String(e)));
              }}
              onDeleteHost={editingHost !== "new" ? (id) => {
                api.deleteHost(id)
                  .then((ws) => { refreshWorkspace(ws); setEditingHost(null); })
                  .catch((e) => setStatus(String(e)));
              } : undefined}
              onWorkspaceUpdate={refreshWorkspace}
            />
          )}
          {editingGroup && (
            <GroupForm
              workspace={workspace}
              group={editingGroup}
              onCancel={() => setEditingGroup(null)}
              onSave={(input) => {
                api.saveGroup(input)
                  .then((ws) => { refreshWorkspace(ws); setEditingGroup(null); })
                  .catch((e) => setStatus(String(e)));
              }}
              onDeleteGroup={editingGroup.id ? (id) => {
                api.deleteGroup(id)
                  .then((ws) => { refreshWorkspace(ws); setEditingGroup(null); })
                  .catch((e) => setStatus(String(e)));
              } : undefined}
              onWorkspaceUpdate={refreshWorkspace}
            />
          )}
        </div>
      </div>
    </div>
  );
}
