import { useState } from "react";
import type { ComponentType } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { Workspace } from "../lib/types";
import type { AppPreferences } from "../lib/preferences";
import { TERMINAL_THEMES, FONT_FAMILIES, ACCENT_COLORS, BG_THEMES, type UiAccent, type UiBg } from "../lib/preferences";
import { SHORTCUT_ACTIONS, defaultShortcuts, comboFromEvent } from "../lib/shortcuts";
import { IconUpload, IconDownload, IconPalette, IconTerminal, IconTransfer, IconKeyboard, IconBell, IconSettings } from "./ui-icons";

interface SettingsPanelProps {
  workspace: Workspace;
  onWorkspaceUpdate: (ws: Workspace) => void;
  onError: (msg: string) => void;
  preferences: AppPreferences;
  onPreferencesChange: (p: AppPreferences) => void;
}

type ImportPending = { path: string };

type SettingsCategory = "apparence" | "terminal" | "sftp" | "raccourcis" | "notifications" | "general";

const CATEGORIES: { key: SettingsCategory; label: string; Icon: ComponentType<{ size?: number }> }[] = [
  { key: "apparence", label: "Apparence", Icon: IconPalette },
  { key: "terminal", label: "Terminal", Icon: IconTerminal },
  { key: "sftp", label: "SFTP", Icon: IconTransfer },
  { key: "raccourcis", label: "Raccourcis", Icon: IconKeyboard },
  { key: "notifications", label: "Notifications", Icon: IconBell },
  { key: "general", label: "Général", Icon: IconSettings },
];

function ShortcutRow({ label, combo, onChange }: { label: string; combo: string; onChange: (combo: string) => void }) {
  const [capturing, setCapturing] = useState(false);
  return (
    <div className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 hover:bg-slate-800/40">
      <span className="text-xs text-slate-300">{label}</span>
      <button
        type="button"
        onClick={() => setCapturing(true)}
        onBlur={() => setCapturing(false)}
        onKeyDown={(e) => {
          if (!capturing) return;
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "Escape") { setCapturing(false); return; }
          if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
          onChange(comboFromEvent(e));
          setCapturing(false);
        }}
        className={`shrink-0 rounded-md border px-2 py-1 font-mono text-[11px] ${
          capturing
            ? "border-[var(--c-accent)] bg-[var(--c-accent-dim)] text-[var(--c-accent-text)]"
            : "border-slate-700 bg-slate-800 text-slate-300 hover:border-slate-600"
        }`}
      >
        {capturing ? "Appuyez sur une touche…" : combo || "—"}
      </button>
    </div>
  );
}

function ToggleRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 hover:bg-slate-800/40">
      <span className="text-xs text-slate-300">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="h-4 w-4 shrink-0 accent-[var(--c-accent)]"
      />
    </label>
  );
}

