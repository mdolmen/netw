use bcc::{BPF, Kprobe, BccError};

use std::{thread, time, env};
use std::sync::{Arc, Mutex};

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use ctrlc;

#[macro_use]
extern crate num_derive;

mod net;

enum ExitCode {
    Success,
    Failure,
}

lazy_static! {
    static ref PROCESSES: Mutex<Vec<net::Process>> = Mutex::new(Vec::new());
}

fn display(runnable: Arc<AtomicBool>) {
    while runnable.load(Ordering::SeqCst) {
        thread::sleep(time::Duration::new(1, 0));

        let procs = PROCESSES.lock().unwrap();

        // clear the screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        for p in procs.iter() {
            println!("{}", p);

            p.print_links();
        }
    }
}

fn do_main(runnable: Arc<AtomicBool>) -> Result<(), BccError> {
    let filters = include_str!("bpf/filters.c");

    println!("[+] Compiling and installing BPF filters...");

    let mut filters = BPF::new(filters)?;

    // TCP probes
    Kprobe::new()
        .handler("kprobe__tcp_sendmsg")
        .function("tcp_sendmsg")
        .attach(&mut filters)?;
    Kprobe::new()
        .handler("kprobe__tcp_cleanup_rbuf")
        .function("tcp_cleanup_rbuf")
        .attach(&mut filters)?;

    // UDP probes
    Kprobe::new()
        .handler("kprobe__udp_sendmsg")
        .function("udp_sendmsg")
        .attach(&mut filters)?;
    Kprobe::new()
        .handler("kprobe__udp_recvmsg")
        .function("udp_recvmsg")
        .attach(&mut filters)?;

    let tcp4_table = filters.table("tcp4_data")?;
    let tcp6_table = filters.table("tcp6_data")?;
    let udp4_table = filters.table("udp4_data")?;
    let udp6_table = filters.table("udp6_data")?;
    // TODO: useless var, read the doc
    let _tcp4_map = filters.init_perf_map(tcp4_table, net::tcp4_cb)?;
    let _tcp6_map = filters.init_perf_map(tcp6_table, net::tcp6_cb)?;
    let _udp4_map = filters.init_perf_map(udp4_table, net::udp4_cb)?;
    let _udp6_map = filters.init_perf_map(udp6_table, net::udp6_cb)?;

    println!("[+] All done! Running...");

    while runnable.load(Ordering::SeqCst) {
        filters.perf_map_poll(200);
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut test = false;

    let runnable = Arc::new(AtomicBool::new(true));
    let arc_main = runnable.clone();
    let arc_display = runnable.clone();

    // TODO: use clap to properly handle args
    if args.len() > 1 {
        test = true;
        println!("[debug] test mode");
    }

    ctrlc::set_handler(move || {
        arc_main.store(false, Ordering::SeqCst);
        if test {
            let _ret = net::log_iperf_to_file();
        }
    })
    .expect("Failed to set handler for SIGINT/SIGTERM");

    if !test {
        thread::spawn(move || {
            display(arc_display);
        });
    }

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
