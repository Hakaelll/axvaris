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
- interface identity and locally derived DNS/GeoIP/ASN metadata.

## What is stored

By default Sabicheiro retains aggregated connection metadata and session summaries for
seven days in a local SQLite database. Retention can be Off, 1, 7, 30, 90 days or
Unlimited. Bookmarks, ignore rules and settings are also local. The database lives in
the platform application-data directory:

- Windows: `%APPDATA%\io.sabicheiro.desktop\` (resolved by Tauri).
- Linux: `$XDG_DATA_HOME/io.sabicheiro.desktop/` or the platform fallback.

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

## Network access by Sabicheiro

Core monitoring requires no Sabicheiro service. Optional **Update GeoIP database**
downloads the public monthly DB-IP Lite archives directly after user action. No
observed IP is sent to DB-IP; whole databases are downloaded and subsequent lookups
remain offline. A failed update keeps the previous valid file.

**System reverse DNS lookups are disabled by default.** If the user enables them in
Privacy settings, Sabicheiro asks the operating system resolver for PTR names of public
remote IPs. Depending on the machine's DNS configuration, those IPs may be visible to
a router, ISP or chosen DNS provider. Results and negative results are cached locally;
private/loopback/multicast addresses are never submitted by this worker.

## Explicit non-features

There is no telemetry, crash-upload service, analytics SDK, advertising identifier,
account, login, cloud sync, remote database, AI model, threat lookup API, MITM, TLS
decryption or credential interception. Logs omit payloads and credentials and rotate
locally.

## Exports

CSV, JSON and PCAPNG exports occur only after an explicit action at a validated path.
Metadata export can target the active query, time range or session. Sabicheiro does not
open or upload the result automatically and refuses to overwrite an existing packet
recording. Rotated packet captures are bounded to four files.
