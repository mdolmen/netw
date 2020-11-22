/*
 * Adapted from BCC example:
 *	https://github.com/iovisor/bcc/blob/master/tools/tcptop.py
 */

#include <uapi/linux/ptrace.h>
#include <net/sock.h>
#include <bcc/proto.h>

struct ipv4_data_t {
    u32 pid;
    u32 saddr;
    u32 daddr;
    u16 lport;
    u16 dport;
    u32 size;
    u32 is_rx;
};

struct ipv6_data_t {
    unsigned __int128 saddr;
    unsigned __int128 daddr;
    u32 pid;
    u16 lport;
    u16 dport;
    u32 size;
    u32 is_rx;
};

BPF_PERF_OUTPUT(tcp4_data);
BPF_PERF_OUTPUT(udp4_data);
BPF_PERF_OUTPUT(tcp6_data);
BPF_PERF_OUTPUT(udp6_data);

int kprobe__tcp_sendmsg(struct pt_regs *ctx, struct sock *sk,
    struct msghdr *msg, size_t size)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (family == AF_INET) {
        struct ipv4_data_t tcp4 = {.pid = pid};

        tcp4.saddr = sk->__sk_common.skc_rcv_saddr;
        tcp4.daddr = sk->__sk_common.skc_daddr;
        tcp4.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        tcp4.dport = ntohs(dport);
        tcp4.size = size;
	tcp4.is_rx = 0;
	
	tcp4_data.perf_submit(ctx, &tcp4, sizeof(tcp4));

    } else if (family == AF_INET6) {
        struct ipv6_data_t tcp6 = {.pid = pid};

        bpf_probe_read(&tcp6.saddr, sizeof(tcp6.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&tcp6.daddr, sizeof(tcp6.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        tcp6.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        tcp6.dport = ntohs(dport);
        tcp6.size = size;
	tcp6.is_rx = 0;

	tcp6_data.perf_submit(ctx, &tcp6, sizeof(tcp6));
    }
    // else drop

    return 0;
}

/*
 * tcp_recvmsg() would be obvious to trace, but is less suitable because:
 * - we'd need to trace both entry and return, to have both sock and size
 * - misses tcp_read_sock() traffic
 * we'd much prefer tracepoints once they are available.
 */
int kprobe__tcp_cleanup_rbuf(struct pt_regs *ctx, struct sock *sk, int copied)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;
    u64 *val, zero = 0;

    if (copied <= 0)
        return 0;

    if (family == AF_INET) {
        struct ipv4_data_t tcp4 = {.pid = pid};

        tcp4.saddr = sk->__sk_common.skc_rcv_saddr;
        tcp4.daddr = sk->__sk_common.skc_daddr;
        tcp4.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        tcp4.dport = ntohs(dport);
        tcp4.size = copied;
	tcp4.is_rx = 1;

	tcp4_data.perf_submit(ctx, &tcp4, sizeof(tcp4));

    } else if (family == AF_INET6) {
        struct ipv6_data_t tcp6 = {.pid = pid};

        bpf_probe_read(&tcp6.saddr, sizeof(tcp6.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&tcp6.daddr, sizeof(tcp6.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        tcp6.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        tcp6.dport = ntohs(dport);
        tcp6.size = copied;
	tcp6.is_rx = 1;

	tcp6_data.perf_submit(ctx, &tcp6, sizeof(tcp6));
    }
    // else drop

    return 0;
}

/*
 * "size_t len" instead of "size_t size" to match with udp_sendmsg() arg name.
 * The struct field does not change.
 *
 */
int kprobe__udp_sendmsg(struct pt_regs *ctx, struct sock *sk,
    struct msghdr *msg, size_t len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (family == AF_INET) {
        struct ipv4_data_t udp4 = {.pid = pid};

        udp4.saddr = sk->__sk_common.skc_rcv_saddr;
        udp4.daddr = sk->__sk_common.skc_daddr;
        udp4.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        udp4.dport = ntohs(dport);
        udp4.size = len;
	udp4.is_rx = 0;
	
	udp4_data.perf_submit(ctx, &udp4, sizeof(udp4));

    }
    // else drop

    return 0;
}

int kprobe__udp_recvmsg(struct pt_regs *ctx, struct sock *sk, struct msghdr *msg,
    size_t len, int noblock, int flags, int *addr_len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (len <= 0)
        return 0;

    if (family == AF_INET) {
        struct ipv4_data_t udp4 = {.pid = pid};

        udp4.saddr = sk->__sk_common.skc_rcv_saddr;
        udp4.daddr = sk->__sk_common.skc_daddr;
        udp4.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        udp4.dport = ntohs(dport);
        udp4.size = len;
	udp4.is_rx = 1;

	udp4_data.perf_submit(ctx, &udp4, sizeof(udp4));

    }
    // else drop

    return 0;
}

int kprobe__udpv6_sendmsg(struct pt_regs *ctx, struct sock *sk,
    struct msghdr *msg, size_t len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (len <= 0)
        return 0;

    if (family == AF_INET6) {
        struct ipv6_data_t udp6 = {.pid = pid};

        bpf_probe_read(&udp6.saddr, sizeof(udp6.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&udp6.daddr, sizeof(udp6.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        udp6.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        udp6.dport = ntohs(dport);
        udp6.size = len;
	udp6.is_rx = 0;

	udp6_data.perf_submit(ctx, &udp6, sizeof(udp6));
    }
    // else drop

    return 0;
}

int kprobe__udpv6_recvmsg(struct pt_regs *ctx, struct sock *sk, struct msghdr *msg,
    size_t len, int noblock, int flags, int *addr_len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (len <= 0)
        return 0;

    if (family == AF_INET6) {
        struct ipv6_data_t udp6 = {.pid = pid};

        bpf_probe_read(&udp6.saddr, sizeof(udp6.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&udp6.daddr, sizeof(udp6.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        udp6.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        udp6.dport = ntohs(dport);
        udp6.size = len;
	udp6.is_rx = 1;

	udp6_data.perf_submit(ctx, &udp6, sizeof(udp6));
    }
    // else drop

    return 0;
}
