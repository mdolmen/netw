Block network connection
------------------------

Approach from `bcc/examples/networking/net_monitor.py`:
* attach a raw socket to the interface on which to filter packets
* the filter is linked to that socket
* we can drop packet from there(?)

Check this out to block based on the domain name:
* https://dnsdist.org/advanced/ebpf.html

Clarifications about firewalling under Linux:
* `netfilter` is the packet filtering framework inside the kernel
* `iptables` is one module of netfilter (ip6tables, arptables are among the
others)

Which brings me to the next reflection: **Is it necessary to provide the feature
of blocking connections from my tool?**
I think not. If there is already a piece of software doing that work then we
should delegate that functionnality to it.
* It probably does it better
* It avoid confusion (having 2 different tools blocking different connections)
* It simplifies the tool

What about the portability? It would be nice to design it from ground zero to be
portable in the future. It could be very interesting to run on Windows or other
closed/exotic OSes to see what really happens. It won't be based on eBPF (except
for the UNIX-based) so it would require to be modular. The blocking feature
could be part of that module.

Worth mentionning that the blocking I'm talking about here concerns the one
based on IP addresses (e.g. one of the connections from a specific process).

Blocking based on the domain name is another challenge. BPF seems the best way
to do that (cf. link above). There is dnsmasq but it only redirect the domain
to a given address, it does not block the traffic of the corresponding IP. Could
be used in a first version but would be more robust with BPF.

**DO NOT FORGET THAT THE MAIN GOAL OF THIS TOOL IS REAL-TIME VISIBILITY OF
APPLICATIONS COMMUNICATIONS WITH THE OUTSIDE WORLD! THEN EXTEND TO BLOCK A WHOLE
PROCESS.**

Current approach for blocking feature:
* BPF filter to block a given PID (attached to the interface net_monitor.py
does)
* Export IP addresses/port to an external firewall to add a rule
* BPF filter to block based on the domain name (cf. dnsdist)

No filtering in kernel, only gather info and block from userspace??
* you can still see if the userspace limitation is bypassed, thus detecting
malicious activity by the process

The big challenge with doing filtering from the kernel is how to filter multiple
PID? How do you pass a list of PID to the BPF code? You need to update that list
frequently. Can use maps but what about the overhead of going over it every
time?

cf. `bcc/examples/networking/dns_matching`

Could be done effectively after all. To investigate. Probably another tool
blocking a list of domain. This list would be filled by Sekhmet.

Looked at `firejail`. uses seccomp-bpf, namespaces and capabilities. NOT THE
PRIORITY. STAY FOCUSED ON THE REST FOR NOW!


Watch which process access sensitive files
------------------------------------------

Can be done with tracepoints. cf. `opensnoop.py`.

Comparaison to list of files to watch in userspace. BPF used on ly to gather the
info. Not resilient against malware, not the goal. Make another app for that

Comparaison to list of files to watch in userspace. BPF used on ly to gather the
info. Not resilient against malware, not the goal. Make another app for that.

Explore the possibility of making "service VM" with Boxy hypervisor.

**THE SAME GOES FOR THE NETWORK PART**


UDP
---

Function `tcp_read_sock()` (cf. comments in tcptop.py) can be found in the
`proto_ops` struct (`inet_stream_ops`)
Source: `net/ipv4/af_inet.c`

Quite obvious that `tcp_sendmsg` is not the only path:
> /*
>  * This routine provides an alternative to tcp_recvmsg() for routines
>  * that would like to handle copying from skbuffs directly in 'sendfile'
>  * fashion.
> ...
> int tcp_read_sock(...)

No such things for UDP (`inet_dgram_ops`) AFAIK.

Need to clarify the "trace both entry and return". Why??

Got it: if we trace only the entry we don't know if the data was actually
consumed or if there was an error.

* `tcp_recvmsg()`: `tcp_cleanup_rbuf()` is called at the end just before
releasing the socket
* there is a call to `tcp_recv_urg` at the end, which does not seems to be
covered bu `cleanup_rubf`, could lose something here??
* same for `tcp_peek_sndq`, are the data sent to userspace at some point?
* the comment on top of `cleanup_rbuf` let me think that all data sent to
userspace go through here

4 protocols for `AF_INET`: TCP, UPD, ICMP and RAW.

`udp_sendmsg` -> `udp_send_skb`

Some good resources on a packet flow:
https://blog.packagecloud.io/eng/2017/02/06/monitoring-tuning-linux-networking-stack-sending-data/
https://blog.packagecloud.io/eng/2016/06/22/monitoring-tuning-linux-networking-stack-receiving-data/#udp-protocol-layer
https://blog.packagecloud.io/eng/2016/10/11/monitoring-tuning-linux-networking-stack-receiving-data-illustrated/

Let's go for tracing the obvious `udp_sendmsg` and  `udp_recvmsg`!


TCP vs UDP
----------

IMPORTANT: Look at the `proto` struct!

net/ipv4/tcp.c
net/ipv6/tcp_ipv6.c
net/ipv4/udp.c
net/ipv6/udp.c

TCP:
* operation sendmsg has the same function `tcp_sendmsg()` as handler for both
V4 and V6, defined in `include/net/tcp.h`

UDP:
* operation sendmsg has `udp_sendmsg()` as handler for V4 and `udpv6_sendmsg()`
for V6, defined in `net/ipv6/udp_impl.h`...
* same for recvmsg


Cache BPF bytecode
------------------

One key aspect that could be improved in this program is the loading time. BPF
compilation take quite some time (a few seconds). It is done at every execution
of the program. This delay is very annoying if we want to start at boot itme
since it will slow down significantly.

It is not necessary to recompile the code every time. It is only mandatory when
the underlying kernel has changed.

Also it is a relatively small piece of code that we have to compile. I believe
this caching feature could be useful in other places as well. There is also the
question of portability that will probably change the way to do things again
(BTF, CO-RE).

Actually may be better not to waste time on this, focus on the user-space stuff
at first, and then switch to libbpf-rs directly, which uses BTF and CO-RE.
