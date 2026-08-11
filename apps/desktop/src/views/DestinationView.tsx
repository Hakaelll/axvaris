import type { DestinationSummary } from "../types";
import { useI18n } from "../i18n";
import { formatBytes } from "../lib/format";

export function DestinationView({
  destinations,
  onFilter,
}: {
  destinations: DestinationSummary[];
  onFilter: (query: string) => void;
}) {
  const t = useI18n();
  const countries = new Map<
    string,
    { name: string; connections: number; traffic: number }
  >();
  for (const item of destinations) {
    const code = item.country_code ?? "??";
    const current = countries.get(code) ?? {
      name: item.country_name ?? t("Unknown"),
      connections: 0,
      traffic: 0,
    };
    current.connections += item.active_connections;
    current.traffic += item.traffic.bytes_received + item.traffic.bytes_sent;
    countries.set(code, current);
  }
  return (
    <section className="data-view">
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("REMOTE INFRASTRUCTURE")}</span>
          <h1>{t("Destinations")}</h1>
          <p>
            {t(
              "Domains, IP addresses, networks and countries contacted by local processes.",
            )}
          </p>
        </div>
        <div className="country-pills" aria-label={t("Filter by country")}>
          {[...countries.entries()]
            .sort((a, b) => b[1].traffic - a[1].traffic)
            .slice(0, 6)
            .map(([code, item]) => (
              <button
                key={code}
                onClick={() =>
                  onFilter(
                    code === "??" ? "country:unknown" : `country:${code}`,
                  )
                }
                type="button"
              >
                <strong>{code}</strong>
                <span>{item.connections}</span>
              </button>
            ))}
        </div>
      </header>
      <div className="rank-list rank-list--destinations" role="list">
        <div className="rank-list__header">
          <span>{t("Destination")}</span>
          <span>{t("Network")}</span>
          <span>{t("Processes")}</span>
          <span>{t("Traffic")}</span>
          <span />
        </div>
        {destinations.map((destination, index) => {
          const query = destination.domain
            ? `domain:${JSON.stringify(destination.domain)}`
            : `ip:${destination.ip}`;
          return (
            <button
              className="rank-row"
              key={`${destination.key}-${index}`}
              onClick={() => onFilter(query)}
              role="listitem"
              type="button"
            >
              <span className="rank-row__identity destination-identity">
                <em>{destination.country_code ?? "?"}</em>
                <span>
                  <strong>{destination.domain ?? destination.ip}</strong>
                  <small>
                    {destination.domain
                      ? destination.ip
                      : t("Unresolved domain")}
                  </small>
                </span>
              </span>
              <span>
                <strong>
                  {destination.asn
                    ? `AS${destination.asn.number}`
                    : t("Unknown")}
                </strong>
                <small>
                  {destination.asn?.organization ??
                    destination.country_name ??
                    t("No local GeoIP match")}
                </small>
              </span>
              <span>
                <strong>{destination.process_names.length}</strong>
                <small>
                  {destination.process_names.slice(0, 2).join(", ")}
                </small>
              </span>
              <span>
                <strong>
                  {formatBytes(
                    destination.traffic.bytes_received +
                      destination.traffic.bytes_sent,
                  )}
                </strong>
                <small>
                  {t("{active} active · {total} seen", {
                    active: destination.active_connections,
                    total: destination.total_connections,
                  })}
                </small>
              </span>
              <span className="rank-row__arrow">→</span>
            </button>
          );
        })}
        {destinations.length === 0 && (
          <div className="list-empty">
            {t("No destinations match the current view.")}
          </div>
        )}
      </div>
    </section>
  );
}
