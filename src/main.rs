#![feature(ip)]

use bcc::{BPF, Kprobe, BccError};

use std::{thread, time, env, error::Error, io, time::Duration};
use std::sync::{Arc, Mutex};
use std::mem::drop;
use std::process::exit;

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use ctrlc;
use clap;

#[macro_use]
extern crate num_derive; // FromPrimitive()
extern crate libc;

mod net;
mod dns;

/*
 * For tui
 */
mod ui;
#[allow(dead_code)]
mod util;

use util::event::{Config, Event, Events};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

enum ExitCode {
    Success,
    Failure,
}

lazy_static! {
    static ref PROCESSES: Mutex<Vec<net::Process>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

macro_rules! log {
    ($x:expr) => {
        let mut tmp = LOGS.lock().unwrap();
        tmp.push($x);
        drop(tmp);
    };
}

fn display(runnable: Arc<AtomicBool>) {
    while runnable.load(Ordering::SeqCst) {
        thread::sleep(time::Duration::new(1, 0));

        let procs = PROCESSES.lock().unwrap();

        // clear the screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        for p in procs.iter() {
            println!("{}", p);

            p.print_tlinks();
            p.print_ulinks();
        }
    }
}

fn tui(runnable: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    let tick_rate = 500;
    let enhanced_graphics = true;

    let events = Events::with_config(Config {
        tick_rate: Duration::from_millis(tick_rate),
        ..Config::default()
    });

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ui::App::new(" Sekhmet ", enhanced_graphics);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match events.next()? {
            Event::Input(key) => match key {
                Key::Char(c)  => {  app.on_key(c);   }
                Key::Up       => {  app.on_up();     }
                Key::Down     => {  app.on_down();   }
                Key::Left     => {  app.on_left();   }
                Key::Right    => {  app.on_right();  }
                _ => {}
            },
            Event::Tick => {
                app.on_tick();
            }
        }

        if app.should_quit {
            runnable.store(false, Ordering::SeqCst);
            break;
        }
    }

    Ok(())
}

fn do_main(runnable: Arc<AtomicBool>) -> Result<(), BccError> {
    let filters = include_str!("bpf/filters.c");

    log!(String::from("[+] Compiling and loading BPF filters..."));

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
    Kprobe::new()
        .handler("kprobe__udpv6_sendmsg")
        .function("udpv6_sendmsg")
        .attach(&mut filters)?;
    Kprobe::new()
        .handler("kprobe__udpv6_recvmsg")
        .function("udpv6_recvmsg")
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

    log!(String::from("[+] All done! Running..."));

    while runnable.load(Ordering::SeqCst) {
        filters.perf_map_poll(200);

        // TODO: add data to a SQL base every now and then (here or somewhere else)
    }

    Ok(())
}

fn main() {
    let runnable = Arc::new(AtomicBool::new(true));
    let arc_main = runnable.clone();
    let arc_display = runnable.clone();
    let mut test = false;
    let mut set_ctrlc = false;

    /*
     * Setup program info and possible arguments.
     */
    let matches = clap::App::new("Sekhmet")
        .version("0.1")
        .about("Log and watch network activity per process.")
        .author("Mathieu D. <mathieu.dolmen@gmail.com>")
        .arg(clap::Arg::with_name("mode")
             .short("m")
             .long("mode")
             .help("Select the execution mode")
             .required(true)
             .takes_value(true)
             .possible_values(&["daemon", "test", "ui", "raw"]))
        .get_matches();

    /*
     * Start the program in the selected mode.
     */
    match matches.value_of("mode").unwrap() {
        "daemon" => {
            println!("TO BE IMPLEMENTED");
            exit(0);
        },
        "test" => {
            test = true;
            set_ctrlc = true;
            println!("[debug] test mode");
        },
        "ui" => {
            thread::spawn(move || {
                tui(arc_display);
            });
        },
        "raw" => {
            set_ctrlc = true;
            thread::spawn(move || {
                display(arc_display);
            });
        },
        _ => unreachable!(),
    }

    if set_ctrlc {
        ctrlc::set_handler(move || {
            arc_main.store(false, Ordering::SeqCst);
            if test {
                let _ret = net::log_iperf_to_file();
            }
        })
        .expect("Failed to set handler for SIGINT/SIGTERM");
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
