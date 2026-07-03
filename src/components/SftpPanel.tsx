import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Group, GroupId, Host, Workspace } from "../lib/types";
import { HostIcon } from "./icons";
import { IconSearch, IconHosts, IconFolder, IconTransfer, IconChevronDown, IconChevronRight } from "./ui-icons";

interface SftpPanelProps {
  workspace: Workspace;
  onOpenTransfer: (host: Host) => void;
}

export function SftpPanel({ workspace, onOpenTransfer }: SftpPanelProps) {
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<GroupId>>(new Set());
  const [hostStatus, setHostStatus] = useState<Record<string, boolean>>({});

  const hostIdsKey = workspace.hosts.map((h) => h.id).join(",");
  useEffect(() => {
    let cancelled = false;
    const poll = () => {
      for (const host of workspace.hosts) {
        api.checkHostStatus(host.id)
          .then((online) => { if (!cancelled) setHostStatus((prev) => ({ ...prev, [host.id]: online })); })
          .catch(() => { if (!cancelled) setHostStatus((prev) => ({ ...prev, [host.id]: false })); });
      }
    };
    poll();
    const interval = setInterval(poll, 30000);
    return () => { cancelled = true; clearInterval(interval); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hostIdsKey]);

  const query = search.trim().toLowerCase();
  const matches = (host: Host) =>
    !query || host.label.toLowerCase().includes(query) || host.address.toLowerCase().includes(query) ||
    host.username.toLowerCase().includes(query) || host.tags.some((t) => t.toLowerCase().includes(query));

  const childGroups = (parentId: GroupId | null) =>
    workspace.groups.filter((g) => g.parentId === parentId).sort((a, b) => a.name.localeCompare(b.name));
  const hostsIn = (groupId: GroupId | null) =>
    workspace.hosts.filter((h) => h.groupId === groupId && matches(h)).sort((a, b) => a.label.localeCompare(b.label));
  const isExpanded = (id: GroupId) => (query ? true : !collapsed.has(id));

  function groupHasMatches(groupId: GroupId): boolean {
    if (hostsIn(groupId).length > 0) return true;
    return childGroups(groupId).some((g) => groupHasMatches(g.id));
  }

  const toggleGroup = (id: GroupId) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const renderHost = (host: Host, depth: number) => (
    <div
      key={host.id}
      style={{ marginLeft: depth * 14 }}
      className="group rounded-lg border border-[var(--c-border)] bg-[var(--c-bg3)]/40 transition-colors hover:border-[var(--c-accent-dim)] hover:bg-[var(--c-bg3)]"
    >
      <div className="flex items-stretch">
        <button
          onClick={() => onOpenTransfer(host)}
          className="flex min-w-0 flex-1 items-center gap-2.5 p-2.5 text-left"
          title={`Transférer — ${host.username}@${host.address}:${host.port}`}
        >
          <div className="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-[var(--c-accent-dim)]">
            {host.icon
              ? <HostIcon iconId={host.icon} customIcons={workspace.customIcons} size={18} />
              : <IconHosts size={14} className="text-[var(--c-accent-text)]" />
            }
            {hostStatus[host.id] !== undefined && (
              <span
                title={hostStatus[host.id] ? "En ligne" : "Hors ligne"}
                className={`absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-[var(--c-bg2)] ${
                  hostStatus[host.id] ? "bg-emerald-500" : "bg-slate-600"
                }`}
              />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-medium text-slate-100">{host.label}</div>
            <div className="truncate text-[11px] text-slate-500">
              {host.username}@{host.address}{host.port !== 22 ? `:${host.port}` : ""}
            </div>
          </div>
        </button>
        <div className="flex shrink-0 items-center px-2 text-slate-500 opacity-0 transition-all group-hover:opacity-100">
          <IconTransfer size={14} />
        </div>
      </div>

      {host.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-2.5 pb-2">
          {host.tags.map((tag) => (
            <span key={tag} className="rounded-full bg-slate-700/60 px-1.5 py-0.5 text-[10px] text-slate-400">
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );

  const renderGroup = (group: Group, depth: number) => {
    if (query && !groupHasMatches(group.id)) return null;
    const expanded = isExpanded(group.id);
    return (
      <div key={group.id} className="space-y-1">
        <div
          style={{ marginLeft: depth * 14 }}
          className="flex items-center gap-0.5 rounded-md px-1 py-1 hover:bg-slate-800/60"
        >
          <button onClick={() => toggleGroup(group.id)} className="flex w-4 shrink-0 items-center justify-center text-slate-500">
            {expanded ? <IconChevronDown size={12} /> : <IconChevronRight size={12} />}
          </button>
          <span className="flex min-w-0 flex-1 items-center gap-1.5 truncate text-sm font-medium text-slate-300">
            {group.icon ? (
              <HostIcon iconId={group.icon} customIcons={workspace.customIcons} size={16} />
            ) : (
              <IconFolder size={14} className="text-slate-500" />
            )}
            {group.name}
          </span>
        </div>
        {expanded && (
          <div className="space-y-1">
            {hostsIn(group.id).map((h) => renderHost(h, depth + 1))}
            {childGroups(group.id).map((g) => renderGroup(g, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="flex h-full min-w-0 flex-col gap-2">
      <div className="relative">
        <div className="pointer-events-none absolute inset-y-0 left-2.5 flex items-center">
          <IconSearch size={13} className="text-slate-500" />
        </div>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Rechercher un hôte…"
          className="w-full rounded-md bg-[var(--c-bg3)] pl-8 pr-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent)]"
        />
      </div>
      <div className="sidebar-scroll min-h-0 min-w-0 flex-1 space-y-1 overflow-y-auto">
        {hostsIn(null).map((h) => renderHost(h, 0))}
        {childGroups(null).map((g) => renderGroup(g, 0))}
        {workspace.hosts.length === 0 && (
          <p className="px-1 py-4 text-center text-sm text-slate-500">Aucun hôte enregistré</p>
        )}
      </div>
    </div>
  );
}
