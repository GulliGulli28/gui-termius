import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import type { FleetOutcome, FleetRun, Host, HostFacts, HostId, Workspace } from "../lib/types";
import { api, onFleetDone, onFleetOutcome } from "../lib/api";
import { IconPlay, IconSearch, IconChevronRight, IconChevronDown, IconRefresh } from "./ui-icons";

/** Colour for a RAM-usage percentage: green under 70, amber under 85, red above. */
function ramColor(pct: number): string {
  if (pct >= 85) return "#ef4444";
  if (pct >= 70) return "#f59e0b";
  return "#22c55e";
}

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString();
}

interface FleetTabProps {
  workspace: Workspace;
  onError: (message: string) => void;
}

type RowStatus = "pending" | "ok" | "fail" | "error";

function outcomeStatus(o: FleetOutcome): RowStatus {
  if (o.error != null) return "error";
  return o.exitCode === 0 ? "ok" : "fail";
}

function statusOf(hostId: HostId, results: Map<HostId, FleetOutcome>, pending: Set<HostId>): RowStatus {
  if (pending.has(hostId)) return "pending";
  const o = results.get(hostId);
  if (!o) return "pending";
  return outcomeStatus(o);
}

function countOutcomes(outcomes: FleetOutcome[]): { ok: number; fail: number } {
  let ok = 0;
  let fail = 0;
  for (const o of outcomes) (outcomeStatus(o) === "ok" ? ok++ : fail++);
  return { ok, fail };
}

function StatusDot({ status }: { status: RowStatus }) {
  if (status === "pending") {
    return <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-[var(--c-text-faint)] border-t-transparent" />;
  }
  const color = status === "ok" ? "#22c55e" : "#ef4444";
  return <span className="inline-block h-2.5 w-2.5 rounded-full" style={{ background: color }} />;
}

