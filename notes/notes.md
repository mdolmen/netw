First step is to manipulate eBPF and understand how it works.

Different types of probes:
* USDT
* Tracepoint
* Uprobe
* Kprobe

The first two are applied at source level, therefore require re-compilation.

Linux tracing systems & how they fit together
https://jvns.ca/blog/2017/07/05/linux-tracing-systems/

A Linux tracing system can be split into 3 parts:

**1. data sources**

Where the tracing data come from, the probes mentionned earlier.

It seems that sockets also are a data source.

**What is the difference between tracepoints and kprobes?**

Tracepoints are present in source code, therefore there are more stables accross
kernel versions than kprobes (it can be placed on any function). kprobes depend
on the name of the function and the number/order of arguments.
Available ones in `/sys/kernel/debug/tracing/events`.

**2. data collection**

`eBPF` is one of such component. Others are `ftrace` or `perf_event`.

It is the eBPF program that send the data to userspace with `ftrace`, `perf` or
`BPF maps`

* `perf`: (Perf Buffer) register event data in a structure, overwritten at each
iteration, no persistence
* `BPF maps`: more complex structure, persistence possible

It is possible to pass *maps* to user space through the *perf* buffer (as a
field of the structure).

**3. frontend**

User interface to actually use these data.

e.g. `bcc`:
* compile C code into eBPF bytecode
* attach to a probe
* communicate with the compiled bytecode to get information from it


So basically I have to write a front end?! A kind of a re-implementation of BCC
that does only what I'm interested in. That library alone deserves its own
project...

`bpf-sys` crate: bindings for libbpf from BCC.


Don't go over the rust problematic just yet. Use it from python first.

* **What filter to use to gather network packets?**

All that we need is in `tcptop.py` from BCC examples. Need to adapt the display
and change the logic a bit to show accumulated amount of data sent per
process/connection.
`tcplife.py` contains example to track connection state change.

* **How to make the correlation packet/process?**

Easy, from userspace. Parse proc/.

* **How to block a packet?**

We don't. transfer information to another app (e.g. firewall) to do the
blocking.

* **What's the difference between BPF_PROG_TYPE_SOCKET_FILTER and BPF_PROG_TYPE_KPROBE?**


Goals
-----

1. Read ressources already listed
2. Reproduce tcptop.py in rust
3. Adapt it to display the information I want

Example: print processes opening a given file
---------------------------------------------

The stack size for an eBPF program is 512 bytes.
cf. https://sysdig.com/blog/the-art-of-writing-ebpf-programs-a-primer/

Use eBPF maps to store data outside the stack.

Based on this: https://www.linuxembedded.fr/2019/03/les-secrets-du-traceur-ebpf/
and bcc examples.

In kernel, the eBPF program forward all file access to user space. It has the
advantage of minimizing the time spent in kernel. The comparison for the
targeted file is done in userspace. To do it in kernel we would have to unwind
the loop. Problematic if we want flexibility for the filter (e.g. changind the
filename to look for, add a new one).

Is it still OK to do like that when it comes to filtering packets?
* not a problem, I want all packets to be traced to record activity of all
processes

[Talk] Netflix talks about Extended BPF
---------------------------------------

BPF API offers a restricted set of features. Cannot panic the kernel, contrary to
loadable kernel module: more power but less predictable/verifiable.

4kb size limit.

Not only for observability, being extended to be a new model of programming,
making Linux a kind of microkernel.

BPF instructions can be passed to an interpreter or a JIT compiler. JIT compiler
safer. A security patch to the compiler does not break the BPF programs. A
patch to the kernel requires to recompile the modules.

bpftrace: commandline tool for one-liner and short BPF programs

Tools to observe BPF programs:
* bpftool
* perf top

[Talk] ftrace: Where modifying a running kernel all started
-----------------------------------------------------------

All functions that can be traced are listed in:
`/sys/kernel/debug/tracing/available_filter_functions`

The tracing works by adding a `__fentry` call to the beginning of every kernel
functions. The functions in that file are sorted by address order (potential
utility to bypass KASLR!?).

There is a section with these address + a int for flags (is the function being
trace or not, etc.).

The first instructions of the fucntions is modify to call `__fentry` thanks to
INT3. The first bytes is replaced  by a break point which will trigger the
interrupt routine where all the opcodes of the "line" can be changed without
causing GFP (General Fault Protection). An IPI is sent in the process to
synchornize all the cores so that everyone see what it's supposed to see.

Mechanism behind ftrace, used for live kernel patching.

Example from bcc scripts
------------------------

**tcplife.py**: great source of inspiration for my project!

**tcptop.py**: even better!
* print the process name holding the connection
* print number of packets tx/rx
* all the data I want are gathered in this script, what I have to do is
use/present it in a more effective/usable way


Various reading
---------------

https://bolinfest.github.io/opensnoop-native/#part-4-standalone-opensnoop-using-ebpf-in-c

BCC requires LLVM to perform the converison to bytecode. `libbpf` does not
(from linux kernel `tools/lib/bpf`, wrapper around bpf syscall).

His goal is to use BPF without relying on BCC at runtime (to get ride of
python and improve perf/portability). What I want to do as well!

To see running krpobes:
https://github.com/iovisor/bcc/blob/master/introspection/bps.c


Though for a moment that `redbpf` was an option but it does not correspond to
what I want. Its gaol is to make the **BPF code** in rust, not using BPF C code
(as its the case in BCC) from a rust application.

Which leaves the follwoing choices:
* `rust-bcc`: write BPF program in C, compile and do the logic in rust
* `libbpf-rs`: rust bindings for `libppf`
	* libbpf , in its newer version (past february 2020) has gain lots of
	functionnality, possible to apply the Compile-Once Run-Everywhere
	paradigm (a.k.a. more portability), and big plus, no need for kernel
	headers nor CLANG on the target machine
	* rust bindings does not seems up-to-date (I could contribute to
	that...)


The art of writing eBPF programs
--------------------------------

About writing "pure" BPF programs.

Accessing memory requires the use of helper function to verify that the memory
is valid.

Dereferencing members of the context structure is OK because already validated
by BPF verifier.

Stack size of BPF programs limited to 512 bytes.

BPF programs can never be preempted.


Thoughts
--------

=> Could start with BCC and then switch to libbpf.

1. Having BCC to compile the BPF program before starting the app is not a
   deal-breaker for what I want to do. First its an app for me. Then if others
   find an interest in the use case, they just have to install some requirements
   (kernel header and bcc).
2. Slowly migrate to `libbpf-rs` in the future as the project grows, and my
   comprehension of the topic increase. Maybe help to make it grow.
3. The priority is to get observability on what's going on. No particular
   constraints about performance or even portability. Let's appreciate that and
   start hacking. `rust-bcc` it is for now!
4. It has an option to statically link `libbpf` and `libbcc`. To explore.


Use a SQL database
------------------

* Config option to define the refresh rate
* Where to update the DB?
* How to access it concurrently? (daemon + UI)
