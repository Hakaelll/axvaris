import type {
  HistoryEntry,
  HistoryQuery,
  InvestigationQuery,
  SavedInvestigation,
} from "../types";
import type { LatestRequest } from "./latestRequest";

export function buildInvestigationQuery({
  query,
  savedAtMs,
  pinnedConnectionIds,
  pinnedEntries,
  previousQuery = null,
}: {
  query: HistoryQuery;
  savedAtMs: number;
  pinnedConnectionIds: string[];
  pinnedEntries: HistoryEntry[];
  previousQuery?: InvestigationQuery | null;
}): InvestigationQuery {
  const previousPins = new Set(previousQuery?.pinned_connection_ids ?? []);
  const keepPinScope =
    previousQuery !== null &&
    previousPins.size === new Set(pinnedConnectionIds).size &&
    pinnedConnectionIds.every((id) => previousPins.has(id));
  return {
    filter: query.query,
    from_ms: query.from_ms,
    to_ms: query.to_ms ?? savedAtMs,
    session_id: query.session_id,
    pinned_connection_ids: pinnedConnectionIds,
    pinned_destination_ips: keepPinScope
      ? previousQuery.pinned_destination_ips
      : [
          ...new Set(
            pinnedEntries.map(
              ({ connection }) => connection.key.remote.address,
            ),
          ),
        ],
    pinned_process_keys: keepPinScope
      ? previousQuery.pinned_process_keys
      : [
          ...new Set(
            pinnedEntries.flatMap(({ connection }) =>
              connection.key.process_instance
                ? [connection.key.process_instance]
                : [],
            ),
          ),
        ],
    pinned_asns: keepPinScope
      ? previousQuery.pinned_asns
      : [
          ...new Set(
            pinnedEntries.flatMap(({ connection }) =>
              connection.asn ? [connection.asn.number] : [],
            ),
          ),
        ],
  };
}

type InvestigationWindow = Pick<InvestigationQuery, "from_ms" | "to_ms">;

export function savedInvestigationWindow(
  current: InvestigationWindow | null,
  query: InvestigationWindow,
): InvestigationWindow {
  return current &&
    current.from_ms === query.from_ms &&
    current.to_ms === query.to_ms
    ? current
    : { from_ms: query.from_ms, to_ms: query.to_ms };
}

export async function saveInvestigationDraft({
  draft,
  persist,
  requests,
}: {
  draft: SavedInvestigation;
  requests: LatestRequest;
  persist: (draft: SavedInvestigation) => Promise<SavedInvestigation>;
}): Promise<SavedInvestigation | null> {
  const token = requests.begin();
  const saved = await persist(draft);
  return requests.isCurrent(token) ? saved : null;
}
