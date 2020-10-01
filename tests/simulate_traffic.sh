#!/bin/bash

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
output1="iperf.json"

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

#
# Generate traffic
#

echo "[+] Starting traffic interception..."
../target/debug/sekhmet test &
pid=$(echo $!)
# Let it compile BPF code
sleep 5

echo "[+] Simulating traffic..."
# IPV4 TCP
ip netns exec $ns iperf3 -s -B 10.0.10.100 -1 &> /dev/null &
sleep 2
ip netns exec $ns iperf3 -4 -c 10.0.10.100 -p 5201 -n 1G -B 10.0.10.200 -J > $output1
sleep 5
kill -s SIGINT $pid
sleep 1

# TODO: IPV4 UDP
# TODO: IPV6 TCP
# TODO: IPV6 UDP

rx_intercepted=$(cat $output0 | jq '.rx')
tx_intercepted=$(cat $output0 | jq '.tx')
rx_iperf=$(cat $output1 | jq '.end.sum_received.bytes')
tx_iperf=$(cat $output1 | jq '.end.sum_sent.bytes')

if [ $rx_intercepted == $rx_iperf ] && [ $tx_intercepted == $tx_iperf ]
then
	echo -e "[test] ${GREEN}IPV4 TCP: OK${NC}"
else
	echo -e "[test] ${RED}IPV4 TCP: iperf3 traffic != traffic intercepted${NC}"
	echo "$rx_intercepted"
	echo "$rx_iperf"
	echo "$tx_intercepted"
	echo "$tx_iperf"
fi

#
# Clean up
#

ip netns del test-traffic
echo "[+] Done!"
