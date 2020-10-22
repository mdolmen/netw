#!/bin/bash

#
# Setup a network namespace with two virtual interfaces and then uses iperf3 to
# simulate traffic.
# 
# The bytes sent/received are compared to what was intercepted by Sekhmet.
#
# Note that there are BPF packets lost when sending very large amount of data
# (+1G) in a short time window. I am usually fine with 1 or 2G but it depends on
# the system state. Not really a problem right now since there won't be that
# kind of throughput to intercept.
#

if [ "$EUID" -ne 0 ]
  then echo "Please run as root."
  exit
fi

# TODO
#if rpm -q $pkg
#then
#	echo "Install jq"
#fi

NC='\033[0m'
RED='\033[0;31m'
GREEN='\033[0;32m'

output0="sekhmet.json"
output_tcp4="iperf_tcp4.json"
output_tcp6="iperf_tcp6.json"

test_result () {
	rx_intercepted=$(cat $output0 | jq '.'$1'.rx')
	tx_intercepted=$(cat $output0 | jq '.'$1'.tx')
	rx_iperf=$(cat $2 | jq '.end.sum_received.bytes')
	tx_iperf=$(cat $2 | jq '.end.sum_sent.bytes')

	# cf. comments in src/net/mod.rs:log_iperf_to_file() for why +37
	if (( $rx_intercepted == $rx_iperf+37 )) && (( $tx_intercepted == $tx_iperf+37 ))
	then
		echo -e "[test] ${GREEN}$1 TCP: OK${NC}"
	else
		echo -e "[test] ${RED}$1 TCP: iperf3 traffic != traffic intercepted${NC}"
		echo "rx_intercepted: $rx_intercepted"
		echo "rx_iperf: $rx_iperf"
		echo "tx_intercepted: $tx_intercepted"
		echo "tx_iperf: $tx_iperf"
	fi
}

#
# Setup network namespace. Could be done without, just used as a convenience not
# to mess with the host network settings.
#

ns="test-traffic"
v0="veth0"
v1="veth1"

echo "[+] Setting up network environment..."
ip netns add $ns
ip netns exec $ns ip link add $v0 type veth peer name $v1
ip netns exec test-traffic ip addr add 10.0.10.100 dev $v0
ip netns exec test-traffic ip addr add 10.0.10.200 dev $v1
ip netns exec test-traffic ip link set dev $v0 up
ip netns exec test-traffic ip link set dev $v1 up
ip netns exec test-traffic ip link set dev lo up
v0_ip6=$(ip netns exec test-traffic ip addr show dev $v0 | sed -e's/^.*inet6 \([^ ]*\)\/.*$/\1/;t;d')
v1_ip6=$(ip netns exec test-traffic ip addr show dev $v1 | sed -e's/^.*inet6 \([^ ]*\)\/.*$/\1/;t;d')

#
# Generate traffic
#

echo "[+] Starting traffic interception..."
../target/debug/sekhmet test &
pid=$(echo $!)
# Let it compile BPF code
sleep 5


#
# IPV4 TCP
#
echo "[+] Simulating ipv4 TCP traffic..."
ip netns exec $ns iperf3 -s -B 10.0.10.100 -p 5201 -1 &> /dev/null &
sleep 2
ip netns exec $ns iperf3 -4 -c 10.0.10.100 -p 5201 -n 500M -J > $output_tcp4
sleep 5

# TODO: IPV4 UDP

#
# IPV6 TCP
#
echo "[+] Simulating ipv6 TCP traffic..."
ip netns exec $ns iperf3 -s -B $v0_ip6'%'$v0 -p 5201 -1 &> /dev/null &
sleep 2
ip netns exec $ns iperf3 -6 -c $v0_ip6'%'$v0 -p 5201 -n 500M -J > $output_tcp6
sleep 5

# TODO: IPV6 UDP

kill -s SIGINT $pid
sleep 3

test_result "ipv4" $output_tcp4
test_result "ipv6" $output_tcp6

#
# Clean up
#

ip netns del test-traffic
echo "[+] Done!"