export function FleetTab({ workspace, onError }: FleetTabProps) {
  const sshHosts = useMemo(() => workspace.hosts.filter((h) => h.kind === "ssh"), [workspace.hosts]);
  const hostById = useMemo(() => new Map(workspace.hosts.map((h) => [h.id, h])), [workspace.hosts]);
  const groupName = (h: Host) => (h.groupId ? workspace.groups.find((g) => g.id === h.groupId)?.name ?? "" : "");

  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<Set<HostId>>(new Set());
  const [command, setCommand] = useState("");
  const [running, setRunning] = useState(false);

  // Results of the current / last run, keyed by host, plus the ordered target
  // list so rows keep a stable order as outcomes stream in out of order.
  const [runTargets, setRunTargets] = useState<HostId[]>([]);
  const [results, setResults] = useState<Map<HostId, FleetOutcome>>(new Map());
  const [pending, setPending] = useState<Set<HostId>>(new Set());
  const [expanded, setExpanded] = useState<Set<HostId>>(new Set());
  const runIdRef = useRef<string | null>(null);

  // Collected host state (facts), keyed by host — populated on demand.
  const [facts, setFacts] = useState<Map<HostId, HostFacts>>(new Map());
  const [collectingFacts, setCollectingFacts] = useState(false);
  const [ramThreshold, setRamThreshold] = useState(80);
  const hasFacts = facts.size > 0;

  // Persisted run history (audit trail) + which panel is shown on the right.
  const [view, setView] = useState<"run" | "history">("run");
  const [history, setHistory] = useState<FleetRun[]>([]);
  const [expandedRuns, setExpandedRuns] = useState<Set<string>>(new Set());

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return sshHosts;
    return sshHosts.filter(
      (h) =>
        h.label.toLowerCase().includes(q) ||
        h.address.toLowerCase().includes(q) ||
        h.tags.some((t) => t.toLowerCase().includes(q)) ||
        groupName(h).toLowerCase().includes(q),
    );
    // groupName is derived from workspace.groups, included via sshHosts identity
  }, [sshHosts, filter, workspace.groups]);

  // One subscription for the tab's lifetime; events are matched to the active
  // run by id so a stale run's late outcomes are ignored.
  useEffect(() => {
    let disposed = false;
    let offOutcome: (() => void) | undefined;
    let offDone: (() => void) | undefined;
    onFleetOutcome((runId, outcome) => {
      if (runId !== runIdRef.current) return;
      setResults((prev) => new Map(prev).set(outcome.hostId, outcome));
      setPending((prev) => {
        const next = new Set(prev);
        next.delete(outcome.hostId);
        return next;
      });
    }).then((fn) => (disposed ? fn() : (offOutcome = fn)));
    onFleetDone((runId) => {
      if (runId !== runIdRef.current) return;
      setRunning(false);
      // The completed run was just persisted server-side — pull it in.
      api.getFleetHistory().then(setHistory).catch(() => {});
    }).then((fn) => (disposed ? fn() : (offDone = fn)));
    return () => {
      disposed = true;
      offOutcome?.();
      offDone?.();
    };
  }, []);

  // Load the persisted history once on mount.
  useEffect(() => {
    api.getFleetHistory().then(setHistory).catch(() => {});
  }, []);

  const toggle = (id: HostId) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const selectAll = () => setSelected(new Set(filtered.map((h) => h.id)));
  const selectNone = () => setSelected(new Set());

  const collectFacts = async () => {
    if (collectingFacts) return;
    const ids = sshHosts.map((h) => h.id);
    if (ids.length === 0) return;
    setCollectingFacts(true);
    try {
      const outcomes = await api.collectFacts(ids);
      setFacts((prev) => {
        const next = new Map(prev);
        for (const o of outcomes) if (o.facts) next.set(o.hostId, o.facts);
        return next;
      });
    } catch (e) {
      onError(String(e));
    } finally {
      setCollectingFacts(false);
    }
  };

  // The "select where RAM > N%" demo: needs facts collected first.
  const selectByRam = () =>
    setSelected(
      new Set(
        sshHosts
          .filter((h) => {
            const pct = facts.get(h.id)?.memUsedPct;
            return pct != null && pct > ramThreshold;
          })
          .map((h) => h.id),
      ),
    );
  const toggleExpanded = (id: HostId) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const toggleRun = (id: string) =>
    setExpandedRuns((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  // Re-run a past run: load its command + re-select the targets that still
  // exist, and switch back to the composer so the user reviews before running.
  const loadRun = (run: FleetRun) => {
    setCommand(run.command);
    setSelected(new Set(run.hostIds.filter((id) => hostById.has(id))));
    setView("run");
  };

  const run = async () => {
    if (running) return;
    const targets = [...selected];
    if (targets.length === 0) {
      onError("Sélectionne au moins un hôte");
      return;
    }
    if (!command.trim()) {
      onError("Saisis une commande à exécuter");
      return;
    }
    const runId = crypto.randomUUID();
    runIdRef.current = runId;
    setRunTargets(targets);
    setResults(new Map());
    setPending(new Set(targets));
    setExpanded(new Set());
    setRunning(true);
    try {
      await api.runFleetCommand(runId, targets, command);
    } catch (e) {
      onError(String(e));
    } finally {
      if (runIdRef.current === runId) setRunning(false);
    }
  };

  const summary = useMemo(() => {
    let ok = 0;
    let fail = 0;
    for (const id of runTargets) {
      const s = statusOf(id, results, pending);
      if (s === "ok") ok++;
      else if (s === "fail" || s === "error") fail++;
    }
    return { ok, fail, pending: pending.size, total: runTargets.length };
  }, [runTargets, results, pending]);

  return (
    <div className="flex h-full min-h-0 bg-[var(--c-bg)] text-[var(--c-text)]">
      {/* ── Target picker ─────────────────────────────────────────────── */}
      <aside className="flex w-72 shrink-0 flex-col border-r border-[var(--c-border)]">
        <div className="flex items-center justify-between px-3 py-2.5">
          <span className="text-xs font-semibold uppercase tracking-wide text-[var(--c-text-secondary)]">
            Cibles · {selected.size}/{sshHosts.length}
          </span>
          <div className="flex gap-1 text-[11px]">
            <button onClick={selectAll} className="rounded px-1.5 py-0.5 text-[var(--c-accent-text)] hover:bg-[var(--c-bg3)]">
              Tout
            </button>
            <button onClick={selectNone} className="rounded px-1.5 py-0.5 text-[var(--c-text-muted)] hover:bg-[var(--c-bg3)]">
              Aucun
            </button>
          </div>
        </div>
        <div className="px-3 pb-2">
          <div className="flex items-center gap-2 rounded-md border border-[var(--c-border)] bg-[var(--c-bg2)] px-2 py-1.5">
            <IconSearch size={13} className="text-[var(--c-text-faint)]" />
            <input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filtrer (nom, tag, groupe…)"
              className="w-full bg-transparent text-xs text-[var(--c-text)] placeholder:text-[var(--c-text-faint)] focus:outline-none"
            />
          </div>
        </div>
        {sshHosts.length > 0 && (
          <div className="mb-1 space-y-1.5 px-3 pb-1">
            <button
              onClick={collectFacts}
              disabled={collectingFacts}
              className="flex w-full items-center justify-center gap-1.5 rounded-md border border-[var(--c-border)] bg-[var(--c-bg2)] px-2 py-1.5 text-xs text-[var(--c-text-secondary)] hover:bg-[var(--c-bg3)] disabled:opacity-50"
            >
              <IconRefresh size={12} className={collectingFacts ? "animate-spin" : ""} />
              {collectingFacts ? "Collecte de l'état…" : "Collecter l'état (OS, RAM)"}
            </button>
            {hasFacts && (
              <div className="flex items-center gap-1.5 text-[11px] text-[var(--c-text-muted)]">
                <span>RAM utilisée &gt;</span>
                <input
                  type="number"
                  min={0}
                  max={100}
                  value={ramThreshold}
                  onChange={(e) => setRamThreshold(Number(e.target.value))}
                  className="w-12 rounded border border-[var(--c-border)] bg-[var(--c-bg2)] px-1 py-0.5 text-center text-[var(--c-text)] focus:border-[var(--c-accent)] focus:outline-none"
                />
                <span>%</span>
                <button
                  onClick={selectByRam}
                  className="ml-auto rounded bg-[var(--c-accent-dim)] px-2 py-0.5 text-[var(--c-accent-text)] hover:bg-[var(--c-accent)] hover:text-white"
                >
                  Sélectionner
                </button>
              </div>
            )}
          </div>
        )}
        <div className="min-h-0 flex-1 overflow-y-auto px-1.5 pb-2">
          {sshHosts.length === 0 ? (
            <p className="px-2 py-4 text-xs text-[var(--c-text-faint)]">Aucun hôte SSH. La flotte ne cible que les hôtes SSH pour l'instant.</p>
          ) : (
            filtered.map((h) => {
              const checked = selected.has(h.id);
              const sub = [groupName(h), h.address].filter(Boolean).join(" · ");
              const f = facts.get(h.id);
              return (
                <label
                  key={h.id}
                  className={`flex cursor-pointer items-center gap-2.5 rounded-md px-2 py-1.5 ${checked ? "bg-[var(--c-accent-dim)]" : "hover:bg-[var(--c-bg3)]"}`}
                >
                  <input type="checkbox" checked={checked} onChange={() => toggle(h.id)} className="accent-[var(--c-accent)]" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm text-[var(--c-text)]">{h.label}</span>
                    {sub && <span className="block truncate text-[11px] text-[var(--c-text-faint)]">{sub}</span>}
                    {f && (
                      <span className="mt-0.5 flex items-center gap-2 text-[11px]">
                        {(f.osName || f.osId) && (
                          <span className="truncate text-[var(--c-text-muted)]">{f.osName || f.osId}</span>
                        )}
                        {f.memUsedPct != null && (
                          <span className="shrink-0 font-medium" style={{ color: ramColor(f.memUsedPct) }}>
                            RAM {Math.round(f.memUsedPct)}%
                          </span>
                        )}
                      </span>
                    )}
                  </span>
                </label>
              );
            })
          )}
        </div>
      </aside>

      {/* ── Command + results ─────────────────────────────────────────── */}
      <section className="flex min-w-0 flex-1 flex-col">
        <div className="border-b border-[var(--c-border)] p-3">
          <div className="flex items-end gap-2">
            <textarea
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  run();
                }
              }}
              rows={2}
              placeholder="Commande à exécuter sur les hôtes sélectionnés…  (Ctrl+Entrée)"
              spellCheck={false}
              className="min-h-[2.5rem] flex-1 resize-y rounded-md border border-[var(--c-border)] bg-[var(--c-bg2)] px-3 py-2 font-mono text-sm text-[var(--c-text)] placeholder:text-[var(--c-text-faint)] focus:border-[var(--c-accent)] focus:outline-none"
            />
            <button
              onClick={run}
              disabled={running || selected.size === 0 || !command.trim()}
              className="flex items-center gap-1.5 rounded-md bg-[var(--c-accent)] px-4 py-2 text-sm font-medium text-white hover:bg-[var(--c-accent-hover)] disabled:cursor-not-allowed disabled:opacity-40"
            >
              <IconPlay size={14} />
              {running ? "En cours…" : `Exécuter (${selected.size})`}
            </button>
          </div>
          {runTargets.length > 0 && (
            <div className="mt-2 flex items-center gap-3 text-xs">
              <span className="text-[#22c55e]">✓ {summary.ok} ok</span>
              <span className="text-[#ef4444]">✕ {summary.fail} échec</span>
              {summary.pending > 0 && <span className="text-[var(--c-text-muted)]">◷ {summary.pending} en cours</span>}
              <span className="text-[var(--c-text-faint)]">· {summary.total} hôte(s)</span>
            </div>
          )}
        </div>

        {/* Résultats / Historique */}
        <div className="flex items-center gap-1 border-b border-[var(--c-border)] px-3 py-1.5 text-xs">
          <button
            onClick={() => setView("run")}
            className={`rounded px-2 py-1 ${view === "run" ? "bg-[var(--c-bg3)] text-[var(--c-text)]" : "text-[var(--c-text-muted)] hover:bg-[var(--c-bg2)]"}`}
          >
            Résultats
          </button>
          <button
            onClick={() => setView("history")}
            className={`rounded px-2 py-1 ${view === "history" ? "bg-[var(--c-bg3)] text-[var(--c-text)]" : "text-[var(--c-text-muted)] hover:bg-[var(--c-bg2)]"}`}
          >
            Historique ({history.length})
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {view === "history" ? (
            history.length === 0 ? (
              <div className="flex h-full items-center justify-center px-6 text-center text-sm text-[var(--c-text-faint)]">
                Aucun run enregistré. Les exécutions passées apparaîtront ici.
              </div>
            ) : (
              <ul>
                {history.map((hrun) => {
                  const counts = countOutcomes(hrun.outcomes);
                  const isOpen = expandedRuns.has(hrun.id);
                  return (
                    <li key={hrun.id} className="border-b border-[var(--c-border)]">
                      <div
                        onClick={() => toggleRun(hrun.id)}
                        className="flex cursor-pointer items-center gap-2 px-3 py-2 hover:bg-[var(--c-bg2)]"
                      >
                        {isOpen ? (
                          <IconChevronDown size={12} className="shrink-0 text-[var(--c-text-faint)]" />
                        ) : (
                          <IconChevronRight size={12} className="shrink-0 text-[var(--c-text-faint)]" />
                        )}
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-mono text-xs text-[var(--c-text)]">{hrun.command}</div>
                          <div className="text-[11px] text-[var(--c-text-faint)]">
                            {formatTimestamp(hrun.startedAtMs)} · {hrun.hostIds.length} hôte(s)
                          </div>
                        </div>
                        <span className="shrink-0 text-xs text-[#22c55e]">✓{counts.ok}</span>
                        <span className="shrink-0 text-xs text-[#ef4444]">✕{counts.fail}</span>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            loadRun(hrun);
                          }}
                          className="shrink-0 rounded bg-[var(--c-accent-dim)] px-2 py-0.5 text-[11px] text-[var(--c-accent-text)] hover:bg-[var(--c-accent)] hover:text-white"
                        >
                          Charger
                        </button>
                      </div>
                      {isOpen && (
                        <div className="space-y-1 px-3 pb-2 pl-7">
                          {hrun.outcomes.map((o) => (
                            <div key={o.hostId} className="text-xs">
                              <div className="flex items-center gap-2">
                                <StatusDot status={outcomeStatus(o)} />
                                <span className="flex-1 truncate text-[var(--c-text-secondary)]">
                                  {hostById.get(o.hostId)?.label ?? o.hostId}
                                </span>
                                <span className="shrink-0 font-mono text-[var(--c-text-faint)]">
                                  {o.error != null ? "—" : o.exitCode ?? "—"} · {o.durationMs} ms
                                </span>
                              </div>
                              {(o.stdout || o.stderr || o.error) && (
                                <details className="ml-5 mt-0.5">
                                  <summary className="cursor-pointer text-[11px] text-[var(--c-text-muted)]">sortie</summary>
                                  {o.error != null && <p className="mt-1 text-[11px] text-[#ef4444]">{o.error}</p>}
                                  {o.stdout && (
                                    <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded bg-[var(--c-bg2)] p-2 font-mono text-[11px] text-[var(--c-text-secondary)]">{o.stdout}</pre>
                                  )}
                                  {o.stderr && (
                                    <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded bg-[var(--c-bg2)] p-2 font-mono text-[11px] text-[#fca5a5]">{o.stderr}</pre>
                                  )}
                                </details>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            )
          ) : runTargets.length === 0 ? (
            <div className="flex h-full items-center justify-center px-6 text-center text-sm text-[var(--c-text-faint)]">
              Sélectionne des hôtes, saisis une commande, puis exécute — le résultat de chaque hôte s'affiche ici.
            </div>
          ) : (
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="sticky top-0 bg-[var(--c-bg2)] text-left text-[11px] uppercase tracking-wide text-[var(--c-text-muted)]">
                  <th className="w-8 py-2 pl-3"></th>
                  <th className="py-2">Hôte</th>
                  <th className="w-16 py-2 text-center">Code</th>
                  <th className="w-20 py-2 pr-3 text-right">Durée</th>
                </tr>
              </thead>
              <tbody>
                {runTargets.map((id) => {
                  const host = hostById.get(id);
                  const outcome = results.get(id);
                  const status = statusOf(id, results, pending);
                  const isOpen = expanded.has(id);
                  const hasDetail = !!outcome && (!!outcome.stdout || !!outcome.stderr || !!outcome.error);
                  return (
                    <Fragment key={id}>
                      <tr
                        onClick={() => hasDetail && toggleExpanded(id)}
                        className={`border-b border-[var(--c-border)] ${hasDetail ? "cursor-pointer hover:bg-[var(--c-bg2)]" : ""}`}
                      >
                        <td className="py-2 pl-3">
                          <div className="flex items-center gap-1">
                            {hasDetail ? (
                              isOpen ? <IconChevronDown size={12} className="text-[var(--c-text-faint)]" /> : <IconChevronRight size={12} className="text-[var(--c-text-faint)]" />
                            ) : (
                              <span className="w-3" />
                            )}
                            <StatusDot status={status} />
                          </div>
                        </td>
                        <td className="py-2 text-[var(--c-text)]">{host?.label ?? id}</td>
                        <td className="py-2 text-center font-mono text-xs">
                          {outcome ? (outcome.error != null ? "—" : outcome.exitCode ?? "—") : ""}
                        </td>
                        <td className="py-2 pr-3 text-right font-mono text-xs text-[var(--c-text-muted)]">
                          {outcome ? `${outcome.durationMs} ms` : ""}
                        </td>
                      </tr>
                      {isOpen && outcome && (
                        <tr className="border-b border-[var(--c-border)] bg-[var(--c-bg)]">
                          <td colSpan={4} className="px-4 py-2">
                            {outcome.error != null && (
                              <p className="mb-2 text-xs text-[#ef4444]">{outcome.error}</p>
                            )}
                            {outcome.stdout && (
                              <pre className="mb-2 max-h-64 overflow-auto whitespace-pre-wrap rounded bg-[var(--c-bg2)] p-2 font-mono text-xs text-[var(--c-text-secondary)]">{outcome.stdout}</pre>
                            )}
                            {outcome.stderr && (
                              <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded bg-[var(--c-bg2)] p-2 font-mono text-xs text-[#fca5a5]">{outcome.stderr}</pre>
                            )}
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </section>
    </div>
  );
}
