import type { Bookmark } from "../types";
import { useI18n } from "../i18n";

function queryFor(bookmark: Bookmark): string {
  switch (bookmark.kind) {
    case "process":
      return `process:${JSON.stringify(bookmark.value)}`;
    case "domain":
      return `domain:${JSON.stringify(bookmark.value)}`;
    case "ip":
      return `ip:${bookmark.value}`;
    case "country":
      return `country:${bookmark.value}`;
  }
}

export function FocusBar({
  bookmarks,
  onFilter,
  onRemove,
}: {
  bookmarks: Bookmark[];
  onFilter: (query: string) => void;
  onRemove: (id: number) => void;
}) {
  const t = useI18n();
  if (bookmarks.length === 0) return null;
  return (
    <div aria-label={t("Saved local filters")} className="focus-bar">
      <span>{t("WATCHING")}</span>
      {bookmarks.slice(0, 8).map((bookmark) => (
        <div className="focus-chip" key={bookmark.id}>
          <button
            onClick={() => onFilter(queryFor(bookmark))}
            title={t("Filter by {kind} {value}", {
              kind: bookmark.kind,
              value: bookmark.value,
            })}
            type="button"
          >
            <small>{bookmark.kind}</small>
            {bookmark.value}
          </button>
          <button
            aria-label={t("Remove bookmark {value}", {
              value: bookmark.value,
            })}
            onClick={() => onRemove(bookmark.id)}
            title={t("Remove local bookmark")}
            type="button"
          >
            ×
          </button>
        </div>
      ))}
      {bookmarks.length > 8 && <small>+{bookmarks.length - 8}</small>}
    </div>
  );
}
