import type { ComponentType } from "react";
import type { HostKind } from "./types";
import { IconTerminal, IconDocker, IconKubernetes, IconMonitor } from "../components/ui-icons";

interface HostKindMeta {
  key: HostKind;
  label: string;
  Icon: ComponentType<{ size?: number; className?: string }>;
}

export const HOST_KINDS: HostKindMeta[] = [
  { key: "ssh", label: "SSH", Icon: IconTerminal },
  { key: "dockerExec", label: "Docker exec", Icon: IconDocker },
  { key: "k8sExec", label: "Kubernetes exec", Icon: IconKubernetes },
  { key: "rdp", label: "RDP", Icon: IconMonitor },
];

export function hostKindMeta(kind: HostKind | undefined): HostKindMeta {
  return HOST_KINDS.find((k) => k.key === (kind ?? "ssh")) ?? HOST_KINDS[0];
}
