import { useEffect, useMemo, useState } from "react";
import { desktopApi } from "#backend";
import { useI18n } from "../i18n";
import type { Connection, LocalNeighbor } from "../types";
import { Icon } from "./Icon";
import { endpointLabel } from "../lib/format";
import { usePresentation } from "../privacy/PresentationMode";

interface LocalPeersProps {
  connections: Connection[];
  onSelect: (id: string) => void;
  selectedId: string | null;
}

interface Device {
  key: string;
  mac: string | null;
  hostname: string | null;
  addresses: Set<string>;
  interfaces: Set<string>;
  processes: Set<string>;
  ports: Set<number>;
  connectionCount: number;
  connectionIds: Set<string>;
  activeCount: number;
  isRouter: boolean;
  isOwnDevice: boolean;
  lastSeen: number | null;
  representativeId: string | null;
  selected: boolean;
}

function createDevice(key: string, mac: string | null): Device {
  return {
    key,
    mac,
    hostname: null,
    addresses: new Set(),
    interfaces: new Set(),
    processes: new Set(),
    ports: new Set(),
    connectionCount: 0,
    connectionIds: new Set(),
    activeCount: 0,
    isRouter: false,
    isOwnDevice: false,
    lastSeen: null,
    representativeId: null,
    selected: false,
  };
}

export function groupDevices(
  connections: Connection[],
  neighbors: LocalNeighbor[],
  selectedId: string | null,
): Device[] {
  const devices = new Map<string, Device>();
  const byAddress = new Map<string, Set<string>>();
  for (const neighbor of neighbors) {
    const key = neighbor.mac_address
      ? `mac:${neighbor.interface_id}:${neighbor.mac_address}`
      : `address:${neighbor.interface_id}:${neighbor.address}`;
    let device = devices.get(key);
    if (!device) {
      device = createDevice(key, neighbor.mac_address);
      devices.set(key, device);
    }
    device.addresses.add(neighbor.address);
    if (neighbor.interface_id) device.interfaces.add(neighbor.interface_id);
    device.isRouter ||= neighbor.is_router;
    const candidates = byAddress.get(neighbor.address) ?? new Set<string>();
    candidates.add(key);
    byAddress.set(neighbor.address, candidates);
  }

  for (const connection of connections) {
    const remote = connection.key.remote;
    const candidates = byAddress.get(remote.address);
    const key =
      remote.scope === "loopback"
        ? "self"
        : candidates?.size === 1
          ? [...candidates][0]!
          : `observed:${remote.address}%${remote.zone ?? ""}`;
    let device = devices.get(key);
    if (!device) {
      device = createDevice(key, null);
      devices.set(key, device);
    }
    device.addresses.add(remote.address);
    device.hostname ||=
      connection.hostname && connection.hostname !== remote.address
        ? connection.hostname
        : null;
    if (connection.interface) device.interfaces.add(connection.interface);
    if (connection.process.name) device.processes.add(connection.process.name);
    if (remote.port) device.ports.add(remote.port);
    device.connectionCount += 1;
    device.connectionIds.add(connection.id);
    device.activeCount += Number(connection.active);
    device.isOwnDevice ||= remote.scope === "loopback";
    device.selected ||= connection.id === selectedId;
    if (
      device.lastSeen === null ||
      connection.last_seen_ms >= device.lastSeen
    ) {
      device.lastSeen = connection.last_seen_ms;
      device.representativeId = connection.id;
    }
  }

  return [...devices.values()].sort(
    (left, right) =>
      Number(right.activeCount > 0) - Number(left.activeCount > 0) ||
      Number(right.isRouter) - Number(left.isRouter) ||
      Number(right.connectionCount > 0) - Number(left.connectionCount > 0) ||
      (right.lastSeen ?? 0) - (left.lastSeen ?? 0) ||
      [...left.addresses][0]!.localeCompare([...right.addresses][0]!),
  );
}

