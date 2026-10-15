import { useI18n } from "../i18n";
import {
  endpointLabel,
  formatBytes,
  formatDateTime,
  formatDuration,
} from "../lib/format";
import { usePresentation } from "../privacy/PresentationMode";
import type { HistoryEntry } from "../types";
import { Icon } from "./Icon";

export function HistoryInspector({
  entry,
  onClose,
  onReconstruct,
}: {
  entry: HistoryEntry;
  onClose: () => void;
  onReconstruct: () => void;
}) {
  const t = useI18n();
  const { display } = usePresentation();
  const { connection } = entry;
  const trafficAvailable = connection.counter_source !== "unavailable";
  const destination = display(
    connection.key.remote.scope === "global" ? "public_ip" : "local_ip",
    connection.key.remote.address,
  );
  return (
    <aside
      className="history-inspector listener-inspector"
      aria-label={t("Selected connection")}
    >
      <header>
        <Icon name="process" />
        <h2>{display("process", connection.process.name)}</h2>
        <button
          aria-label={t("Close inspector")}
          className="icon-button"
          onClick={onClose}
          type="button"
        >
          <Icon name="close" />
        </button>
      </header>
      <p>
        {formatDateTime(connection.first_seen_ms)} ·{" "}
        {formatDuration(connection.last_seen_ms - connection.first_seen_ms)}
      </p>
      <div className="history-inspector__route">
        <span>
          <Icon name="settings" />
          <strong>{display("process", connection.process.name)}</strong>
          <small>PID {connection.process.pid ?? "?"}</small>
        </span>
        <Icon name="chevron" />
        <span>
          <Icon name="globe" />
          <code>{destination}</code>
          <small>
            {connection.protocol.toUpperCase()} {connection.key.remote.port}
          </small>
        </span>
      </div>
      <dl>
        <div>
          <dt>{t("Destination")}</dt>
          <dd>
            <code>
              {display(
                connection.key.remote.scope === "global"
                  ? "public_ip"
                  : "local_ip",
                endpointLabel(
                  connection.key.remote.address,
                  connection.key.remote.port,
                ),
              )}
            </code>
          </dd>
        </div>
        <div>
          <dt>DNS</dt>
          <dd>
            {connection.domain
              ? display("domain", connection.domain)
              : connection.hostname
                ? display("domain", connection.hostname)
                : t("No DNS evidence")}
          </dd>
        </div>
        <div>
          <dt>{t("State")}</dt>
          <dd>{t(connection.state)}</dd>
        </div>
        <div>
          <dt>{t("Evidence")}</dt>
          <dd>{t(connection.evidence.correlation_quality)}</dd>
        </div>
        <div>
          <dt>{t("Sent")}</dt>
          <dd>
            {trafficAvailable
              ? formatBytes(connection.traffic.bytes_sent)
              : "—"}
          </dd>
        </div>
        <div>
          <dt>{t("Received")}</dt>
          <dd>
            {trafficAvailable
              ? formatBytes(connection.traffic.bytes_received)
              : "—"}
          </dd>
        </div>
        <div>
          <dt>{t("Interface")}</dt>
          <dd>
            {connection.interface
              ? display("interface", connection.interface)
              : t("Interface unavailable")}
          </dd>
        </div>
      </dl>
      <button
        className="button button--primary"
        onClick={onReconstruct}
        type="button"
      >
        {t("Reconstruct globe")}
      </button>
    </aside>
  );
}
