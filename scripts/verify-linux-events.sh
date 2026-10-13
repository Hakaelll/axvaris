#!/bin/sh
set -eu
if [ ! -r /sys/kernel/tracing/events/sock/inet_sock_set_state/format ]; then
  mount -t tracefs tracefs /sys/kernel/tracing
fi
install -D -m 0755 target/linux-container/debug/axvaris-capture-helper /usr/libexec/axvaris-capture-helper
target/linux-container/debug/capture-fixture --provider linux-ebpf --poll-ms 10 --report target/axvaris-linux-events.json
