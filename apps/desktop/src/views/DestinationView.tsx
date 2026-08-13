import { useEffect, useState } from "react";
import type { AsnProfile, DestinationSummary } from "../types";
import { useI18n } from "../i18n";
import { formatBytes, formatDateTime } from "../lib/format";
import { desktopApi } from "../lib/backend";

export function DestinationView({
  destinations,
  onFilter,
}: {
  destinations: DestinationSummary[];
  onFilter: (query: string) => void;
}) {
  const t = useI18n();
  const [selectedAsn, setSelectedAsn] = useState<number | null>(null);
  const [asnProfile, setAsnProfile] = useState<AsnProfile | null>(null);
  const [asnLoading, setAsnLoading] = useState(false);
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
  useEffect(() => {
    if (selectedAsn === null) {
      setAsnProfile(null);
      return;
    }
    let cancelled = false;
    setAsnLoading(true);
    void desktopApi.getAsnProfile(selectedAsn).then((profile) => {
      if (!cancelled) {
        setAsnProfile(profile);
        setAsnLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [selectedAsn]);
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
      <div className="destination-intelligence-summary">
        <span>{t("Destination intelligence")}</span>
        <strong>{destinations.length}</strong>
        <small>{t("canonical IPs with traceable local evidence")}</small>
        {[...new Set(destinations.flatMap((item) => item.asn?.number ?? []))]
          .slice(0, 5)
          .map((asn) => (
            <button
              aria-pressed={selectedAsn === asn}
              className={selectedAsn === asn ? "is-active" : undefined}
              key={asn}
              onClick={() => setSelectedAsn(asn)}
              type="button"
            >
              AS{asn}
            </button>
          ))}
      </div>
      {(selectedAsn !== null || asnLoading) && (
        <section className="asn-profile" aria-live="polite">
          <header>
            <div>
              <span className="eyebrow">{t("LOCAL ASN PROFILE")}</span>
              <h2>
                {selectedAsn === null ? t("Loading ASN") : `AS${selectedAsn}`}
              </h2>
              <p>
                {t(
                  "The organization describes the observed network record; it does not prove ownership of a domain, service or content.",
                )}
              </p>
            </div>
            <button
              className="button button--secondary"
              disabled={selectedAsn === null}
              onClick={() => onFilter(`asn:${selectedAsn}`)}
              type="button"
            >
              {t("Filter this ASN")}
            </button>
          </header>
          {asnLoading ? (
            <div className="loading-line" />
          ) : asnProfile ? (
            <>
              <div className="asn-profile__metrics">
                <span>
                  <small>{t("ORGANIZATION")}</small>
                  <strong>{asnProfile.organization ?? t("Unknown")}</strong>
                </span>
                <span>
                  <small>{t("OBSERVED IPS")}</small>
                  <strong>{asnProfile.observed_ips.length}</strong>
                </span>
                <span>
                  <small>{t("PROCESSES")}</small>
                  <strong>{asnProfile.process_names.length}</strong>
                </span>
                <span>
                  <small>{t("SESSIONS")}</small>
                  <strong>{asnProfile.session_ids.length}</strong>
                </span>
                <span>
                  <small>{t("TRAFFIC")}</small>
                  <strong>
                    {formatBytes(
                      asnProfile.traffic.bytes_received +
                        asnProfile.traffic.bytes_sent,
                    )}
                  </strong>
                </span>
                <span>
                  <small>{t("OBSERVED")}</small>
                  <strong>
                    {formatDateTime(asnProfile.first_seen_ms)} — {formatDateTime(asnProfile.last_seen_ms)}
                  </strong>
                </span>
              </div>
              <div className="asn-profile__relations">
                <div>
                  <strong>{t("Observed IPs")}</strong>
                  <span>
                    {asnProfile.observed_ips.slice(0, 12).map((ip) => (
                      <button key={ip} onClick={() => onFilter(`ip:${ip}`)} type="button">
                        {ip}
                      </button>
                    ))}
                  </span>
                </div>
                <div>
                  <strong>{t("Related processes")}</strong>
                  <span>
                    {asnProfile.process_names.slice(0, 12).map((process) => (
                      <button
                        key={process}
                        onClick={() => onFilter(`process:${JSON.stringify(process)}`)}
                        type="button"
                      >
                        {process}
                      </button>
                    ))}
                  </span>
                </div>
                <div>
                  <strong>{t("Estimated countries")}</strong>
                  <span>
                    {asnProfile.estimated_countries.length > 0
                      ? asnProfile.estimated_countries.join(" · ")
                      : t("Unknown")}
                  </span>
                </div>
              </div>
            </>
          ) : null}
        </section>
      )}
      <div className="rank-list rank-list--destinations">
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
              type="button"
            >
              <span className="rank-row__identity destination-identity">
                <em>{destination.country_code ?? "?"}</em>
                <span>
                  <strong>{destination.domain ?? destination.ip}</strong>
                  <small>
                    {destination.domain
                      ? `${destination.ip} / ${destination.ports.map((port) => `:${port}`).join(", ")}`
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
                <small>
                  {t("Estimated network location")} / {t(destination.geo_precision)}
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
                <small
                  title={`${formatDateTime(destination.first_seen_ms)} — ${formatDateTime(destination.last_seen_ms)}`}
                >
                  {t("{count} recurrences", {
                    count: destination.recurrence,
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
