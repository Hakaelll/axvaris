import { useI18n } from "../i18n";
import type { LocalNotification } from "../hooks/useSabicheiro";

interface NotificationCenterProps {
  items: LocalNotification[];
  onClear: () => void;
  onDismiss: (id: string) => void;
}

export function NotificationCenter({
  items,
  onClear,
  onDismiss,
}: NotificationCenterProps) {
  const t = useI18n();
  if (items.length === 0) return null;
  return (
    <section
      aria-label={t("Local notifications")}
      aria-live="polite"
      className="notification-center"
    >
      <header>
        <strong>{t("Local notifications")}</strong>
        <button onClick={onClear} type="button">
          {t("Clear all")}
        </button>
      </header>
      {items.slice(0, 4).map((item) => (
        <article key={item.id}>
          <div>
            <strong>{t(item.title)}</strong>
            <p>{item.detail}</p>
          </div>
          <button
            aria-label={t("Dismiss notification")}
            onClick={() => onDismiss(item.id)}
            type="button"
          >
            ×
          </button>
        </article>
      ))}
    </section>
  );
}
