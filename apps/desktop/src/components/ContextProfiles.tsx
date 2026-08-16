import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import { desktopApi } from "../lib/backend";
import type { HistoryEntry, NetworkProfile } from "../types";
import { formatBytes, formatDateTime } from "../lib/format";

interface ContextProfilesProps {
  entries: HistoryEntry[];
  onFilter: (query: string) => void;
}

function counts(values: string[]): [string, number][] {
  const result = new Map<string, number>();
  for (const value of values) result.set(value, (result.get(value) ?? 0) + 1);
  return [...result.entries()].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
}

export function ContextProfiles({ entries, onFilter }: ContextProfilesProps) {
  const t = useI18n();
  const processNames = useMemo(
    () => [...new Set(entries.map((entry) => entry.connection.process.name))].sort(),
    [entries],
  );
  const [selected, setSelected] = useState("");
  const [profiles, setProfiles] = useState<NetworkProfile[]>([]);
  const [leftProfile, setLeftProfile] = useState("");
  const [rightProfile, setRightProfile] = useState("");

  useEffect(() => {
    if (!selected && processNames[0]) setSelected(processNames[0]);
    if (selected && !processNames.includes(selected)) setSelected(processNames[0] ?? "");
  }, [processNames, selected]);
  useEffect(() => {
    void desktopApi.listNetworkProfiles().then((items) => {
      setProfiles(items);
      setLeftProfile((current) => current || items[1]?.id || items[0]?.id || "");
      setRightProfile((current) => current || items[0]?.id || "");
    });
  }, []);

  const related = entries.filter((entry) => entry.connection.process.name === selected);
  const revisions = counts(
    related.map(
      (entry) =>
        entry.connection.process.executable_revision_id ??
        `${entry.connection.process.executable ?? entry.connection.process.name}:uninspected`,
    ),
  );
  const destinations = counts(
    related.map((entry) => entry.connection.key.remote.address),
  );
  const domains = counts(
    related.flatMap((entry) => entry.connection.domain ? [entry.connection.domain] : []),
  );
  const ports = counts(
    related.map((entry) => `${entry.connection.protocol}/${entry.connection.key.remote.port}`),
  );
  const paths = counts(
    related.map((entry) => entry.connection.interface ?? t("System default")),
  );
  const listeners = related.filter(
    (entry) => entry.connection.state === "listening",
  );
  const totalBytes = related.reduce(
    (sum, entry) =>
      sum +
      entry.connection.traffic.bytes_received +
      entry.connection.traffic.bytes_sent,
    0,
  );
  const first = related.length
    ? Math.min(...related.map((entry) => entry.connection.first_seen_ms))
    : null;
  const last = related.length
    ? Math.max(...related.map((entry) => entry.connection.last_seen_ms))
    : null;
  const hours = Array.from({ length: 24 }, (_, hour) => ({
    hour,
    count: related.filter(
      (entry) => new Date(entry.connection.first_seen_ms).getHours() === hour,
    ).length,
  }));
  const maxHour = Math.max(1, ...hours.map((item) => item.count));
  const left = profiles.find((profile) => profile.id === leftProfile) ?? null;
  const right = profiles.find((profile) => profile.id === rightProfile) ?? null;

  const rename = async (profile: NetworkProfile, name: string) => {
    const updated = await desktopApi.renameNetworkProfile(
      profile.id,
      name.trim() || null,
    );
    setProfiles((items) => [updated, ...items.filter((item) => item.id !== updated.id)]);
  };

  const ranked = (
    title: string,
    values: [string, number][],
    query: (value: string) => string,
  ) => (
    <section>
      <h3>{title}</h3>
      {values.slice(0, 8).map(([value, count]) => (
        <button key={value} onClick={() => onFilter(query(value))} type="button">
          <span>{value}</span><strong>{count}</strong>
        </button>
      ))}
      {values.length === 0 && <p>{t("No evidence in the selected window.")}</p>}
    </section>
  );

  return (
    <section className="context-profiles" aria-labelledby="application-profile-title">
      <header>
        <div><span className="eyebrow">{t("HISTORICAL CONTEXT")}</span><h2 id="application-profile-title">{t("Application and network profiles")}</h2><p>{t("Every aggregate below is computed from the exact common query and drills back to retained connection IDs.")}</p></div>
        <select aria-label={t("Application profile")} onChange={(event) => setSelected(event.target.value)} value={selected}>{processNames.map((name) => <option key={name}>{name}</option>)}</select>
      </header>
      <div className="application-profile-summary">
        <article><strong>{related.length}</strong><span>{t("connections")}</span></article>
        <article><strong>{destinations.length}</strong><span>{t("destinations")}</span></article>
        <article><strong>{domains.length}</strong><span>{t("domains")}</span></article>
        <article><strong>{revisions.length}</strong><span>{t("binary revisions")}</span></article>
        <article><strong>{listeners.length}</strong><span>{t("listeners")}</span></article>
        <article><strong>{formatBytes(totalBytes)}</strong><span>{t("observed traffic")}</span></article>
      </div>
      <div className="application-profile-window"><span>{t("First observed")}: <strong>{formatDateTime(first)}</strong></span><span>{t("Last observed")}: <strong>{formatDateTime(last)}</strong></span></div>
      <div className="application-profile-grid">
        {ranked(t("Executable revisions"), revisions, () => `process:${JSON.stringify(selected)}`)}
        {ranked(t("Destinations"), destinations, (value) => `ip:${value}`)}
        {ranked(t("Domains"), domains, (value) => `domain:${JSON.stringify(value)}`)}
        {ranked(t("Ports and protocols"), ports, (value) => `protocol:${value.split("/")[0]} port:${value.split("/")[1]}`)}
        {ranked(t("Observed paths"), paths, () => `process:${JSON.stringify(selected)}`)}
        <section className="hour-profile"><h3>{t("Usual hours")}</h3><div aria-label={t("Connections by local hour")} role="img">{hours.map((item) => <span key={item.hour} style={{ height: `${Math.max(2, (item.count / maxHour) * 100)}%` }} title={`${item.hour}:00 · ${item.count}`} />)}</div><small>00:00 — 23:00</small></section>
      </div>

      <section className="network-profile-panel">
        <header><div><h3>{t("Pseudonymous network profiles")}</h3><p>{t("Raw SSID, gateway and addresses are discarded from the profile identity; names are local and optional.")}</p></div><span>{profiles.length}</span></header>
        <div className="network-profile-list">
          {profiles.map((profile) => (
            <article key={profile.id}>
              <code>{profile.id}</code>
              <input aria-label={t("Local name for {id}", { id: profile.id })} defaultValue={profile.display_name ?? ""} onBlur={(event) => void rename(profile, event.target.value)} />
              <span>{profile.vpn_observed ? t("VPN observed") : t("Direct network observed")}</span>
              <small>{t("Created {date}", { date: formatDateTime(profile.created_at_ms) })}</small>
            </article>
          ))}
        </div>
        <div className="network-profile-compare">
          <select aria-label={t("Earlier network profile")} onChange={(event) => setLeftProfile(event.target.value)} value={leftProfile}>{profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.display_name ?? profile.id}</option>)}</select>
          <span>↔</span>
          <select aria-label={t("Later network profile")} onChange={(event) => setRightProfile(event.target.value)} value={rightProfile}>{profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.display_name ?? profile.id}</option>)}</select>
          {left && right && <p>{left.vpn_observed === right.vpn_observed ? t("Both profiles have the same observed VPN flag.") : t("The observed VPN flag changed between these profiles.")}</p>}
        </div>
        <aside>{t("Current v5 history does not retroactively assign a network profile to legacy connection rows. Comparisons remain scoped to captured profile evidence and never guess an association.")}</aside>
      </section>
    </section>
  );
}
