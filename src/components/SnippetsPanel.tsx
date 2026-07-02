import { useEffect, useRef, useState } from "react";
import type { SnippetId, Workspace } from "../lib/types";
import { IconPlay, IconTrash, IconPlus, IconClose } from "./ui-icons";

interface SnippetsPanelProps {
  workspace: Workspace;
  onAddSnippet: (name: string, command: string) => void;
  onDeleteSnippet: (id: SnippetId) => void;
  onRunSnippet: (command: string) => void;
}

export function SnippetsPanel({ workspace, onAddSnippet, onDeleteSnippet, onRunSnippet }: SnippetsPanelProps) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [showForm, setShowForm] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [command]);

  const submit = () => {
    if (!name.trim() || !command.trim()) return;
    onAddSnippet(name.trim(), command.trim());
    setName("");
    setCommand("");
    setShowForm(false);
  };

  return (
    <div className="flex h-full flex-col gap-2">
      {/* Add form at top */}
      <div>
        <button
          onClick={() => setShowForm((v) => !v)}
          className={`flex w-full items-center justify-center gap-1.5 rounded-md border py-1.5 text-xs font-medium transition-colors ${
            showForm
              ? "border-[var(--c-accent)] bg-[var(--c-accent-dim)] text-[var(--c-accent-text)]"
              : "border-dashed border-slate-700 text-slate-400 hover:border-[var(--c-accent)] hover:text-[var(--c-accent-text)]"
          }`}
        >
          <IconPlus size={13} /> Ajouter un snippet
        </button>
        {showForm && (
          <div className="mt-2 space-y-1.5 rounded-lg border border-[var(--c-border)] bg-[var(--c-bg3)]/40 p-2.5">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Nom"
              autoFocus
              className={inputClass}
            />
            <textarea
              ref={textareaRef}
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) submit(); }}
              placeholder={"Commande ou script…\n(Ctrl+Entrée pour valider)"}
              rows={3}
              className={`${inputClass} resize-none overflow-hidden font-mono`}
            />
            <div className="flex gap-1.5">
              <button
                onClick={submit}
                className="flex-1 rounded-md bg-[var(--c-accent)] py-1.5 text-xs font-medium text-white hover:bg-[var(--c-accent-hover)]"
              >
                Enregistrer
              </button>
              <button
                onClick={() => { setShowForm(false); setName(""); setCommand(""); }}
                className="flex items-center justify-center rounded-md bg-slate-700 px-2.5 py-1.5 text-xs text-slate-300 hover:bg-slate-600"
              >
                <IconClose size={12} />
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Snippet list */}
      <div className="flex-1 space-y-2 overflow-y-auto">
        {workspace.snippets.map((snippet) => {
          const isMultiLine = snippet.command.includes("\n");
          return (
            <div key={snippet.id} className="rounded-lg border border-[var(--c-border)] bg-[var(--c-bg3)]/40 p-2.5">
              <p className="truncate text-sm font-medium text-slate-100">{snippet.name}</p>
              <pre className="mt-0.5 line-clamp-3 whitespace-pre-wrap font-mono text-xs text-slate-400">{snippet.command}</pre>
              {isMultiLine && (
                <span className="mt-1 inline-block rounded bg-slate-700/50 px-1.5 py-0.5 text-[10px] text-slate-500">script</span>
              )}
              <div className="mt-2 grid grid-cols-2 gap-1">
                <button
                  onClick={() => onRunSnippet(snippet.command)}
                  className="flex items-center justify-center gap-1.5 rounded-md bg-[var(--c-accent)] px-1.5 py-1.5 text-xs text-white hover:bg-[var(--c-accent-hover)]"
                >
                  <IconPlay size={11} /> Exécuter
                </button>
                <button
                  onClick={() => onDeleteSnippet(snippet.id)}
                  className="flex items-center justify-center gap-1.5 rounded-md bg-slate-700 px-1.5 py-1.5 text-xs text-rose-300 hover:bg-rose-900/60"
                >
                  <IconTrash size={11} /> Supprimer
                </button>
              </div>
            </div>
          );
        })}
        {workspace.snippets.length === 0 && (
          <p className="px-1 py-4 text-center text-sm text-slate-500">Aucun snippet</p>
        )}
      </div>
    </div>
  );
}

const inputClass = "w-full rounded-md bg-slate-800 px-2 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-1 focus:ring-[var(--c-accent)]";
