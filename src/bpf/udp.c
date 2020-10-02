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
BPF_PERF_OUTPUT(ipv4_udp_data);

struct ipv6_data_t {
    unsigned __int128 saddr;
    unsigned __int128 daddr;
    u32 pid;
    u16 lport;
    u16 dport;
    u32 size;
    u32 is_rx;
};
BPF_PERF_OUTPUT(ipv6_udp_data);

/*
 * "size_t len" instead of "size_t size" to match with udp_sendmsg() arg name.
 * The struct field does not change.
 *
 * TODO: actually test it but that should do the trick
 */
int kprobe__udp_sendmsg(struct pt_regs *ctx, struct sock *sk,
    struct msghdr *msg, size_t len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (family == AF_INET) {
        struct ipv4_data_t ipv4_udp = {.pid = pid};

        ipv4_udp.saddr = sk->__sk_common.skc_rcv_saddr;
        ipv4_udp.daddr = sk->__sk_common.skc_daddr;
        ipv4_udp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv4_udp.dport = ntohs(dport);
        ipv4_udp.size = len;
	ipv4_udp.is_rx = 0;
	
	ipv4_udp_data.perf_submit(ctx, &ipv4_udp, sizeof(ipv4_udp));

    } else if (family == AF_INET6) {
        struct ipv6_data_t ipv6_udp = {.pid = pid};

        bpf_probe_read(&ipv6_udp.saddr, sizeof(ipv6_udp.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&ipv6_udp.daddr, sizeof(ipv6_udp.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        ipv6_udp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv6_udp.dport = ntohs(dport);
        ipv6_udp.size = len;
	ipv6_udp.is_rx = 0;

	ipv6_udp_data.perf_submit(ctx, &ipv6_udp, sizeof(ipv6_udp));
    }
    // else drop

    return 0;
}

// TODO: change signature and function name
int kprobe__udp_recvmsg(struct pt_regs *ctx, struct sock *sk, struct msghdr *msg,
    size_t len, int noblock, int flags, int *addr_len)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;
    u64 *val, zero = 0;

    if (len <= 0)
        return 0;

    if (family == AF_INET) {
        struct ipv4_data_t ipv4_udp = {.pid = pid};

        ipv4_udp.saddr = sk->__sk_common.skc_rcv_saddr;
        ipv4_udp.daddr = sk->__sk_common.skc_daddr;
        ipv4_udp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv4_udp.dport = ntohs(dport);
        ipv4_udp.size = len;
	ipv4_udp.is_rx = 1;

	ipv4_udp_data.perf_submit(ctx, &ipv4_udp, sizeof(ipv4_udp));

    } else if (family == AF_INET6) {
        struct ipv6_data_t ipv6_udp = {.pid = pid};

        bpf_probe_read(&ipv6_udp.saddr, sizeof(ipv6_udp.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&ipv6_udp.daddr, sizeof(ipv6_udp.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        ipv6_udp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv6_udp.dport = ntohs(dport);
        ipv6_udp.size = len;
	ipv6_udp.is_rx = 1;

	ipv6_udp_data.perf_submit(ctx, &ipv6_udp, sizeof(ipv6_udp));
    }
    // else drop

    return 0;
}
