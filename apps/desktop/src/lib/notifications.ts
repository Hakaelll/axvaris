import type { AppSettings, Connection } from "../types";

export interface LocalNotification {
  id: string;
  title: string;
  detail: string;
  created_at_ms: number;
}

export interface NotificationTracking {
  seenConnectionIds: Set<string>;
  seenProcesses: Set<string>;
  seenCountries: Set<string>;
  cooldowns: Map<string, number>;
}

function isQuietHour(settings: AppSettings, now: number): boolean {
  const start = settings.notification_quiet_start_minute;
  const end = settings.notification_quiet_end_minute;
  if (
    !settings.notification_quiet_hours_enabled ||
    start > 1_439 ||
    end > 1_439
  )
    return false;
  const localTime = new Date(now);
  const minute = localTime.getHours() * 60 + localTime.getMinutes();
  return start <= end
    ? minute >= start && minute < end
    : minute >= start || minute < end;
}

export function processNotificationConnections(
  active: Connection[],
  settings: AppSettings,
  tracking: NotificationTracking,
  now: number,
): LocalNotification[] {
  const created: LocalNotification[] = [];
  const quiet = isQuietHour(settings, now);
  for (const connection of active) {
    if (tracking.seenConnectionIds.has(connection.id)) continue;
    const processKey = `${connection.process.name}:${connection.process.pid ?? "?"}`;
    const isNewProcess = !tracking.seenProcesses.has(processKey);
    const country = connection.geo?.country_code ?? null;
    const isNewCountry = Boolean(
      country && !tracking.seenCountries.has(country),
    );
    for (const rule of settings.notification_rules) {
      if (!rule.enabled || quiet) continue;
      const value = rule.value?.trim().toLowerCase() ?? "";
      const matches =
        (rule.kind === "new_process" && isNewProcess) ||
        (rule.kind === "new_country" && isNewCountry) ||
        (rule.kind === "port" &&
          String(connection.key.remote.port) === value) ||
        (rule.kind === "asn" &&
          Boolean(
            connection.asn &&
              (String(connection.asn.number) === value ||
                (connection.asn.organization ?? "")
                  .toLowerCase()
                  .includes(value)),
          ));
      const previous = tracking.cooldowns.get(rule.id) ?? 0;
      if (
        !matches ||
        now - previous < Math.max(5, rule.cooldown_seconds) * 1_000
      )
        continue;
      tracking.cooldowns.set(rule.id, now);
      const title =
        rule.kind === "new_process"
          ? "New network process"
          : rule.kind === "new_country"
            ? "New destination country"
            : rule.kind === "asn"
              ? "ASN rule matched"
              : "Port rule matched";
      const detail = `${connection.process.name} → ${connection.domain ?? connection.hostname ?? connection.key.remote.address}:${connection.key.remote.port}`;
      created.push({
        id: `${rule.id}-${connection.id}-${now}`,
        title,
        detail,
        created_at_ms: now,
      });
      if (
        typeof Notification !== "undefined" &&
        Notification.permission === "granted"
      ) {
        new Notification(`Axvaris / ${title}`, { body: detail });
      }
    }
    tracking.seenConnectionIds.add(connection.id);
    tracking.seenProcesses.add(processKey);
    if (country) tracking.seenCountries.add(country);
  }

  return created;
}
