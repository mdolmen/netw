Sekhmet
=======

Goddess of war and destroyer of the ennemies. (*britanicca.com*)


What?
-----

I want to see information about network connections (open channels, amount of
data being transferred, etc.), in real-time, grouped by applications.

I should be able to block certain connections, white/black list specific target
or a whole application from communicating with the outside world.

In other word: a software firewall.


Current approach
----------------

* eBPF to filter packets
* written in rust for portability, efficiency, safety, etc.
* UI to see what's happening and to change the rules
	* Web UI (complete view) + Gnome extension (quick look)


Status
------

Working version.

* Display processes communicating over the network
* Display amount of data transferred per link and per process
* All links are displayed, wether established or not
* TCP and UDP, IPv4 and IPv6


How to use
----------

Run:

```bash
cargo build
sudo ./target/debug/sekhmet
```

Unit tests:

```bash
# --test-threads=1 important otherwise fails occasionaly because of the global
# vector containing the processes being empty when accessed. Only an issue when
# test cases are run concurrently.
cargo test -- --test-threads=1
```

Test that you actually intercept something:

```bash
cd tests
sudo ./simulate_traffic.sh
```
