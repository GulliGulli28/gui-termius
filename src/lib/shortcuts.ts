import { useEffect, type KeyboardEvent as ReactKeyboardEvent } from "react";

export interface ShortcutAction {
  id: string;
  label: string;
  defaultKey: string;
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  { id: "palette.open", label: "Ouvrir la palette de commandes", defaultKey: "Ctrl+K" },
  { id: "sidebar.toggle", label: "Afficher/masquer la barre latérale", defaultKey: "Ctrl+B" },
  { id: "split.toggle", label: "Activer/désactiver le mode split", defaultKey: "Ctrl+\\" },
  { id: "tab.close", label: "Fermer l'onglet actif", defaultKey: "Ctrl+W" },
  { id: "tab.newLocalTerminal", label: "Nouveau terminal local", defaultKey: "Ctrl+T" },
  { id: "tab.next", label: "Onglet suivant", defaultKey: "Ctrl+Tab" },
  { id: "tab.prev", label: "Onglet précédent", defaultKey: "Ctrl+Shift+Tab" },
  { id: "settings.open", label: "Ouvrir les paramètres", defaultKey: "Ctrl+," },
];

export function defaultShortcuts(): Record<string, string> {
  return Object.fromEntries(SHORTCUT_ACTIONS.map((a) => [a.id, a.defaultKey]));
}

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

function normalizeKey(key: string): string {
  if (key === " ") return "Space";
  if (key.length === 1) return key.toUpperCase();
  return key;
}

/** Renders a `KeyboardEvent` as a combo string like `"Ctrl+Shift+K"`, matching the format used to store/display shortcuts. */
export function comboFromEvent(e: KeyboardEvent | ReactKeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (!MODIFIER_KEYS.has(e.key)) parts.push(normalizeKey(e.key));
  return parts.join("+");
}

export function matchesCombo(e: KeyboardEvent, combo: string): boolean {
  return !!combo && comboFromEvent(e) === combo;
}

/**
 * Attaches one window-level keydown listener that dispatches to `handlers` based on
 * `shortcuts` (action id -> combo string, e.g. from `AppPreferences.keyboardShortcuts`).
 * Elements that need to capture raw keys themselves (e.g. a shortcut-rebind input, or
 * xterm's own key handling) should `stopPropagation` so they never reach this listener.
 */
export function useGlobalShortcuts(shortcuts: Record<string, string>, handlers: Record<string, (() => void) | undefined>) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      for (const [id, combo] of Object.entries(shortcuts)) {
        if (matchesCombo(e, combo)) {
          const handler = handlers[id];
          if (handler) {
            e.preventDefault();
            handler();
          }
          return;
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [shortcuts, handlers]);
}
