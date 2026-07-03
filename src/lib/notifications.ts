export type NotificationKind = "info" | "success" | "error";

export interface AppNotification {
  id: string;
  kind: NotificationKind;
  message: string;
  timestamp: number;
  read: boolean;
}

let nextNotificationId = 0;

export function createNotification(kind: NotificationKind, message: string): AppNotification {
  return { id: `notif-${nextNotificationId++}`, kind, message, timestamp: Date.now(), read: false };
}
