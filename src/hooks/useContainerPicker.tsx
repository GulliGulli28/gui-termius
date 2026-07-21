import { useState } from "react";
import { api } from "../lib/api";
import type { DockerContainer, Host, K8sPod } from "../lib/types";
import { parsePodPickerId, podPickerId } from "../lib/types";
import { ConnectionPickerModal } from "../components/ConnectionPickerModal";

export interface UseContainerPickerResult {
  /** The host a Docker/K8s picker is currently open for, if any — exposed
   * (not just hidden inside `pickerModal`) because some callers need it for
   * more than rendering the modal itself (e.g. `TransferTab.tsx` keeps its
   * source dropdown showing the host being picked into, not the
   * still-unchanged prior selection, until something's actually picked). */
  dockerPickerHost: Host | null;
  k8sPickerHost: Host | null;
  openDockerPicker: (host: Host) => void;
  openK8sPicker: (host: Host) => void;
  /** Render this inline wherever the picker modal(s) should appear — renders
   * nothing when neither picker is open. */
  pickerModal: React.ReactNode;
}

/** Docker container / Kubernetes pod picker: opens on a menu click, fetches
 * the live list, and lets the caller decide what "picked" means
 * (`onPickDocker`/`onPickK8s` — connect a terminal, open a transfer pane,
 * whatever this call site needs). Previously three near-identical copies of
 * the same state+fetch+modal JSX (`HostsPanel`/`TransferTab`/`SftpPanel`).
 *
 * `SplitPane.tsx` isn't a fourth copy of this: it fetches eagerly as soon as
 * its single `source` changes rather than on an explicit "open" trigger, and
 * has no separate close action (reverting to the local terminal instead) —
 * different enough in shape/lifecycle that folding it in here would cost
 * more than it'd save, so it's left with its own inline state. */
export function useContainerPicker(
  onPickDocker: (host: Host, containerId: string) => void,
  onPickK8s: (host: Host, podName: string, containerName: string | null) => void,
): UseContainerPickerResult {
  const [dockerPickerHost, setDockerPickerHost] = useState<Host | null>(null);
  const [dockerContainers, setDockerContainers] = useState<DockerContainer[] | null>(null);
  const [dockerPickerError, setDockerPickerError] = useState<string | null>(null);
  const [k8sPickerHost, setK8sPickerHost] = useState<Host | null>(null);
  const [k8sPods, setK8sPods] = useState<K8sPod[] | null>(null);
  const [k8sPickerError, setK8sPickerError] = useState<string | null>(null);

  const openDockerPicker = (host: Host) => {
    setDockerPickerHost(host);
    setDockerContainers(null);
    setDockerPickerError(null);
    api.listDockerContainers(host.id).then(setDockerContainers).catch((e) => setDockerPickerError(String(e)));
  };

  const openK8sPicker = (host: Host) => {
    setK8sPickerHost(host);
    setK8sPods(null);
    setK8sPickerError(null);
    api.listK8sPods(host.id).then(setK8sPods).catch((e) => setK8sPickerError(String(e)));
  };

  const pickerModal = (
    <>
      {dockerPickerHost && (
        <ConnectionPickerModal
          title={`Conteneurs Docker — ${dockerPickerHost.label}`}
          loading={dockerContainers === null && !dockerPickerError}
          error={dockerPickerError}
          items={(dockerContainers ?? []).map((c) => ({ id: c.id, name: c.name || c.id.slice(0, 12), meta: `${c.image} · ${c.status}`, up: c.state === "running" }))}
          onPick={(containerId) => { onPickDocker(dockerPickerHost, containerId); setDockerPickerHost(null); }}
          onClose={() => setDockerPickerHost(null)}
        />
      )}
      {k8sPickerHost && (
        <ConnectionPickerModal
          title={`Pods Kubernetes — ${k8sPickerHost.label}`}
          loading={k8sPods === null && !k8sPickerError}
          error={k8sPickerError}
          items={(k8sPods ?? []).flatMap((pod) =>
            pod.containers.length > 1
              ? pod.containers.map((c) => ({
                  id: podPickerId(pod.name, c),
                  name: `${pod.name} › ${c}`,
                  meta: `${pod.namespace} · ${pod.phase}`,
                  up: pod.ready,
                }))
              : [{ id: podPickerId(pod.name), name: pod.name, meta: `${pod.namespace} · ${pod.phase}`, up: pod.ready }],
          )}
          onPick={(id) => {
            const { podName, containerName } = parsePodPickerId(id);
            onPickK8s(k8sPickerHost, podName, containerName);
            setK8sPickerHost(null);
          }}
          onClose={() => setK8sPickerHost(null)}
        />
      )}
    </>
  );

  return { dockerPickerHost, k8sPickerHost, openDockerPicker, openK8sPicker, pickerModal };
}
