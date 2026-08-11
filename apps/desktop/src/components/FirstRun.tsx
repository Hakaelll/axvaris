import type { NetworkInterface } from "../types";
import { useI18n } from "../i18n";
import { Icon } from "./Icon";

interface FirstRunProps {
  interfaces: NetworkInterface[];
  language: string;
  onLanguage: (language: "en" | "es") => void;
  onLive: (selectedInterface: string | null) => void;
  onDemo: () => void;
}

export function FirstRun({
  interfaces,
  language,
  onLanguage,
  onLive,
  onDemo,
}: FirstRunProps) {
  const t = useI18n();
  const available = interfaces.filter(
    (item) => item.is_up && !item.is_loopback,
  );
  const preferred = available.find((item) => item.addresses.length > 0);
  return (
    <div className="first-run">
      <div className="first-run__signal" aria-hidden="true">
        <i />
        <i />
        <i />
        <span />
      </div>
      <label className="first-run__language">
        <span>{t("Language")}</span>
        <select
          aria-label={t("Interface language")}
          onChange={(event) => onLanguage(event.target.value as "en" | "es")}
          value={language.startsWith("es") ? "es" : "en"}
        >
          <option value="en">English</option>
          <option value="es">Castellano</option>
        </select>
      </label>
      <span className="eyebrow">{t("LOCAL NETWORK OBSERVABILITY")}</span>
      <h1>{t("See where your computer connects.")}</h1>
      <p>
        {t(
          "Sabicheiro observes operating-system socket metadata and keeps its history on this device. It does not inspect payloads during normal capture.",
        )}
      </p>
      <div className="first-run__actions">
        <button
          className="button button--primary"
          onClick={() => onLive(preferred?.id ?? null)}
          type="button"
        >
          <Icon name="play" />
          {t("Start live capture")}
        </button>
        <button
          className="button button--secondary"
          onClick={onDemo}
          type="button"
        >
          {t("Explore deterministic demo")}
        </button>
      </div>
      <div className="first-run__facts">
        <span>
          <Icon name="shield" />
          {t("No account or telemetry")}
        </span>
        <span>
          <Icon name="database" />
          {t("Local SQLite history")}
        </span>
        <span>
          <Icon name="globe" />
          {t("Free local GeoIP")}
        </span>
      </div>
      <small>
        {t(
          "Live capture may require elevated OS permissions for complete process attribution. Demo mode never touches the network.",
        )}
      </small>
    </div>
  );
}
