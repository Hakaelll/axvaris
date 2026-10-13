#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

/* The loader checks tracefs offsets before attaching this stable tracepoint. */
struct socket_state {
    __u64 common;
    __u64 socket;
    __s32 old_state;
    __s32 new_state;
    __u16 local_port;
    __u16 remote_port;
    __u16 family;
    __u16 protocol;
    __u8 local4[4];
    __u8 remote4[4];
    __u8 local6[16];
    __u8 remote6[16];
};

struct owner { __u64 created_ns; __u32 pid; char name[16]; };
struct event {
    __u64 observed_ns;
    __u64 owner_ns;
    __u32 pid;
    __u32 bytes;
    __u32 interface_index;
    __u16 local_port;
    __u16 remote_port;
    __u8 family;
    __u8 protocol;
    __u8 state;
    __u8 incoming;
    __u8 local[16];
    __u8 remote[16];
    char name[16];
};
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 4 * 1024 * 1024);
} EVENTS SEC(".maps");
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, __u64);
    __type(value, struct owner);
} OWNERS SEC(".maps");
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} LOST SEC(".maps");
struct namespace_id { __u64 device; __u64 inode; };
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct namespace_id);
} PID_NAMESPACE SEC(".maps");

static __always_inline void lost(void) {
    __u32 key = 0;
    __u64 *count = bpf_map_lookup_elem(&LOST, &key);
    if (count) __sync_fetch_and_add(count, 1);
}

SEC("tracepoint/sock/inet_sock_set_state")
int tcp_state(struct socket_state *ctx) {
    if (ctx->protocol != 6 || (ctx->family != 2 && ctx->family != 10)) return 0;
    __u64 socket = ctx->socket;
    if (ctx->new_state == 2) {
        struct owner current = {};
        current.created_ns = bpf_ktime_get_ns();
        __u32 key = 0;
        struct namespace_id *namespace = bpf_map_lookup_elem(&PID_NAMESPACE, &key);
        struct bpf_pidns_info identity = {};
        if (namespace && !bpf_get_ns_current_pid_tgid(namespace->device, namespace->inode,
                                                     &identity, sizeof(identity)))
            current.pid = identity.tgid;
        bpf_get_current_comm(current.name, sizeof(current.name));
        if (bpf_map_update_elem(&OWNERS, &socket, &current, BPF_ANY)) lost();
    }
    struct owner *owner = bpf_map_lookup_elem(&OWNERS, &socket);
    struct event *record = bpf_ringbuf_reserve(&EVENTS, sizeof(*record), 0);
    if (!record) {
        lost();
        if (ctx->new_state == 6 || ctx->new_state == 7) bpf_map_delete_elem(&OWNERS, &socket);
        return 0;
    }
    __builtin_memset(record, 0, sizeof(*record));
    record->observed_ns = bpf_ktime_get_ns();
    record->family = ctx->family;
    record->protocol = 6;
    record->state = ctx->new_state;
    record->local_port = ctx->local_port;
    record->remote_port = ctx->remote_port;
    if (owner) {
        record->pid = owner->pid;
        record->owner_ns = owner->created_ns;
        __builtin_memcpy(record->name, owner->name, 16);
    }
    if (ctx->family == 2) {
        __builtin_memcpy(record->local, ctx->local4, 4);
        __builtin_memcpy(record->remote, ctx->remote4, 4);
    } else {
        __builtin_memcpy(record->local, ctx->local6, 16);
        __builtin_memcpy(record->remote, ctx->remote6, 16);
    }
    bpf_ringbuf_submit(record, 0);
    if (ctx->new_state == 6 || ctx->new_state == 7) bpf_map_delete_elem(&OWNERS, &socket);
    return 0;
}

static __always_inline int udp_packet(struct __sk_buff *skb, __u8 incoming) {
    __u8 header[48] = {};
    if (bpf_skb_load_bytes(skb, 0, header, 20)) return 1;
    __u8 version = header[0] >> 4;
    __u32 offset;
    if (version == 4) {
        if (header[9] != 17) return 1;
        /* Fragmented datagrams cannot provide a complete peer tuple. */
        if ((header[6] & 0x3f) || header[7]) { lost(); return 1; }
        offset = (header[0] & 15) * 4;
        if (offset < 20 || offset > 60) { lost(); return 1; }
    } else if (version == 6) {
        if (bpf_skb_load_bytes(skb, 0, header, 40)) { lost(); return 1; }
        /* Extension headers need a dedicated parser; never guess their ports. */
        if (header[6] != 17) return 1;
        offset = 40;
    } else return 1;
    __u8 udp[8] = {};
    if (bpf_skb_load_bytes(skb, offset, udp, sizeof(udp))) { lost(); return 1; }
    struct event *record = bpf_ringbuf_reserve(&EVENTS, sizeof(*record), 0);
    if (!record) { lost(); return 1; }
    __builtin_memset(record, 0, sizeof(*record));
    record->observed_ns = bpf_ktime_get_ns();
    record->family = version == 4 ? 2 : 10;
    record->protocol = 17;
    record->incoming = incoming;
    record->interface_index = skb->ifindex;
    __u16 source = ((__u16)udp[0] << 8) | udp[1];
    __u16 destination = ((__u16)udp[2] << 8) | udp[3];
    record->local_port = incoming ? destination : source;
    record->remote_port = incoming ? source : destination;
    __u16 length = ((__u16)udp[4] << 8) | udp[5];
    record->bytes = length > 8 ? length - 8 : 0;
    if (version == 4) {
        __builtin_memcpy(record->local, header + (incoming ? 16 : 12), 4);
        __builtin_memcpy(record->remote, header + (incoming ? 12 : 16), 4);
    } else {
        __builtin_memcpy(record->local, header + (incoming ? 24 : 8), 16);
        __builtin_memcpy(record->remote, header + (incoming ? 8 : 24), 16);
    }
    /* Packet hooks run in softirq too, so current PID is not a socket owner. */
    bpf_ringbuf_submit(record, 0);
    return 1;
}
SEC("cgroup_skb/egress") int udp_egress(struct __sk_buff *skb) { return udp_packet(skb, 0); }
SEC("cgroup_skb/ingress") int udp_ingress(struct __sk_buff *skb) { return udp_packet(skb, 1); }
char LICENSE[] SEC("license") = "Dual MIT/GPL";
