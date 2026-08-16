# Privacy

## Plain-language promise

**Everything is processed locally by default.** Normal socket observation, aggregation,
GeoIP lookup, history, baselines, search and visualization do not send metadata to
Sabicheiro or any commercial service. The two explicit network-capable options—GeoIP
dataset download and system reverse DNS—are described below.

## What is observed

- local and remote IP addresses and ports;
- TCP/UDP protocol, direction and connection state;
- PID, process name and executable path when the OS permits it;
- timestamps, duration and available byte/packet counters;
- interface/route identity and locally derived DNS/GeoIP/ASN metadata;
- optional local executable identity (revision, SHA-256 and signature/package status)
  only when the user requests inspection.

## What is stored

By default Sabicheiro retains immutable evidence observations, projected connection
metadata, one-second traffic
samples and session summaries for seven days in a local SQLite database. Retention can
be Off, 1, 7, 30, 90 days or Unlimited. Bookmarks, rules, annotations, notification
state, network-profile pseudonyms and settings are also local. Raw SSID, gateway and
interface addresses are discarded when creating a network-profile identity. The database lives in
the platform application-data directory:

- Windows: `%APPDATA%\io.sabicheiro.desktop\` (resolved by Tauri).
- Linux: `$XDG_DATA_HOME/io.sabicheiro.desktop/` or the platform fallback.

**Ephemeral capture** can be enabled before starting a session. In that mode new live
connection history, session metadata and behavioural baselines remain in memory and
are not written to SQLite. Separate controls delete retained sessions, GeoIP files,
DNS cache, local logs, or all local application data; destructive choices require a
confirmation and capture must be stopped where database safety requires it.

Database retention never deletes user-created CSV/JSON/PCAPNG files; users manage those
files independently. Relative export names are scoped beneath the app-data `exports`
directory, while absolute paths must point to an existing directory.

Structured application logs are written below the same app-data directory, rotate
daily and are pruned after seven days. The implemented log events contain lifecycle
and error metadata, never packet payloads or credentials.

## Payloads and recordings

Normal monitoring stores metadata, not packet payload. Full packet content is written
only after the user explicitly selects **Start Recording**, chooses a destination and
accepts the permission requirement. Recording state, duration and size remain visible.
PCAP/PCAPNG files may contain sensitive data and are never uploaded automatically.

Optional handshake metadata is a separate mode, disabled by default and visibly
indicated while active. It is limited to bounded TLS/QUIC identification fields such as
version, SNI and ALPN when observable. Buffers have byte/time budgets, are zeroized on
clear/drop and are not written as payload, keys or decrypted content. The mode's local
start/stop audit contains settings and timestamps, not packet bytes. Sabicheiro does
not perform MITM, install certificates or decrypt sessions.

## Network access by Sabicheiro

Core monitoring requires no Sabicheiro service. Optional **Update GeoIP database**
downloads the public monthly DB-IP Lite archives directly after user action. It gets
the complete free dataset, never performs an IP-by-IP API query, and requires no
account, key or subscription. No
observed IP is sent to DB-IP; whole databases are downloaded and subsequent lookups
remain offline. A failed update keeps the previous valid file.

Both optional paths go through a typed local network-action broker. A grant binds the
intent, fixed destination class, explicit consent identifier and response-size limit;
each allowed/denied action is recorded locally with redacted detail. **Offline
enforced** rejects every optional network grant even if another setting is enabled.

**System reverse DNS lookups are disabled by default.** If the user enables them in
Privacy settings, Sabicheiro asks the operating system resolver for PTR names of public
remote IPs. Depending on the machine's DNS configuration, those IPs may be visible to
a router, ISP or chosen DNS provider. Results and negative results are cached locally;
private/loopback/multicast addresses are never submitted by this worker.

When Windows permits the optional DNS Client ETW session, already-observed local DNS
answers can be correlated without making any new lookup. ETW queues are bounded and do
not retain packet payloads.

## Explicit non-features

There is no telemetry, crash-upload service, analytics SDK, advertising identifier,
account, login, subscription, cloud sync, remote database, AI model, paid API, threat
lookup API, MITM, TLS decryption or credential interception. Logs omit payloads and
credentials and rotate locally.

## Exports

CSV, JSON, redacted support ZIP and PCAPNG exports occur only after an explicit action
at a validated path.
Metadata export can target the active query, time range or session. Sabicheiro does not
open or upload the result automatically and refuses to overwrite an existing packet
recording. The support bundle uses an explicit allow-list and excludes connection/raw
IP history, packet payloads, recordings and secrets. Rotated packet captures are
bounded to four files.

The `.sabi` investigation format is a versioned ZIP container with an exact bounded
entry set, manifest hashes and CRC checks. Redaction can remove local IPs, users, paths
and sensitive names before export. Imports open in a separate read-only workspace and
are never merged into local history or uploaded automatically. A modified byte causes
verification failure.