export function LocalPeers({
  connections,
  onSelect,
  selectedId,
}: LocalPeersProps) {
  const t = useI18n();
  const [neighbors, setNeighbors] = useState<LocalNeighbor[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const [deviceKey, setDeviceKey] = useState<string | null>(null);
  const { display } = usePresentation();

  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    const refresh = async () => {
      if (document.visibilityState !== "visible") {
        timer = window.setTimeout(() => void refresh(), 20_000);
        return;
      }
      setLoading(true);
      try {
        const entries = await desktopApi.listLocalNeighbors();
        if (!cancelled) {
          setNeighbors(entries);
          setError(null);
        }
      } catch (cause) {
        if (!cancelled) {
          setNeighbors([]);
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
          timer = window.setTimeout(() => void refresh(), 20_000);
        }
      }
    };
    void refresh();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [refreshToken]);

  const devices = useMemo(
    () => groupDevices(connections, neighbors, selectedId),
    [connections, neighbors, selectedId],
  );
  const identified = devices.filter((device) => device.mac !== null);
  const unverified = devices.filter((device) => device.mac === null);
  const selectedDevice =
    devices.find((device) => device.key === deviceKey) ??
    devices.find((device) => device.selected) ??
    devices[0];
  const interfaceGroups = new Map<string, Device[]>();
  for (const device of identified.slice(0, 100)) {
    for (const networkInterface of device.interfaces.size
      ? device.interfaces
      : [t("Unknown interface")]) {
      const group = interfaceGroups.get(networkInterface) ?? [];
      group.push(device);
      interfaceGroups.set(networkInterface, group);
    }
  }

  const deviceLabel = (device: Device) =>
    device.hostname ??
    (device.isOwnDevice
      ? t("This device")
      : device.isRouter
        ? t("Router")
        : device.mac
          ? `${t("Device")} · ${device.mac.slice(-5)}`
          : ([...device.addresses][0] ?? t("Device")));

  const renderDevice = (device: Device) => {
    const addresses = [...device.addresses].sort();
    const label =
      device.hostname ??
      (device.isOwnDevice
        ? t("This device")
        : device.isRouter
          ? t("Router")
          : device.mac
            ? `${t("Device")} · ${device.mac.slice(-5)}`
            : addresses[0]);
    const content = (
      <>
        <Icon name={device.isRouter ? "process" : "terminal"} />
        <span className="local-peers__top">
          <strong>{label}</strong>
          <small>
            {device.activeCount > 0
              ? t("Active")
              : device.connectionCount > 0
                ? t("Recently observed")
                : t("Neighbor cache")}
          </small>
        </span>
        <span className="local-peers__addresses">
          {addresses.map((address) => (
            <code key={address}>{display("local_ip", address)}</code>
          ))}
        </span>
      </>
    );
    return (
      <button
        aria-pressed={selectedDevice?.key === device.key}
        className="local-peers__item"
        key={device.key}
        onClick={() => setDeviceKey(device.key)}
        type="button"
      >
        {content}
      </button>
    );
  };

  return (
    <section aria-label={t("Local devices")} className="local-peers">
      <div className="local-peers__intro">
        <div className="local-peers__heading">
          <h1>{t("Local devices")}</h1>
          <span>{t("{count} device groups", { count: devices.length })}</span>
          <button
            className="button button--secondary"
            disabled={loading}
            onClick={() => setRefreshToken((value) => value + 1)}
            type="button"
          >
            {t(loading ? "Checking devices..." : "Refresh devices")}
          </button>
        </div>
        <p>
          {t(
            "Devices are grouped only when the operating system has a link address for them. The list uses existing neighbor and connection records; it does not scan the network.",
          )}
        </p>
        {error && (
          <p className="local-peers__error" role="status">
            {t("Neighbor cache unavailable")}: {error}.{" "}
            {t("Observed connections remain listed below.")}
          </p>
        )}
      </div>
      {devices.length === 0 ? (
        <p className="local-peers__empty">
          {t(
            "No local devices are known yet. Start live capture or communicate with a device on this network.",
          )}
        </p>
      ) : (
        <div className="local-peers__workspace">
          <div className="local-peers__scroll">
            {identified.length > 0 && (
              <section aria-label={t("Devices with MAC evidence")}>
                <h3>
                  {t("Devices with MAC evidence")}{" "}
                  <small>{identified.length}</small>
                </h3>
                {[...interfaceGroups].map(([networkInterface, entries]) => (
                  <section className="device-interface" key={networkInterface}>
                    <h4>
                      {t("Interface")} {display("interface", networkInterface)}
                    </h4>
                    <div className="local-peers__list">
                      {entries.map(renderDevice)}
                    </div>
                  </section>
                ))}
              </section>
            )}
            {unverified.length > 0 && (
              <section aria-label={t("Addresses without device identity")}>
                <h3>
                  {t("Addresses without device identity")}{" "}
                  <small>{unverified.length}</small>
                </h3>
                <div className="local-peers__list">
                  {unverified
                    .slice(0, Math.max(0, 100 - identified.length))
                    .map(renderDevice)}
                </div>
              </section>
            )}
            {devices.length > 100 && (
              <p className="local-peers__more">
                {t("Showing the 100 most recent peers of {count}.", {
                  count: devices.length,
                })}
              </p>
            )}
          </div>
          {selectedDevice && (
            <aside className="device-inspector" aria-label={t("Device")}>
              <header>
                <Icon name={selectedDevice.isRouter ? "process" : "terminal"} />
                <h2>{deviceLabel(selectedDevice)}</h2>
              </header>
              <p>
                {t(
                  selectedDevice.mac
                    ? "Grouped by MAC address"
                    : "Address only — device unverified",
                )}
              </p>
              <dl>
                <div>
                  <dt>{t("IP address")}</dt>
                  <dd>
                    {[...selectedDevice.addresses].map((address) => (
                      <code key={address}>{display("local_ip", address)}</code>
                    ))}
                  </dd>
                </div>
                <div>
                  <dt>{t("MAC address")}</dt>
                  <dd>
                    <code>
                      {selectedDevice.mac
                        ? display("local_ip", selectedDevice.mac)
                        : t("Unavailable")}
                    </code>
                  </dd>
                </div>
                <div>
                  <dt>{t("Interface")}</dt>
                  <dd>
                    {[...selectedDevice.interfaces]
                      .map((item) => display("interface", item))
                      .join(", ") || t("Unknown interface")}
                  </dd>
                </div>
              </dl>
              <h3>
                {t("Live connections")}{" "}
                <span>{selectedDevice.connectionCount}</span>
              </h3>
              {selectedDevice.connectionCount === 0 ? (
                <div className="today-empty">
                  <Icon name="activity" />
                  <strong>
                    {t("No connection from this computer observed")}
                  </strong>
                </div>
              ) : (
                <div className="device-connections">
                  {connections
                    .filter((connection) =>
                      selectedDevice.connectionIds.has(connection.id),
                    )
                    .slice(0, 100)
                    .map((connection) => (
                      <button
                        type="button"
                        key={connection.id}
                        onClick={() => onSelect(connection.id)}
                      >
                        <strong>
                          {display("process", connection.process.name)}
                        </strong>
                        <code>
                          {display(
                            "local_ip",
                            endpointLabel(
                              connection.key.remote.address,
                              connection.key.remote.port,
                            ),
                          )}
                        </code>
                        <small>
                          {connection.protocol.toUpperCase()} ·{" "}
                          {t(connection.active ? "Active" : "closed")}
                        </small>
                      </button>
                    ))}
                </div>
              )}
            </aside>
          )}
        </div>
      )}
    </section>
  );
}
