import { useEffect, useState } from "react";
import type { Host, HostId } from "../lib/types";

/** Polls `fetchOne(host)` once immediately and then every 30s, for every
 * host matching `filter`, keyed by host id — the "is this SSH host
 * reachable" / "how many Docker containers are running" / "how many K8s
 * pods are ready" indicators shown inline in `HostsPanel`'s host list,
 * previously three near-identical copies of the same interval+cleanup
 * dance. Best-effort: a fetch that throws sets that host's value to
 * `onError` rather than failing the whole panel.
 *
 * `filter`/`fetchOne` are read via closure, not tracked as effect
 * dependencies (only the host id list is) — same intentional omission the
 * three original copies already had: they only ever close over stable
 * things (`host.kind`, the `api` singleton), so a fresh closure identity
 * each render never changes what polling actually does. */
export function usePolledHostStat<T>(
  hosts: Host[],
  filter: (host: Host) => boolean,
  fetchOne: (host: Host) => Promise<T>,
  onError: T,
): Record<HostId, T> {
  const [values, setValues] = useState<Record<HostId, T>>({});
  const hostIdsKey = hosts.map((h) => h.id).join(",");

  useEffect(() => {
    let cancelled = false;
    const targets = hosts.filter(filter);
    const poll = () => {
      for (const host of targets) {
        fetchOne(host)
          .then((value) => { if (!cancelled) setValues((prev) => ({ ...prev, [host.id]: value })); })
          .catch(() => { if (!cancelled) setValues((prev) => ({ ...prev, [host.id]: onError })); });
      }
    };
    poll();
    const interval = setInterval(poll, 30000);
    return () => { cancelled = true; clearInterval(interval); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hostIdsKey]);

  return values;
}
