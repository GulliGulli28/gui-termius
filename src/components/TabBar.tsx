import { IconTerminal, IconTransfer, IconMonitor, IconSplit, IconClose } from "./ui-icons";

interface TabMeta {
  id: string;
  kind: "terminal" | "transfer" | "local-terminal";
  label: string;
}

interface TabBarProps {
  tabs: TabMeta[];
  activeTabId: string | null;
  splitOpen: boolean;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onToggleSplit: () => void;
}

function TabIcon({ kind }: { kind: TabMeta["kind"] }) {
  if (kind === "terminal") return <IconTerminal size={13} />;
  if (kind === "transfer") return <IconTransfer size={13} />;
  return <IconMonitor size={13} />;
}

export function TabBar({ tabs, activeTabId, splitOpen, onSelect, onClose, onToggleSplit }: TabBarProps) {
  return (
    <div className="flex shrink-0 items-center gap-1 border-b border-[var(--c-border)] bg-[var(--c-bg2)] p-1.5">
      <div className="flex min-w-0 flex-1 gap-1 overflow-x-auto">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          return (
            <div
              key={tab.id}
              onClick={() => onSelect(tab.id)}
              className={`flex shrink-0 cursor-pointer items-center gap-1.5 rounded-md px-2.5 py-1.5 text-sm transition-colors ${
                isActive
                  ? "bg-[var(--c-accent)] text-white"
                  : "bg-[var(--c-bg3)] text-slate-300 hover:bg-slate-700"
              }`}
            >
              <TabIcon kind={tab.kind} />
              <span className="max-w-[12rem] truncate">{tab.label}</span>
              <button
                onClick={(e) => { e.stopPropagation(); onClose(tab.id); }}
                className="flex items-center rounded p-0.5 opacity-60 hover:opacity-100"
                aria-label="Fermer l'onglet"
              >
                <IconClose size={10} />
              </button>
            </div>
          );
        })}
      </div>
      <button
        onClick={onToggleSplit}
        title={splitOpen ? "Quitter le mode split" : "Mode split — deux terminaux côte à côte"}
        className={`flex shrink-0 items-center justify-center rounded-md p-1.5 transition-colors ${
          splitOpen
            ? "bg-[var(--c-accent)] text-white"
            : "text-slate-400 hover:bg-[var(--c-bg3)] hover:text-slate-200"
        }`}
      >
        <IconSplit size={15} />
      </button>
    </div>
  );
}
