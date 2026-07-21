import { useState } from "react";
import type { SqlConnection, SqlConnectionId, SqlEngine, Workspace } from "../lib/types";
import { IconTrash } from "./ui-icons";

export interface SqlConnectionFormData {
  id: SqlConnectionId | null;
  label: string;
  engine: SqlEngine;
  tunnelHostId: string | null;
  address: string;
  port: number;
  username: string;
  database: string | null;
  groupId: null;
  tags: string[];
  secret: string | null;
}

interface SqlConnectionFormProps {
  workspace: Workspace;
  /** `null` — a new connection. */
  connection: SqlConnection | null;
  onCancel: () => void;
  onSave: (input: SqlConnectionFormData) => void;
  onDeleteConnection?: (id: SqlConnectionId) => void;
}

const DEFAULT_PORTS: Record<SqlEngine, string> = { mysql: "3306", postgres: "5432" };

/** Right-panel form for creating/editing a SQL connection — same slot and
 * layout as `HostForm`/`GroupForm` (see `App.tsx`'s `showRightPanel`), rather
 * than an inline expansion in `SqlConnectionsPanel`'s list. */
export function SqlConnectionForm({ workspace, connection, onCancel, onSave, onDeleteConnection }: SqlConnectionFormProps) {
  const [label, setLabel] = useState(connection?.label ?? "");
  const [engine, setEngine] = useState<SqlEngine>(connection?.engine ?? "mysql");
  const [tunnelHostId, setTunnelHostId] = useState(connection?.tunnelHostId ?? "");
  const [address, setAddress] = useState(connection?.address ?? "");
  const [port, setPort] = useState(String(connection?.port ?? DEFAULT_PORTS.mysql));
  const [username, setUsername] = useState(connection?.username ?? "");
  const [password, setPassword] = useState("");
  const [database, setDatabase] = useState(connection?.database ?? "");
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Only switches the port if it's still at one engine's default — a custom
  // port the user already typed in is left untouched.
  const onEngineChange = (next: SqlEngine) => {
    setEngine(next);
    if (Object.values(DEFAULT_PORTS).includes(port)) setPort(DEFAULT_PORTS[next]);
  };

  const submit = () => {
    const p = Number(port);
    if (!label.trim() || !address.trim() || !username.trim() || !Number.isInteger(p) || p < 1 || p > 65535) {
      setError("Champs de connexion SQL invalides");
      return;
    }
    onSave({
      id: connection?.id ?? null,
      label: label.trim(),
      engine,
      tunnelHostId: tunnelHostId || null,
      address: address.trim(),
      port: p,
      username: username.trim(),
      database: database.trim() || null,
      groupId: null,
      tags: [],
      secret: password || null,
    });
  };

  return (
    <div className="flex flex-1 flex-col overflow-y-auto p-4">
      <div className="w-full space-y-4 rounded-xl bg-[var(--c-bg2)] p-5 shadow-[var(--shadow-md)]">
        <h2 className="text-[16px] font-semibold text-[var(--c-text)]">
          {connection ? "Modifier la connexion SQL" : "Nouvelle connexion SQL"}
        </h2>

        {error && <p className="rounded-md bg-rose-950 px-3 py-2 text-sm text-rose-300">{error}</p>}

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Nom</span>
          <input value={label} onChange={(e) => setLabel(e.target.value)} placeholder="Nom" autoFocus className={inputFullClass} />
        </div>

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Moteur</span>
          <select value={engine} onChange={(e) => onEngineChange(e.target.value as SqlEngine)} className={selectClass}>
            <option value="mysql">MySQL</option>
            <option value="postgres">PostgreSQL</option>
          </select>
        </div>

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Tunnel</span>
          <select value={tunnelHostId} onChange={(e) => setTunnelHostId(e.target.value)} className={selectClass}>
            <option value="">Connexion directe (pas de tunnel)</option>
            {workspace.hosts
              .filter((h) => (h.kind ?? "ssh") === "ssh")
              .map((h) => (
                <option key={h.id} value={h.id}>Tunnel SSH via {h.label}</option>
              ))}
          </select>
          {tunnelHostId && (
            <p className="px-0.5 text-[11px] leading-relaxed text-[var(--c-text-muted)]">
              L'adresse ci-dessous doit être joignable <em>depuis</em> cet hôte — souvent
              127.0.0.1 si la base n'écoute qu'en local sur le serveur.
            </p>
          )}
        </div>

        <div className="flex gap-1.5">
          <div className="min-w-0 flex-1 space-y-1">
            <span className="text-xs font-medium text-[var(--c-text-secondary)]">Adresse</span>
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="Adresse" className={`${inputClass} w-full font-mono`} />
          </div>
          <div className="w-20 shrink-0 space-y-1">
            <span className="text-xs font-medium text-[var(--c-text-secondary)]">Port</span>
            <input value={port} onChange={(e) => setPort(e.target.value)} placeholder="Port" inputMode="numeric" className={`${inputClass} w-full font-mono`} />
          </div>
        </div>

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Utilisateur</span>
          <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Utilisateur" className={inputFullClass} />
        </div>

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Mot de passe</span>
          <input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            placeholder={connection ? "Laisser vide pour ne pas changer" : "Mot de passe"}
            className={inputFullClass}
          />
        </div>

        <div className="space-y-1">
          <span className="text-xs font-medium text-[var(--c-text-secondary)]">Base de données (optionnel)</span>
          <input value={database} onChange={(e) => setDatabase(e.target.value)} placeholder="Base de données (optionnel)" className={inputFullClass} />
          {engine === "postgres" && !database.trim() && (
            <p className="px-0.5 text-[11px] leading-relaxed text-[var(--c-text-muted)]">
              Laissé vide : la connexion listera toutes les bases du serveur au lieu d'une seule.
            </p>
          )}
        </div>

        <div className="flex gap-2 pt-2">
          <button onClick={submit} className="flex-1 rounded-md bg-[var(--c-accent)] px-3 py-2 text-sm font-medium text-white hover:bg-[var(--c-accent-hover)]">
            {connection ? "Enregistrer" : "Ajouter"}
          </button>
          <button onClick={onCancel} className="flex-1 rounded-md bg-[var(--c-bg3)] px-3 py-2 text-sm font-medium text-[var(--c-text)] hover:bg-white/5">
            Annuler
          </button>
        </div>

        {connection && onDeleteConnection && (
          <div className="border-t border-[var(--c-border)] pt-3">
            {confirmDelete ? (
              <div className="space-y-2 rounded-lg bg-rose-950/30 p-3">
                <p className="text-sm text-rose-300">Supprimer cette connexion définitivement ?</p>
                <div className="flex gap-2">
                  <button
                    onClick={() => onDeleteConnection(connection.id)}
                    className="flex-1 rounded-md bg-rose-700 px-3 py-2 text-sm font-medium text-white hover:bg-rose-600"
                  >
                    Oui, supprimer
                  </button>
                  <button onClick={() => setConfirmDelete(false)} className="flex-1 rounded-md bg-[var(--c-bg3)] px-3 py-2 text-sm font-medium text-[var(--c-text)] hover:bg-white/5">
                    Annuler
                  </button>
                </div>
              </div>
            ) : (
              <button
                onClick={() => setConfirmDelete(true)}
                className="flex w-full items-center justify-center gap-2 rounded-md py-2 text-sm text-rose-400 hover:bg-rose-950/40 hover:text-rose-300"
              >
                <IconTrash size={13} /> Supprimer cette connexion
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

const inputClass = "rounded-md bg-[var(--c-bg3)] px-3 py-2 text-sm text-[var(--c-text)] placeholder:text-[var(--c-text-muted)] focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]";
const inputFullClass = `${inputClass} w-full`;
const selectClass = "w-full rounded-md bg-[var(--c-bg3)] px-3 py-2 text-sm text-[var(--c-text)] focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]";
