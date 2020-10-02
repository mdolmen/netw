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
BPF_PERF_OUTPUT(ipv4_tcp_data);

struct ipv6_data_t {
    unsigned __int128 saddr;
    unsigned __int128 daddr;
    u32 pid;
    u16 lport;
    u16 dport;
    u32 size;
    u32 is_rx;
};
BPF_PERF_OUTPUT(ipv6_tcp_data);

int kprobe__tcp_sendmsg(struct pt_regs *ctx, struct sock *sk,
    struct msghdr *msg, size_t size)
{
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    u16 dport = 0, family = sk->__sk_common.skc_family;

    if (family == AF_INET) {
        struct ipv4_data_t ipv4_tcp = {.pid = pid};

        ipv4_tcp.saddr = sk->__sk_common.skc_rcv_saddr;
        ipv4_tcp.daddr = sk->__sk_common.skc_daddr;
        ipv4_tcp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv4_tcp.dport = ntohs(dport);
        ipv4_tcp.size = size;
	ipv4_tcp.is_rx = 0;
	
	ipv4_tcp_data.perf_submit(ctx, &ipv4_tcp, sizeof(ipv4_tcp));

    } else if (family == AF_INET6) {
        struct ipv6_data_t ipv6_tcp = {.pid = pid};

        bpf_probe_read(&ipv6_tcp.saddr, sizeof(ipv6_tcp.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&ipv6_tcp.daddr, sizeof(ipv6_tcp.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        ipv6_tcp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv6_tcp.dport = ntohs(dport);
        ipv6_tcp.size = size;
	ipv6_tcp.is_rx = 0;

	ipv6_tcp_data.perf_submit(ctx, &ipv6_tcp, sizeof(ipv6_tcp));
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
        struct ipv4_data_t ipv4_tcp = {.pid = pid};

        ipv4_tcp.saddr = sk->__sk_common.skc_rcv_saddr;
        ipv4_tcp.daddr = sk->__sk_common.skc_daddr;
        ipv4_tcp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv4_tcp.dport = ntohs(dport);
        ipv4_tcp.size = copied;
	ipv4_tcp.is_rx = 1;

	ipv4_tcp_data.perf_submit(ctx, &ipv4_tcp, sizeof(ipv4_tcp));

    } else if (family == AF_INET6) {
        struct ipv6_data_t ipv6_tcp = {.pid = pid};

        bpf_probe_read(&ipv6_tcp.saddr, sizeof(ipv6_tcp.saddr),
            &sk->__sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
        bpf_probe_read(&ipv6_tcp.daddr, sizeof(ipv6_tcp.daddr),
            &sk->__sk_common.skc_v6_daddr.in6_u.u6_addr32);
        ipv6_tcp.lport = sk->__sk_common.skc_num;
        dport = sk->__sk_common.skc_dport;
        ipv6_tcp.dport = ntohs(dport);
        ipv6_tcp.size = copied;
	ipv6_tcp.is_rx = 1;

	ipv6_tcp_data.perf_submit(ctx, &ipv6_tcp, sizeof(ipv6_tcp));
    }
    // else drop

    return 0;
}
