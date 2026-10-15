import { useI18n } from "../i18n";
import { endpointLabel, formatTime } from "../lib/format";
import { usePresentation } from "../privacy/PresentationMode";
import type { Connection } from "../types";
import { Icon } from "./Icon";

export function ServiceDiagram({
  connections,
  selectedId,
  onSelect,
}: {
  connections: Connection[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useI18n();
  const { display } = usePresentation();
  return (
    <section className="service-diagram" aria-labelledby="services-title">
      <header>
        <h1 id="services-title">{t("Listeners")}</h1>
        <p>{t("Socket metadata only")}</p>
      </header>
      <div className="service-diagram__labels">
        <span>{t("Process")}</span>
        <span>{t("Local endpoint")}</span>
      </div>
      <div className="service-diagram__list">
        {connections.slice(0, 100).map((connection) => (
          <button
            className="service-link"
            key={connection.id}
            aria-pressed={selectedId === connection.id}
            onClick={() => onSelect(connection.id)}
            type="button"
          >
            <span className="service-link__node">
              <Icon name="settings" />
              <span>
                <strong>{display("process", connection.process.name)}</strong>
                <small>
                  PID {connection.process.pid ?? "?"} ·{" "}
                  {formatTime(connection.last_seen_ms)}
                </small>
              </span>
            </span>
            <span className="service-link__edge" aria-hidden="true" />
            <span className="service-link__node">
              <Icon name="globe" />
              <span>
                <strong>
                  {display(
                    "local_ip",
                    endpointLabel(
                      connection.key.local.address,
                      connection.key.local.port,
                    ),
                  )}
                </strong>
                <small>
                  {connection.protocol.toUpperCase()} · {t(connection.state)} ·{" "}
                  {t(connection.active ? "Active" : "closed")}
                </small>
              </span>
            </span>
          </button>
        ))}
        {connections.length === 0 && (
          <div className="today-empty">
            <Icon name="process" />
            <strong>{t("No listener evidence in the latest snapshot")}</strong>
            <p>{t("Start live capture or use deterministic demo data.")}</p>
          </div>
        )}
        {connections.length > 100 && (
          <p>
            {t("Showing {shown} of {count} sockets", {
              shown: 100,
              count: connections.length,
            })}
          </p>
        )}
      </div>
    </section>
  );
}
