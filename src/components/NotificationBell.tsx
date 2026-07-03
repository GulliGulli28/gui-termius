import { useState } from "react";
import type { AppNotification } from "../lib/notifications";
import { IconBell, IconClose } from "./ui-icons";

interface NotificationBellProps {
  notifications: AppNotification[];
  onDismiss: (id: string) => void;
  onClearAll: () => void;
  onMarkAllRead: () => void;
}

const KIND_DOT: Record<AppNotification["kind"], string> = {
  info: "bg-sky-400",
  success: "bg-emerald-400",
  error: "bg-rose-400",
};

function formatTime(ts: number): string {
  const diffSec = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (diffSec < 60) return "à l'instant";
  if (diffSec < 3600) return `il y a ${Math.floor(diffSec / 60)} min`;
  if (diffSec < 86400) return `il y a ${Math.floor(diffSec / 3600)} h`;
  return new Date(ts).toLocaleDateString();
}

export function NotificationBell({ notifications, onDismiss, onClearAll, onMarkAllRead }: NotificationBellProps) {
  const [open, setOpen] = useState(false);
  const unreadCount = notifications.filter((n) => !n.read).length;

  return (
    <div className="relative flex items-center">
      <button
        onClick={() => { setOpen((v) => !v); if (!open) onMarkAllRead(); }}
        title="Notifications"
        className={`relative flex h-6 w-7 items-center justify-center rounded text-slate-500 hover:bg-slate-700 hover:text-slate-200 ${open ? "bg-slate-700 text-slate-200" : ""}`}
      >
        <IconBell size={14} />
        {unreadCount > 0 && (
          <span className="absolute right-0.5 top-0.5 flex h-3.5 min-w-3.5 items-center justify-center rounded-full bg-rose-500 px-0.5 text-[9px] font-semibold leading-none text-white">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute left-0 top-full z-20 mt-1 w-80 overflow-hidden rounded-lg border border-slate-700 bg-[var(--c-bg2)] shadow-xl">
            <div className="flex items-center justify-between border-b border-[var(--c-border)] px-3 py-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Notifications</span>
              {notifications.length > 0 && (
                <button onClick={onClearAll} className="text-[11px] text-slate-500 hover:text-slate-300">Tout effacer</button>
              )}
            </div>
            <div className="max-h-80 overflow-y-auto sidebar-scroll">
              {notifications.length === 0 ? (
                <p className="px-3 py-6 text-center text-sm text-slate-500">Aucune notification</p>
              ) : (
                notifications.slice().reverse().map((n) => (
                  <div key={n.id} className="group flex items-start gap-2 border-b border-[var(--c-border)] px-3 py-2 last:border-b-0 hover:bg-[var(--c-bg3)]">
                    <span className={`mt-1 h-1.5 w-1.5 shrink-0 rounded-full ${KIND_DOT[n.kind]}`} />
                    <div className="min-w-0 flex-1">
                      <p className="text-xs text-slate-200">{n.message}</p>
                      <p className="mt-0.5 text-[10px] text-slate-500">{formatTime(n.timestamp)}</p>
                    </div>
                    <button
                      onClick={() => onDismiss(n.id)}
                      className="flex shrink-0 items-center rounded p-0.5 text-slate-500 opacity-0 hover:text-slate-200 group-hover:opacity-100"
                      aria-label="Effacer"
                    >
                      <IconClose size={10} />
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
