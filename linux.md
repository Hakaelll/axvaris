# Axvaris on Linux

Axvaris uses INET_DIAG socket snapshots without running the interface as root. The optional Aya helper adds TCP lifecycle events and UDP datagrams. Install and enable the helper separately. Normal capture continues when the helper is missing, lacks permissions or cannot attach to the running kernel.

## Build the desktop application

On Debian 12 or Ubuntu with WebKitGTK 4.1 available:

```sh
sudo apt update
sudo apt install build-essential pkg-config clang libbpf-dev iproute2 \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf file libcap2-bin
git clone https://github.com/Hakaelll/axvaris.git
cd axvaris
pnpm install --frozen-lockfile
pnpm check
pnpm tauri build
```

Install Rust 1.87 or later and the pnpm version pinned in `package.json` first. Clang compiles the embedded BPF object on Linux. An INET_DIAG-only capture build is available with `cargo build -p sabicheiro-capture --no-default-features`. The desktop retains `io.sabicheiro.desktop` and its existing data location.

## Enable local event capture

Build and install the fixed helper path as an administrator:

```sh
cargo build -p sabicheiro-capture --release --bin axvaris-capture-helper --locked
sudo install -D -m 0755 target/release/axvaris-capture-helper /usr/libexec/axvaris-capture-helper
sudo setcap cap_bpf,cap_perfmon,cap_net_admin,cap_dac_read_search=ep /usr/libexec/axvaris-capture-helper
```

The host needs a compatible kernel, mounted tracefs, cgroup v2, BPF support and permission to attach. The loader validates the TCP tracepoint layout before loading. File capabilities may be restricted by container, filesystem or security policy. Do not run the desktop as root to bypass those restrictions.

The helper accepts no arguments or executable/object paths. Its binary and parent directories must be root-owned and not writable by other users. After attaching, it drops all capabilities and enables `no_new_privs`. Closing its input releases its BPF links. An unavailable helper is retried every 30 seconds while live capture is active.

To disable event capture:

```sh
sudo setcap -r /usr/libexec/axvaris-capture-helper
```

## Observation limits

- Active TCP opens can carry process identity. Passive TCP and UDP owners remain explicitly unknown. Previously captured process metadata survives process exit.
- UDP counts cover payload bytes and datagrams; TCP byte counters are unavailable. UDP conversations close after 30 seconds of inactivity, not a native close event.
- Fragmented IPv4 and IPv6 extension headers are not decoded by the UDP provider.
- TCP tracepoints cover the host; UDP covers the attached cgroup subtree. Namespace visibility and `/proc` access also constrain snapshot ownership.
- Shared socket descriptors do not establish a unique process owner.

## Reproduce kernel validation in Docker

```sh
docker build -f scripts/linux-capture.Dockerfile -t axvaris-linux-capture .
docker run --rm -v "$PWD:/workspace" axvaris-linux-capture \
  cargo build -p sabicheiro-capture --bins --locked
docker run --rm --privileged --pid=host --cgroupns=host \
  -v "$PWD:/workspace" axvaris-linux-capture sh scripts/verify-linux-events.sh
```

Use a disposable container for the privileged fixture. It mounts tracefs and installs the helper inside that container. Results are written under `target/`. Raw reports include local process evidence and should be reviewed before sharing.

The controlled event fixture detected 18/18 expected flows with no false positives or incorrect process attributions on Docker's WSL2 kernel. This does not establish package installation, suspension/VPN recovery, assistive technology support or prolonged resource stability. See [the validation record](docs/AXVARIS_VALIDATION.md).
