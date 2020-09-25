use bcc::{BPF, Kprobe, BccError};

use std::{thread, time};
use std::sync::{Arc, Mutex};

use core::sync::atomic::{AtomicBool, Ordering};
use ctrlc;
use lazy_static::lazy_static;

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

            // TODO
            p.print_connections();
        }
    }
}

fn do_main(runnable: Arc<AtomicBool>) -> Result<(), BccError> {
    let tcptop = include_str!("bpf/tcptop.c");

    println!("[+] Compiling and installing BPF filters...");

    let mut filter = BPF::new(tcptop)?;

    Kprobe::new()
        .handler("kprobe__tcp_sendmsg")
        .function("tcp_sendmsg")
        .attach(&mut filter)?;
    Kprobe::new()
        .handler("kprobe__tcp_cleanup_rbuf")
        .function("tcp_cleanup_rbuf")
        .attach(&mut filter)?;


    let ipv4_table = filter.table("ipv4_tcp_data")?;
    // TODO: useless var, read the doc
    let _ipv4_map = filter.init_perf_map(ipv4_table, net::ipv4_tcp_cb)?;

    println!("[+] All done! Running...");

    while runnable.load(Ordering::SeqCst) {
        filter.perf_map_poll(200);
    }

    Ok(())
}

fn main() {
    let runnable = Arc::new(AtomicBool::new(true));
    let arc_main = runnable.clone();
    let arc_display = runnable.clone();

    ctrlc::set_handler(move || {
        arc_main.store(false, Ordering::SeqCst);
    })
    .expect("Failed to set handler for SIGINT/SIGTERM");

    thread::spawn(move || {
        display(arc_display);
    });

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
