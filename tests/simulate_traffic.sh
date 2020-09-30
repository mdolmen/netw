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

#
# Setup network namespace. Could be done without, just used as a convenience not
# to mess with the host network settings.
#

output="iperf.json"
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
sleep 1
ip netns exec $ns iperf3 -4 -c 10.0.10.100 -p 5201 -n 2G -B 10.0.10.200 -J > $output
sleep 3
kill -s SIGINT $pid

# TODO: IPV4 UDP
# TODO: IPV6 TCP
# TODO: IPV6 UDP

# TODO: parse JSON from iperf and sekhmet
#sum_sender = data['end']['sum_sent']['bytes']

# TODO: compare both and print result

#
# Clean up
#

ip netns del test-traffic
echo "[+] Done!"
