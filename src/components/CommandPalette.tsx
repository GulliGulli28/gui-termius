import { useEffect, useRef, useState } from "react";

export interface PaletteCommand {
  id: string;
  label: string;
  hint?: string;
  run: () => void;
}

interface CommandPaletteProps {
  commands: PaletteCommand[];
  onClose: () => void;
}

export function CommandPalette({ commands, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { inputRef.current?.focus(); }, []);

  const filtered = query.trim()
    ? commands.filter((c) => c.label.toLowerCase().includes(query.trim().toLowerCase()))
    : commands;

  useEffect(() => { setActiveIndex(0); }, [query]);

  const runAt = (index: number) => {
    const cmd = filtered[index];
    if (cmd) { cmd.run(); onClose(); }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[15vh]" onClick={onClose}>
      <div
        className="w-full max-w-lg overflow-hidden rounded-lg border border-slate-700 bg-[var(--c-bg2)] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Escape") { e.preventDefault(); onClose(); }
            if (e.key === "ArrowDown") { e.preventDefault(); setActiveIndex((i) => Math.min(i + 1, filtered.length - 1)); }
            if (e.key === "ArrowUp") { e.preventDefault(); setActiveIndex((i) => Math.max(i - 1, 0)); }
            if (e.key === "Enter") { e.preventDefault(); runAt(activeIndex); }
          }}
          placeholder="Tapez une commande… (se connecter, fermer l'onglet, paramètres…)"
          className="w-full border-b border-[var(--c-border)] bg-transparent px-4 py-3 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none"
        />
        <div className="sidebar-scroll max-h-80 overflow-y-auto py-1">
          {filtered.length === 0 && <p className="px-4 py-6 text-center text-sm text-slate-500">Aucun résultat</p>}
          {filtered.map((cmd, i) => (
            <button
              key={cmd.id}
              onClick={() => runAt(i)}
              onMouseEnter={() => setActiveIndex(i)}
              className={`flex w-full items-center justify-between gap-2 px-4 py-2 text-left text-sm transition-colors ${
                i === activeIndex ? "bg-[var(--c-accent-dim)] text-[var(--c-accent-text)]" : "text-slate-300"
              }`}
            >
              <span className="truncate">{cmd.label}</span>
              {cmd.hint && <span className="shrink-0 rounded bg-slate-700/60 px-1.5 py-0.5 text-[10px] text-slate-400">{cmd.hint}</span>}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
