Check amount send/recv
----------------------

* `std::process::Command` to exec a script
	* not needed

* bash script to setup a net namespace with only 2 veth
* (ns) launch binary
* (ns) iperf3 to generate traffic
* (ns) quit program
* check that TX == sender and RX == receiver (output of iperf3), packet may be
  dropped (normal behavior)
* `jq` tool to parse json in bash

One bash script in build/ or tests/ to be run as a standalone, not part as the
unit testing

Interesting fact to note iperf3 on the loopback interface for ipv4: the receiver
does not actually receive the data through the same path as normal TCP traffic,
i.e. the BPF filter does not catch any packet (only for the sender). On the
other side, both sender/receiver packets are captured in ipv6. So
`tcp_cleanup_rbuf` is not part of the flow of a receiving packet on lo, which
seems logical since lo does not really receive any data.

Transfer 2G of data. It is high enough to test the behavior on a large amount of
packets as well as to be easily identifiable and low enough for iperf3 to send it
in one stream which is easier to get the sum from.

Note that there are BPF packet lost when sending very large amount of data,
like 10G. With such big amount iperf3 send via multiple streams (and multiple
processes) which is probably the cause of filling to fast the perf buffer.
Starting at 3G we start observing lost samples.
Even at 1G. Depends on the system state. Not really a problem right now since
there won't be that kind of throughput to intercept.

I use 500M of data. Usually it is OK... Sometimes I intercepted a little bit
more than what was actually sent by iperf. Not sure where this could come from.
