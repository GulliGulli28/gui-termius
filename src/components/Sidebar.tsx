import { useEffect, useRef, useState } from "react";
import type { Group, GroupId, Host, HostId, KeyId, PortForwardId, SnippetId, Workspace } from "../lib/types";
import type { AppPreferences } from "../lib/preferences";
import type { ComponentType } from "react";
import { HostsPanel } from "./HostsPanel";
import { KeychainPanel } from "./KeychainPanel";
import { SettingsPanel } from "./SettingsPanel";
import { SnippetsPanel } from "./SnippetsPanel";
import { TunnelsPanel } from "./TunnelsPanel";
import { IconHosts, IconSnippets, IconTunnels, IconKeychain, IconSettings } from "./ui-icons";

export type SidebarPanelKind = "hosts" | "snippets" | "tunnels" | "keychain" | "settings";

interface SidebarProps {
  workspace: Workspace;
  panel: SidebarPanelKind;
  onPanelChange: (panel: SidebarPanelKind) => void;
  onConnect: (host: Host) => void;
  onOpenTransfer: (host: Host) => void;
  onOpenLocalTerminal: () => void;
  onQuickSSH: (cmd: string) => void;
  onNewHost: () => void;
  onEditHost: (host: Host) => void;
  onNewGroup: () => void;
  onNewHostInGroup: (groupId: GroupId) => void;
  onNewGroupUnder: (parentId: GroupId) => void;
  onEditGroup: (group: Group) => void;
  onAddSnippet: (name: string, command: string) => void;
  onUpdateSnippet: (id: SnippetId, name: string, command: string) => void;
  onDeleteSnippet: (id: SnippetId) => void;
  onRunSnippet: (command: string) => void;
  onAddForward: (input: { hostId: HostId; kind: "local" | "remote"; bindAddress: string; bindPort: number; destAddress: string; destPort: number }) => void;
  onDeleteForward: (id: PortForwardId) => void;
  onAddKey: (name: string, path: string, passphrase: string | null) => void;
  onDeleteKey: (id: KeyId) => void;
  onRenameKey: (id: KeyId, name: string) => void;
  onWorkspaceUpdate: (ws: Workspace) => void;
  onError: (message: string) => void;
  preferences: AppPreferences;
  onPreferencesChange: (p: AppPreferences) => void;
}

const TABS: { key: Exclude<SidebarPanelKind, "settings">; label: string; Icon: ComponentType<{ size?: number }> }[] = [
  { key: "hosts", label: "Hôtes", Icon: IconHosts },
  { key: "snippets", label: "Snippets", Icon: IconSnippets },
  { key: "tunnels", label: "Tunnels", Icon: IconTunnels },
  { key: "keychain", label: "Clés", Icon: IconKeychain },
];

export function Sidebar(props: SidebarProps) {
  const { workspace, panel, onPanelChange } = props;
  const asideRef = useRef<HTMLElement>(null);
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const el = asideRef.current;
    if (!el) return;
    const obs = new ResizeObserver(([entry]) => {
      setCompact(entry.contentRect.width < 260);
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  return (
    <aside ref={asideRef} className="flex min-w-0 flex-1 flex-col bg-[var(--c-bg2)]">
      {/* Tab bar + settings gear */}
      <div className="flex shrink-0 items-center gap-1 border-b border-[var(--c-border)] p-2">
        <div className="flex flex-1 gap-1">
          {TABS.map((t) => (
            <button
              key={t.key}
              onClick={() => onPanelChange(t.key)}
              title={compact ? t.label : undefined}
              className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-1 py-1.5 text-[10px] font-medium transition-colors ${
                panel === t.key ? "bg-[var(--c-accent)] text-white" : "text-slate-400 hover:bg-slate-800 hover:text-slate-200"
              }`}
            >
              <t.Icon size={13} />
              {!compact && <span>{t.label}</span>}
            </button>
          ))}
        </div>
        <button
          onClick={() => onPanelChange(panel === "settings" ? "hosts" : "settings")}
          className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors ${
            panel === "settings" ? "bg-[var(--c-accent)] text-white" : "text-slate-400 hover:bg-slate-800 hover:text-slate-200"
          }`}
          title="Paramètres"
        >
          <IconSettings size={14} />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        {panel === "hosts" && (
          <HostsPanel
            workspace={workspace}
            onConnect={props.onConnect}
            onOpenTransfer={props.onOpenTransfer}
            onOpenLocalTerminal={props.onOpenLocalTerminal}
            onQuickSSH={props.onQuickSSH}
            onNewHost={props.onNewHost}
            onEditHost={props.onEditHost}
            onNewGroup={props.onNewGroup}
            onNewHostInGroup={props.onNewHostInGroup}
            onNewGroupUnder={props.onNewGroupUnder}
            onEditGroup={props.onEditGroup}
            onWorkspaceUpdate={props.onWorkspaceUpdate}
            onError={props.onError}
          />
        )}
        {panel === "snippets" && (
          <SnippetsPanel workspace={workspace} onAddSnippet={props.onAddSnippet} onUpdateSnippet={props.onUpdateSnippet} onDeleteSnippet={props.onDeleteSnippet} onRunSnippet={props.onRunSnippet} />
        )}
        {panel === "tunnels" && (
          <TunnelsPanel workspace={workspace} onAddForward={props.onAddForward} onDeleteForward={props.onDeleteForward} onError={props.onError} />
        )}
        {panel === "keychain" && (
          <KeychainPanel workspace={workspace} onAddKey={props.onAddKey} onDeleteKey={props.onDeleteKey} onRenameKey={props.onRenameKey} />
        )}
        {panel === "settings" && (
          <SettingsPanel
            workspace={workspace}
            onWorkspaceUpdate={props.onWorkspaceUpdate}
            onError={props.onError}
            preferences={props.preferences}
            onPreferencesChange={props.onPreferencesChange}
          />
        )}
      </div>
    </aside>
  );
}
