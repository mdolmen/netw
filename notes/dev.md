* llcstat.rs: why ref_table mut?
* what is bpf.table() does?
	* with bpf an object returned by BPF::new()
* what's the difference between "use ..." and "extern crate ..."

BPF_MAP_TYPE_HASH: updates of such a table are atomic
BPF_MAP_TYPE_ARRAY : not atomic update but faster lookup

The kernel keeps track of the number of references to each BPF map and frees a
map only when this count reaches 0.

(BCC ref guide)
`BPF.perf_buffer_poll(timeout=T)`:
> This polls from all open perf ring buffers, calling the callback function that
> was provided when calling open_perf_buffer for each entry.
> 
> The timeout parameter is optional and measured in milliseconds. In its absence,
> polling continues indefinitely.

`core::marker::Send`
> Types that can be transferred across thread boundaries.
> 
> This trait is automatically implemented when the compiler determines it's
> appropriate.
> 
> An example of a non-Send type is the reference-counting pointer rc::Rc. If two
> threads attempt to clone Rcs that point to the same reference-counted value,
> they might try to update the reference count at the same time, which is
> undefined behavior because Rc doesn't use atomic operations. Its cousin
> sync::Arc does use atomic operations (incurring some overhead) and thus is Send.


**What are smart pointers?**

Pointer with additional metadat. An example is a ref counter.
> In Rust, which uses the concept of ownership and borrowing, an additional
> difference between references and smart pointers is that references are pointers
> that only borrow data; in contrast, in many cases, smart pointers own the data
> they point to.

> Smart pointers are usually implemented using structs. The characteristic that
> distinguishes a smart pointer from an ordinary struct is that smart pointers
> implement the Deref and Drop traits.


`Box<T>`: allocate data on the heap.

`FnMut`: a function which can change state, typically a closure (a.k.a. lambda
function)

```rust
				     ----- function pointer (the closure)
				     |
				     v
fn perf_data_callback() -> Box<dyn FnMut(&[u8]) + Send> {
                                           |
	      ------------------------------
              |
	      v
    Box::new(|x| {
        // This callback
	...
    })

    // '+ Send' for multithreading support
    // Box<T> because size unkonwn at compile time so stored on the heap
}
```
