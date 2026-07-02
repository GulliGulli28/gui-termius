import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AuthMethod, EnvVar, Entry, GroupId, HostId, KeyId, PaneListed, PaneOpened, PaneSource, PortForwardId, PortForwardKind, SnippetId, Workspace } from "./types";

export const api = {
  getWorkspace: () => invoke<Workspace>("get_workspace"),

  saveHost: (input: {
    id: HostId | null;
    label: string;
    address: string;
    port: number;
    username: string;
    auth: AuthMethod;
    jumpVia: HostId[];
    groupId: GroupId | null;
    tags: string[];
    startupSnippets: SnippetId[];
    envVars: EnvVar[];
    icon: string | null;
    secret: string | null;
  }) => invoke<Workspace>("save_host", { input }),

  deleteHost: (hostId: HostId) => invoke<Workspace>("delete_host", { hostId }),

  saveGroup: (input: { id: GroupId | null; name: string; parentId: GroupId | null; icon: string | null }) => invoke<Workspace>("save_group", { input }),
  deleteGroup: (groupId: GroupId) => invoke<Workspace>("delete_group", { groupId }),

  addSnippet: (name: string, command: string) => invoke<Workspace>("add_snippet", { name, command }),
  deleteSnippet: (snippetId: SnippetId) => invoke<Workspace>("delete_snippet", { snippetId }),

  addForward: (input: { hostId: HostId; kind: PortForwardKind; bindAddress: string; bindPort: number; destAddress: string; destPort: number }) =>
    invoke<Workspace>("add_forward", { input }),
  deleteForward: (forwardId: PortForwardId) => invoke<Workspace>("delete_forward", { forwardId }),

  addPrivateKey: (name: string, path: string, passphrase: string | null) => invoke<Workspace>("add_private_key", { name, path, passphrase }),
  deletePrivateKey: (keyId: KeyId) => invoke<Workspace>("delete_private_key", { keyId }),
  renamePrivateKey: (keyId: KeyId, name: string) => invoke<Workspace>("rename_private_key", { keyId, name }),
  addCustomIcon: (name: string, dataUrl: string) => invoke<Workspace>("add_custom_icon", { name, dataUrl }),
  deleteCustomIcon: (iconId: string) => invoke<Workspace>("delete_custom_icon", { iconId }),
  readIconFile: (path: string) => invoke<string>("read_icon_file", { path }),

  exportWorkspace: (path: string) => invoke<void>("export_workspace", { path }),
  importWorkspace: (path: string, replace: boolean) => invoke<Workspace>("import_workspace", { path, replace }),
  exportHost: (hostId: HostId, path: string) => invoke<void>("export_host", { hostId, path }),
  importHostFromFile: (path: string) => invoke<Workspace>("import_host_from_file", { path }),
  startForward: (forwardId: PortForwardId) => invoke<void>("start_forward", { forwardId }),
  stopForward: (forwardId: PortForwardId) => invoke<void>("stop_forward", { forwardId }),
  runningForwards: () => invoke<PortForwardId[]>("running_forwards"),

  connectTerminal: (hostId: HostId) => invoke<string>("connect_terminal", { hostId }),
  writeTerminal: (sessionId: string, data: string) => invoke<void>("write_terminal", { sessionId, data }),
  resizeTerminal: (sessionId: string, cols: number, rows: number) => invoke<void>("resize_terminal", { sessionId, cols, rows }),
  closeTerminal: (sessionId: string) => invoke<void>("close_terminal", { sessionId }),

  openLocalTerminal: () => invoke<string>("open_local_terminal"),
  writeLocalTerminal: (sessionId: string, data: string) => invoke<void>("write_local_terminal", { sessionId, data }),
  resizeLocalTerminal: (sessionId: string, cols: number, rows: number) => invoke<void>("resize_local_terminal", { sessionId, cols, rows }),
  closeLocalTerminal: (sessionId: string) => invoke<void>("close_local_terminal", { sessionId }),

  openPane: (source: PaneSource) => invoke<PaneOpened>("open_pane", { source }),
  closePane: (paneId: string) => invoke<void>("close_pane", { paneId }),
  listPane: (paneId: string, path: string) => invoke<PaneListed>("list_pane", { paneId, path }),
  copyEntry: (sourcePaneId: string, sourceCwd: string, entry: Entry, destPaneId: string, destCwd: string) =>
    invoke<PaneListed>("copy_entry", { sourcePaneId, sourceCwd, entry, destPaneId, destCwd }),
};

export function onTerminalData(handler: (id: string, data: string) => void): Promise<UnlistenFn> {
  return listen<{ id: string; data: string }>("terminal-data", (event) => handler(event.payload.id, event.payload.data));
}

export function onTerminalClosed(handler: (id: string) => void): Promise<UnlistenFn> {
  return listen<{ id: string }>("terminal-closed", (event) => handler(event.payload.id));
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

export function base64ToBytes(text: string): Uint8Array {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}