export function SettingsPanel({ onWorkspaceUpdate, onError, preferences, onPreferencesChange }: SettingsPanelProps) {
  const [category, setCategory] = useState<SettingsCategory>("apparence");
  const [importPending, setImportPending] = useState<ImportPending | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const fileFilters = [{ name: "JSON", extensions: ["json"] }];

  const flash = (msg: string) => {
    setDone(msg);
    setTimeout(() => setDone(null), 3000);
  };

  const handleExportWorkspace = async () => {
    try {
      const path = await save({
        title: "Exporter la configuration",
        defaultPath: "termius-config.json",
        filters: fileFilters,
      });
      if (path) {
        await api.exportWorkspace(path);
        flash("Configuration exportée ✓");
      }
    } catch (e) { onError(String(e)); }
  };

  const handleImportWorkspaceFile = async () => {
    try {
      const path = await open({ title: "Importer une configuration", multiple: false, filters: fileFilters });
      if (path && typeof path === "string") setImportPending({ path });
    } catch (e) { onError(String(e)); }
  };

  const confirmImport = async (replace: boolean) => {
    if (!importPending) return;
    try {
      const ws = await api.importWorkspace(importPending.path, replace);
      onWorkspaceUpdate(ws);
      flash(replace ? "Configuration remplacée ✓" : "Configuration fusionnée ✓");
    } catch (e) { onError(String(e)); }
    setImportPending(null);
  };

  const setShortcut = (id: string, combo: string) => {
    onPreferencesChange({ ...preferences, keyboardShortcuts: { ...preferences.keyboardShortcuts, [id]: combo } });
  };

  return (
    <div className="flex h-full min-w-0">
      {/* Category rail */}
      <nav className="flex w-12 shrink-0 flex-col items-center gap-1 border-r border-[var(--c-border)] py-2">
        {CATEGORIES.map((c) => {
          const active = category === c.key;
          return (
            <button
              key={c.key}
              onClick={() => setCategory(c.key)}
              title={c.label}
              className={`relative flex h-10 w-10 items-center justify-center rounded-lg transition-colors ${
                active ? "bg-[var(--c-accent-dim)] text-[var(--c-accent-text)]" : "text-slate-500 hover:bg-slate-800/70 hover:text-slate-300"
              }`}
            >
              {active && <span className="absolute right-0 top-1.5 bottom-1.5 w-[2px] rounded-full bg-[var(--c-accent)]" />}
              <c.Icon size={19} />
            </button>
          );
        })}
      </nav>

      {/* Category content */}
      <div className="sidebar-scroll min-w-0 flex-1 space-y-4 overflow-y-auto p-2">
        <p className="text-[10px] font-semibold uppercase tracking-wider text-slate-500">
          {CATEGORIES.find((c) => c.key === category)?.label}
        </p>

        {done && (
          <div className="rounded-md bg-emerald-900/60 px-3 py-2 text-xs text-emerald-200">{done}</div>
        )}

        {category === "apparence" && (
          <>
            <section className="space-y-2">
              <p className="text-xs font-medium text-slate-400">Fond de l'interface</p>
              <div className="space-y-2 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
                <div className="flex flex-wrap gap-2">
                  {(Object.entries(BG_THEMES) as [UiBg, typeof BG_THEMES[UiBg]][]).map(([key, bg]) => {
                    const active = (preferences.uiBg ?? "slate") === key;
                    return (
                      <button
                        key={key}
                        type="button"
                        title={bg.label}
                        onClick={() => onPreferencesChange({ ...preferences, uiBg: key })}
                        className={`flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs transition-all ${active ? "ring-2 ring-white ring-offset-1 ring-offset-slate-800" : "opacity-70 hover:opacity-100"}`}
                        style={{ backgroundColor: bg.bg2, borderColor: bg.border, color: "#e2e8f0" }}
                      >
                        <span className="h-2.5 w-2.5 shrink-0 rounded-full" style={{ backgroundColor: bg.bg }} />
                        {bg.label}
                        {active && <span className="text-[var(--c-accent-text)]">✓</span>}
                      </button>
                    );
                  })}
                </div>
              </div>
            </section>

            <section className="space-y-2">
              <p className="text-xs font-medium text-slate-400">Couleur d'accent de l'interface</p>
              <div className="space-y-2 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
                <div className="flex flex-wrap gap-2">
                  {(Object.entries(ACCENT_COLORS) as [UiAccent, typeof ACCENT_COLORS[UiAccent]][]).map(([key, color]) => {
                    const active = (preferences.uiAccent ?? "indigo") === key;
                    return (
                      <button
                        key={key}
                        type="button"
                        title={color.label}
                        onClick={() => onPreferencesChange({ ...preferences, uiAccent: key })}
                        className={`flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs transition-all ${active ? "ring-2 ring-white ring-offset-1 ring-offset-slate-800" : "opacity-70 hover:opacity-100"}`}
                        style={{ backgroundColor: color.c600, color: "#fff" }}
                      >
                        {active && <span>✓</span>}
                        {color.label}
                      </button>
                    );
                  })}
                </div>
              </div>
            </section>
          </>
        )}

        {category === "terminal" && (
          <section className="space-y-3 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
            <div className="space-y-1">
              <label className="block text-[11px] text-slate-400">Thème</label>
              <select
                value={preferences.terminalThemeName}
                onChange={(e) => onPreferencesChange({ ...preferences, terminalThemeName: e.target.value })}
                className="w-full rounded-md bg-slate-800 px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
              >
                {Object.entries(TERMINAL_THEMES).map(([key, entry]) => (
                  <option key={key} value={key}>{entry.label}</option>
                ))}
              </select>
            </div>

            <div className="space-y-1">
              <label className="block text-[11px] text-slate-400">Police</label>
              <select
                value={preferences.terminalFontFamily}
                onChange={(e) => onPreferencesChange({ ...preferences, terminalFontFamily: e.target.value })}
                className="w-full rounded-md bg-slate-800 px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent-hover)]"
              >
                {FONT_FAMILIES.map((f) => (
                  <option key={f.value} value={f.value}>{f.label}</option>
                ))}
              </select>
            </div>

            <div className="space-y-1">
              <label className="block text-[11px] text-slate-400">
                Taille de police : <span className="text-slate-200">{preferences.terminalFontSize} px</span>
              </label>
              <input
                type="range"
                min={10}
                max={24}
                step={1}
                value={preferences.terminalFontSize}
                onChange={(e) => onPreferencesChange({ ...preferences, terminalFontSize: Number(e.target.value) })}
                className="w-full accent-indigo-500"
              />
              <div className="flex justify-between text-[10px] text-slate-600">
                <span>10</span><span>24</span>
              </div>
            </div>

            <div
              className="rounded-md p-2 font-mono text-xs"
              style={{
                backgroundColor: TERMINAL_THEMES[preferences.terminalThemeName]?.theme.background ?? "#020617",
                color: TERMINAL_THEMES[preferences.terminalThemeName]?.theme.foreground ?? "#e2e8f0",
                fontFamily: preferences.terminalFontFamily,
                fontSize: `${Math.min(preferences.terminalFontSize, 13)}px`,
              }}
            >
              user@server:~$ echo "Aperçu du thème"<br />
              <span style={{ color: TERMINAL_THEMES[preferences.terminalThemeName]?.theme.green }}>✓</span>
              {" "}Aperçu du thème
            </div>
          </section>
        )}

        {category === "sftp" && (
          <section className="space-y-3 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
            <div className="space-y-1">
              <label className="block text-[11px] text-slate-400">
                Taille du texte : <span className="text-slate-200">{preferences.sftpFontSize ?? 13} px</span>
              </label>
              <input
                type="range"
                min={11}
                max={18}
                step={1}
                value={preferences.sftpFontSize ?? 13}
                onChange={(e) => onPreferencesChange({ ...preferences, sftpFontSize: Number(e.target.value) })}
                className="w-full accent-indigo-500"
              />
              <div className="flex justify-between text-[10px] text-slate-600">
                <span>11</span><span>18</span>
              </div>
            </div>
            <div
              className="rounded-md border border-[var(--c-border)] bg-[var(--c-bg)] p-2"
              style={{ fontSize: `${preferences.sftpFontSize ?? 13}px` }}
            >
              <div className="flex items-center gap-2 text-slate-300">
                <span>📁</span><span className="flex-1 font-medium text-[var(--c-accent-text)]">documents</span>
                <span className="text-slate-500">Dossier</span>
              </div>
              <div className="mt-1 flex items-center gap-2 text-slate-300">
                <span>📄</span><span className="flex-1">rapport-2024.pdf</span>
                <span className="text-slate-500">PDF</span>
                <span className="text-slate-500">2.4 Mo</span>
              </div>
            </div>
          </section>
        )}

        {category === "raccourcis" && (
          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <p className="text-[11px] text-slate-500">Cliquez sur une combinaison pour la réaffecter.</p>
              <button
                onClick={() => onPreferencesChange({ ...preferences, keyboardShortcuts: defaultShortcuts() })}
                className="text-[11px] text-slate-500 hover:text-slate-300"
              >
                Réinitialiser
              </button>
            </div>
            <div className="rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-1.5">
              {SHORTCUT_ACTIONS.map((action) => (
                <ShortcutRow
                  key={action.id}
                  label={action.label}
                  combo={preferences.keyboardShortcuts[action.id] ?? ""}
                  onChange={(combo) => setShortcut(action.id, combo)}
                />
              ))}
            </div>
          </section>
        )}

        {category === "notifications" && (
          <section className="space-y-2 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-1.5">
            <ToggleRow
              label="Notifier à la perte de connexion"
              checked={preferences.notifyOnDisconnect}
              onChange={(v) => onPreferencesChange({ ...preferences, notifyOnDisconnect: v })}
            />
            <ToggleRow
              label="Notifier à la fin d'un transfert SFTP"
              checked={preferences.notifyOnTransferDone}
              onChange={(v) => onPreferencesChange({ ...preferences, notifyOnTransferDone: v })}
            />
          </section>
        )}

        {category === "general" && (
          <>
            <section className="space-y-2">
              <p className="text-xs font-medium text-slate-400">Session</p>
              <div className="space-y-1 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-1.5">
                <ToggleRow
                  label="Restaurer les onglets au démarrage"
                  checked={preferences.restoreTabsOnLaunch}
                  onChange={(v) => onPreferencesChange({ ...preferences, restoreTabsOnLaunch: v })}
                />
                <p className="px-2 pb-1 text-[11px] leading-relaxed text-slate-500">
                  Les onglets réapparaissent sans se reconnecter automatiquement — cliquez sur un onglet restauré pour vous reconnecter.
                </p>
              </div>
            </section>

            <section className="space-y-2">
              <p className="text-xs font-medium text-slate-400">Import / Export</p>
              <div className="space-y-2 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
                <p className="text-[11px] leading-relaxed text-slate-500">
                  Exporte toute la configuration (hôtes, dossiers, snippets, clés, icônes…) dans un fichier JSON.
                  Les mots de passe et passphrases ne sont jamais exportés.
                </p>
                <button
                  onClick={handleExportWorkspace}
                  className="flex w-full items-center justify-center gap-2 rounded-md bg-sky-700 px-3 py-2 text-xs font-medium text-white hover:bg-sky-600"
                >
                  <IconUpload size={13} /> Exporter la configuration
                </button>
              </div>

              <div className="space-y-2 rounded-lg border border-[var(--c-border)] bg-slate-800/30 p-3">
                <p className="text-[11px] leading-relaxed text-slate-500">
                  Importe une configuration depuis un fichier JSON.<br />
                  <strong className="text-slate-400">Fusionner</strong> ajoute et met à jour sans supprimer l'existant.{" "}
                  <strong className="text-slate-400">Remplacer</strong> écrase toute la configuration actuelle.
                </p>

                {!importPending ? (
                  <button
                    onClick={handleImportWorkspaceFile}
                    className="flex w-full items-center justify-center gap-2 rounded-md bg-slate-700 px-3 py-2 text-xs font-medium text-slate-200 hover:bg-slate-600"
                  >
                    <IconDownload size={13} /> Importer une configuration…
                  </button>
                ) : (
                  <div className="space-y-1.5">
                    <p className="text-xs text-sky-300">Choisissez le mode d'import :</p>
                    <div className="flex flex-wrap gap-1.5">
                      <button
                        onClick={() => confirmImport(false)}
                        className="flex-1 basis-[100px] rounded-md bg-sky-700 py-2 text-xs font-medium text-white hover:bg-sky-600"
                        title="Ajoute et met à jour sans supprimer l'existant"
                      >
                        Fusionner
                      </button>
                      <button
                        onClick={() => confirmImport(true)}
                        className="flex-1 basis-[100px] rounded-md bg-rose-700 py-2 text-xs font-medium text-white hover:bg-rose-600"
                        title="Remplace entièrement la configuration actuelle"
                      >
                        Remplacer
                      </button>
                      <button
                        onClick={() => setImportPending(null)}
                        className="shrink-0 rounded-md bg-slate-700 px-2.5 py-2 text-xs text-slate-300 hover:bg-slate-600"
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </section>
          </>
        )}
      </div>
    </div>
  );
}
