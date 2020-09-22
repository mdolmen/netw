use bcc::{BPF, Kprobe, BccError};
use bcc::table::Table;
use bcc::perf_event::init_perf_map;

use std::{mem, ptr, thread, time};
use std::sync::Arc;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use core::sync::atomic::{AtomicBool, Ordering};
use ctrlc;

enum ExitCode {
    Success,
    Failure,
}

#[repr(C)]
struct ipv4_data_t {
    pid: u32,
    saddr: u32,
    daddr: u32,
    lport: u16,
    dport: u16,
    size: u32,
}

fn ipv4_send_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct(x);

        println!(
            "{} | {} | {} | {} | {} | {}",
            data.pid,
            Ipv4Addr::from(data.saddr.to_be()),
            Ipv4Addr::from(data.daddr.to_be()),
            data.lport,
            data.dport,
            data.size,
        );
    })
}

fn to_map(table: &mut Table) -> HashMap<(u32, u32), u64> {
    let mut map = HashMap::new();

    for entry in table.iter() {
        let key = parse_struct(&entry.key);
        let value = parse_u64(entry.value);

        map.insert((key.pid, key.saddr), value);
    }

    map
}

fn parse_struct(addr: &[u8]) -> ipv4_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv4_data_t) }
}

fn parse_u64(x: Vec<u8>) -> u64 {
    let mut v = [0_u8; 8];

    for i in 0..8 {
        v[i] = *x.get(i).unwrap_or(&0);
    }

    unsafe { mem::transmute(v) }
}

fn do_main(runnable: Arc<AtomicBool>) -> Result<(), BccError> {
    let tcptop = include_str!("bpf/tcptop.c");

    println!("[+] Compiling and installing filter...");

    let mut filter = BPF::new(tcptop)?;

    Kprobe::new()
        .handler("kprobe__tcp_sendmsg")
        .function("tcp_sendmsg")
        .attach(&mut filter)?;

    let ipv4_send_data = filter.table("ipv4_send_data")?;
    let mut ipv4_send_map = init_perf_map(ipv4_send_data, ipv4_send_cb)?;

    println!("[+] All done! Running...");

    println!("PID  |    SADDR    |    DADDR    | LPORT | DPORT | SIZE");

    while runnable.load(Ordering::SeqCst) {
        ipv4_send_map.poll(200);
    }

    Ok(())
}

fn main() {
    let runnable = Arc::new(AtomicBool::new(true));
    let r = runnable.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Failed to set handler for SIGINT/SIGTERM");

    match do_main(runnable) {
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(ExitCode::Failure as i32);
        }
        _ => {
            std::process::exit(ExitCode::Success as i32);
        }
    }
}
