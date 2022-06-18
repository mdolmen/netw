Idea
----

Compute some sort of hash to diff with external packet interception.

Will allow to detect rootkit communication.


Leads
-----

`struct sock` has fields sk_txhash and sk_rxhash set to a random number.

Cf. `include/net/sock.h`
