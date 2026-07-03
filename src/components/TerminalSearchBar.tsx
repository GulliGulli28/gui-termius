import { useEffect, useRef, useState } from "react";
import { IconSearch, IconChevronDown, IconChevronRight, IconClose } from "./ui-icons";

interface TerminalSearchBarProps {
  onSearch: (value: string, direction: "next" | "prev") => void;
  onClose: () => void;
}

export function TerminalSearchBar({ onSearch, onClose }: TerminalSearchBarProps) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { inputRef.current?.focus(); }, []);

  return (
    <div className="absolute right-3 top-3 z-10 flex items-center gap-1 rounded-md border border-[var(--c-border)] bg-[var(--c-bg2)] px-2 py-1.5 shadow-xl">
      <IconSearch size={12} className="shrink-0 text-slate-500" />
      <input
        ref={inputRef}
        value={value}
        onChange={(e) => { setValue(e.target.value); onSearch(e.target.value, "next"); }}
        onKeyDown={(e) => {
          if (e.key === "Enter") { e.preventDefault(); onSearch(value, e.shiftKey ? "prev" : "next"); }
          if (e.key === "Escape") { e.preventDefault(); onClose(); }
        }}
        placeholder="Rechercher dans le terminal…"
        className="w-48 bg-transparent text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none"
      />
      <button onClick={() => onSearch(value, "prev")} title="Occurrence précédente (Maj+Entrée)" className="flex shrink-0 items-center rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-slate-200">
        <IconChevronRight size={11} className="-rotate-90" />
      </button>
      <button onClick={() => onSearch(value, "next")} title="Occurrence suivante (Entrée)" className="flex shrink-0 items-center rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-slate-200">
        <IconChevronDown size={11} />
      </button>
      <button onClick={onClose} title="Fermer (Échap)" className="flex shrink-0 items-center rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-slate-200">
        <IconClose size={11} />
      </button>
    </div>
  );
}
