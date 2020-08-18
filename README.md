Sekhmet
=======

Godess of war and destroyer of the enemies. (*britanicca.com*)


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

Project initialized.
