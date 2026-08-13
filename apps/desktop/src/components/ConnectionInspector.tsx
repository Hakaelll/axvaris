import { useEffect, useState } from "react";
import { desktopApi } from "../lib/backend";
import { useI18n } from "../i18n";
import type {
  Connection,
  ConnectionHistoryStats,
  IdentityTimelineEvent,
  RuleKind,
} from "../types";
import {
  endpointLabel,
  formatBytes,
  formatDateTime,
  formatDuration,
} from "../lib/format";
import { Icon } from "./Icon";

interface InspectorProps {
  connection: Connection;
  flowTrafficAvailable: boolean;
  onClose: () => void;
  onFilterProcess: (process: string) => void;
  onFilterDestination: (destination: string) => void;
  onFilterCountry: (country: string) => void;
  onBookmark: (kind: Exclude<RuleKind, "localhost">, value: string) => void;
  onIgnore: (kind: RuleKind, value: string) => void;
}

function Field({
  label,
  children,
  mono = false,
}: {
  label: string;
  children: React.ReactNode;
  mono?: boolean;
}) {
  return (
    <div className={`inspector-field${mono ? " is-mono" : ""}`}>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

export function ConnectionInspector({
  connection,
  flowTrafficAvailable,
  onClose,
  onFilterProcess,
  onFilterDestination,
  onFilterCountry,
  onBookmark,
  onIgnore,
}: InspectorProps) {
  const t = useI18n();
  const destination =
    connection.domain ?? connection.hostname ?? connection.key.remote.address;
  const destinationKind: "domain" | "ip" =
    connection.domain || connection.hostname ? "domain" : "ip";
  const [history, setHistory] = useState<ConnectionHistoryStats | null>(null);
  const [identityTimeline, setIdentityTimeline] = useState<
    IdentityTimelineEvent[]
  >([]);

  useEffect(() => {
    let cancelled = false;
    setHistory(null);
    void desktopApi
      .getConnectionHistoryStats(connection.key.remote.address)
      .then((result) => {
        if (!cancelled) setHistory(result);
      })
      .catch(() => {
        if (!cancelled) {
          setHistory({
            first_ever_seen_ms: connection.first_seen_ms,
            previous_connections: Math.max(0, connection.repeat_count - 1),
            sessions: 1,
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [
    connection.first_seen_ms,
    connection.key.remote.address,
    connection.repeat_count,
  ]);

  useEffect(() => {
    let cancelled = false;
    setIdentityTimeline([]);
    const timer = window.setTimeout(() => {
      void desktopApi
        .getIdentityTimeline(connection.id, null)
        .then((events) => {
          if (!cancelled) setIdentityTimeline(events);
        })
        .catch(() => {
          // A just-opened live row may not have reached the next SQLite batch yet.
          // The evidence fields below remain the authoritative immediate view.
        });
    }, 300);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [connection.id]);

  return (
    <aside aria-label={t("Connection inspector")} className="inspector">
      <header>
        <div>
          <span
            className={`connection-state${connection.active ? " is-live" : ""}`}
          />
          <span>
            {connection.active
              ? t("ACTIVE CONNECTION")
              : t("RECENT CONNECTION")}
          </span>
        </div>
        <button
          aria-label={t("Close inspector")}
          className="icon-button"
          onClick={onClose}
          type="button"
        >
          <Icon name="close" />
        </button>
      </header>
      <div className="inspector__hero">
        <span className="process-avatar process-avatar--large">
          {connection.process.name.slice(0, 2).toUpperCase()}
        </span>
        <div>
          <h2>{connection.process.name}</h2>
          <p>
            {connection.process.executable ??
              `PID ${connection.process.pid ?? t("unknown")}`}
          </p>
        </div>
      </div>
      <div className="inspector__route">
        <span className="route-node is-local">
          <i />
          {t("THIS DEVICE")}
        </span>
        <span className="route-line">
          <i />
        </span>
        <span className="route-node">
          <i />
          {connection.geo?.country_code ?? "?"}
        </span>
      </div>
      {connection.unusual.map((signal) => (
        <section
          className="signal-callout"
          key={`${signal.kind}-${signal.title}`}
        >
          <strong>{t(signal.title)}</strong>
          <p>{t(signal.reason)}</p>
          {signal.typical && <small>{t(signal.typical)}</small>}
        </section>
      ))}
      <section className="inspector-section">
        <h3>{t("Evidence")}</h3>
        <dl>
          <Field label={t("Endpoint role")}>
            {t(connection.evidence.endpoint_role)}
          </Field>
          <Field label={t("Reason")} mono>
            {t(connection.evidence.role_reason)}
          </Field>
          <Field label={t("OS source")} mono>
            {t(connection.evidence.observation_source)}
          </Field>
          <Field label={t("Correlation")}>
            {t(connection.evidence.correlation_quality)}
          </Field>
          <Field label={t("Evidence sequence")} mono>
            {connection.evidence.sequence || t("Unavailable")}
          </Field>
        </dl>
      </section>
      <section className="inspector-section">
        <h3>{t("Identity")}</h3>
        <dl>
          <Field label={t("Name")}>{connection.process.name}</Field>
          <Field label={t("Executable")} mono>
            {connection.process.executable ?? t("Unknown")}
          </Field>
          <Field label="PID" mono>
            {connection.process.pid ?? t("Unknown")}
          </Field>
          <Field label={t("Process instance")} mono>
            {connection.key.process_instance ?? t("Not exposed")}
          </Field>
          <Field label={t("Path")} mono>
            {connection.process.executable_path ??
              t("Not exposed by the operating system")}
          </Field>
        </dl>
      </section>
      <section className="inspector-section">
        <h3>{t("Endpoint")}</h3>
        <dl>
          <Field label="IP" mono>
            {connection.key.remote.address}
          </Field>
          <Field label={t("Hostname")} mono>
            {connection.hostname ?? t("Unresolved")}
          </Field>
          <Field label={t("Domain")} mono>
            {connection.domain ?? t("Unresolved")}
          </Field>
          <Field label={t("Endpoint")} mono>
            {endpointLabel(
              connection.key.remote.address,
              connection.key.remote.port,
            )}
          </Field>
          <Field label={t("Address scope")}>
            {t(connection.key.remote.scope)}
          </Field>
          <Field label={t("Observed family")}>
            {connection.key.remote.observed_family.toUpperCase()}
          </Field>
          <Field label={t("Zone / interface")} mono>
            {connection.key.remote.zone ??
              connection.key.interface_id ??
              t("Not applicable")}
          </Field>
        </dl>
      </section>
      <section className="inspector-section">
        <h3>{t("Estimated network location")}</h3>
        <dl>
          <Field label={t("Country")}>
            {connection.geo
              ? (connection.geo.country_name ??
                connection.geo.country_code ??
                t("Unknown"))
              : t("Unknown")}
          </Field>
          <Field label={t("Location")}>
            {[connection.geo?.city, connection.geo?.region]
              .filter(Boolean)
              .join(", ") || t("Unknown")}
          </Field>
          <Field label={t("Coordinates")} mono>
            {connection.geo?.latitude !== null &&
            connection.geo?.latitude !== undefined &&
            connection.geo.longitude !== null
              ? `${connection.geo.latitude.toFixed(4)}, ${connection.geo.longitude.toFixed(4)}`
              : t("Not plotted")}
          </Field>
          <Field label={t("Network owner")}>
            {connection.asn
              ? `AS${connection.asn.number}${connection.asn.organization ? ` / ${connection.asn.organization}` : ""}`
              : t("Unknown")}
          </Field>
          <Field label={t("Precision")}>
            {connection.geo ? t(connection.geo.precision) : t("Unknown")}
          </Field>
          <Field label={t("Placement")}>
            {connection.geo ? t(connection.geo.placement) : t("Not plotted")}
          </Field>
          <Field label={t("Accuracy radius")}>
            {connection.geo?.accuracy_radius_km !== null &&
            connection.geo?.accuracy_radius_km !== undefined
              ? `${connection.geo.accuracy_radius_km} km`
              : t("Not provided")}
          </Field>
          <Field label={t("Source")} mono>
            {connection.geo?.source ?? t("No local match")}
          </Field>
          <Field label={t("Dataset")} mono>
            {connection.geo?.dataset_edition ?? t("Unavailable")}
          </Field>
          <Field label={t("Dataset date")}>
            {connection.geo?.dataset_build_epoch_ms
              ? formatDateTime(connection.geo.dataset_build_epoch_ms)
              : t("Unavailable")}
          </Field>
          <Field label={t("Matched prefix")} mono>
            {connection.geo?.matched_prefix !== null &&
            connection.geo?.matched_prefix !== undefined
              ? `/${connection.geo.matched_prefix}`
              : t("Unavailable")}
          </Field>
          <Field label="SHA-256" mono>
            {connection.geo?.dataset_sha256
              ? `${connection.geo.dataset_sha256.slice(0, 16)}…`
              : t("Unavailable")}
          </Field>
        </dl>
      </section>
      <section className="inspector-section">
        <h3>{t("Diagnostic")}</h3>
        <dl>
          <Field label={t("Protocol")}>
            {connection.protocol.toUpperCase()} / {t(connection.direction)}
          </Field>
          <Field label={t("Local")} mono>
            {endpointLabel(
              connection.key.local.address,
              connection.key.local.port,
            )}
          </Field>
          <Field label={t("Remote")} mono>
            {endpointLabel(
              connection.key.remote.address,
              connection.key.remote.port,
            )}
          </Field>
          <Field label={t("Interface")}>
            {connection.interface ?? t("System default")}
          </Field>
          <Field label={t("State")}>{t(connection.state)}</Field>
          <Field label={t("Capture generation")} mono>
            {connection.key.capture_generation}
          </Field>
          <Field label={t("Flow generation")} mono>
            {connection.key.flow_generation}
          </Field>
          <Field label={t("Counter source")} mono>
            {t(connection.counter_source)}
          </Field>
        </dl>
      </section>
      <section className="inspector-section inspector-section--traffic">
        <h3>{t("Traffic")}</h3>
        {!flowTrafficAvailable ? (
          <p className="inspector-metric-note">
            {t(
              "The current socket provider exposes endpoints but not per-connection byte counters. Live device totals are shown in the bottom bar.",
            )}
          </p>
        ) : (
          <>
            <div className="traffic-pair">
              <div>
                <span>{t("DOWN")}</span>
                <strong>
                  {formatBytes(connection.rates.down_bytes_per_second, true)}
                </strong>
                <small>
                  {formatBytes(connection.traffic.bytes_received)} {t("total")}
                </small>
              </div>
              <div>
                <span>{t("UP")}</span>
                <strong>
                  {formatBytes(connection.rates.up_bytes_per_second, true)}
                </strong>
                <small>
                  {formatBytes(connection.traffic.bytes_sent)} {t("total")}
                </small>
              </div>
            </div>
            <dl>
              <Field label={t("Packets down")}>
                {connection.traffic.packets_received.toLocaleString()}
              </Field>
              <Field label={t("Packets up")}>
                {connection.traffic.packets_sent.toLocaleString()}
              </Field>
            </dl>
          </>
        )}
      </section>
      <section className="inspector-section">
        <h3>{t("Timing")}</h3>
        <dl>
          <Field label={t("First seen")}>
            {formatDateTime(connection.first_seen_ms)}
          </Field>
          <Field label={t("Last seen")}>
            {formatDateTime(connection.last_seen_ms)}
          </Field>
          <Field label={t("Duration")}>
            {formatDuration(connection.duration_ms)}
          </Field>
          <Field label={t("Observed")}>
            {connection.repeat_count}{" "}
            {connection.repeat_count === 1 ? t("time") : t("times")}
          </Field>
        </dl>
      </section>
      <section className="inspector-section">
        <h3>{t("History")}</h3>
        <dl>
          <Field label={t("First ever seen")}>
            {history?.first_ever_seen_ms
              ? formatDateTime(history.first_ever_seen_ms)
              : t("Checking local history...")}
          </Field>
          <Field label={t("Previous connections")}>
            {history?.previous_connections ?? "-"}
          </Field>
          <Field label={t("Sessions")}>{history?.sessions ?? "-"}</Field>
        </dl>
        {identityTimeline.length > 0 && (
          <ol className="identity-timeline">
            {identityTimeline.map((event) => (
              <li key={event.id}>
                <time>{formatDateTime(event.timestamp_ms)}</time>
                <strong>{t(event.title)}</strong>
                <span>{t(event.detail)}</span>
                <small>
                  {t(event.source)} / {t(event.confidence)}
                </small>
              </li>
            ))}
          </ol>
        )}
      </section>
      <div className="inspector__actions">
        <button
          onClick={() => onFilterProcess(connection.process.name)}
          type="button"
        >
          {t("Filter process")}
        </button>
        <button onClick={() => onFilterDestination(destination)} type="button">
          {t("Filter destination")}
        </button>
        {connection.geo?.country_code && (
          <button
            onClick={() => onFilterCountry(connection.geo!.country_code!)}
            type="button"
          >
            {t("Show country")}
          </button>
        )}
        <button
          onClick={() => onBookmark("process", connection.process.name)}
          type="button"
        >
          {t("Watch process")}
        </button>
        <button
          onClick={() => onBookmark(destinationKind, destination)}
          type="button"
        >
          {t("Watch destination")}
        </button>
        <button
          onClick={() => onIgnore(destinationKind, destination)}
          type="button"
        >
          {t("Ignore destination")}
        </button>
      </div>
    </aside>
  );
}
