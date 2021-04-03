#![feature(ip)]

use bcc::{BPF, Kprobe, BccError};

use std::{thread, time, error::Error, io, time::Duration};
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use std::mem::drop;
use std::path::Path;

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use ctrlc;
use clap;

use rusqlite::{Connection};
use chrono::Utc;

#[macro_use]
extern crate num_derive; // FromPrimitive()
extern crate libc;

mod net;
mod dns;
mod database;

/*
 * For tui
 */
mod ui;
#[allow(dead_code)]
mod util;

use util::event::{Config, Event, Events};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};
use database::{create_db, open_db, update_db, get_procs, get_dates};
use crate::net::Process;

enum ExitCode {
    Success,
    Failure,
}

#[derive(Clone)]
struct Date {
    pub int_form: u32,
    pub str_form: String, // MM/DD
}

impl Date {
    pub fn get_dates_str() -> Vec<String> {
        let dates = DATES.lock().unwrap().to_vec();
        let mut dates_str = Vec::new();

        for d in dates.iter() {
            dates_str.push(d.str_form.clone());
        }

        dates_str
    }
}

lazy_static! {
    // TODO: ring buffer to limit size in memory
    // TODO: save in some shared memory so UI can connect to running daemon??
    static ref PROCESSES: Mutex<Vec<Process>> = Mutex::new(Vec::new());
}
lazy_static! {
    // TODO: ring buffer too so we can flush regularly to file, without having duplicates in it
    static ref LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref DATES: Mutex<Vec<Date>> = Mutex::new(Vec::new());
}

static DEBUG: bool = true;

macro_rules! log {
    ($x:expr) => {
        let mut tmp = LOGS.lock().unwrap();
        tmp.push($x);
        drop(tmp);

        if DEBUG { println!("{}", $x) }
    };
}

///
/// Raw terminal display.
///
/// * `runnable` - A reference shared by all threads
///
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

///
/// Terminal UI
///
/// * `runnable` - A reference shared by all threads
///
fn tui(runnable: Arc<AtomicBool>, source: String) -> Result<(), Box<dyn Error>> {
    let mut tick_rate = 500;
    let enhanced_graphics = true;
    let procs: Vec<Process>;
    let mut app = ui::App::new(" Sekhmet ", enhanced_graphics);

    /*
     * Select the input source to display data from.
     */
    if source == "realtime" {
        procs = PROCESSES.lock().unwrap().to_vec();
    } else {
        let db = open_db(&source).unwrap();
        log!(String::from(format!("[+] Database {} opened", &source)));

        // TODO: make the same as 'freq'
        tick_rate = 500; //120000;
        procs = get_procs(&db);

        let mut dates = DATES.lock().unwrap();

        for date in get_dates(&db).iter() {
            let tmp = *date / 10000;
            let month = tmp / 100;
            let day = tmp % 100;

            let date_str = String::from(format!("{:02}/{:02}", month, day));
            dates.push( Date { int_form: *date, str_form: date_str } );
        };

        app.db(db);
    }

    app.procs(procs);

    let events = Events::with_config(Config {
        tick_rate: Duration::from_millis(tick_rate),
        ..Config::default()
    });

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

///
/// Run in daemon mode. The data retrieved by the probes is stored in a SQL database.
///
/// * `runnable` - A reference shared by all threads
/// * `freq`     - Time, in seconds, between two updates of the db
///
fn run_daemon(runnable: Arc<AtomicBool>, filename: String, _freq: u64) {
    // TODO: use freq
    let delay = Duration::new(2, 0);
    let date = Utc::now().format("%m%d%Y").to_string();
    let date: u32 = date.parse().unwrap();
    let mut db: Connection;

    if !Path::new(&filename).exists() {
        db = create_db(&filename).unwrap();
        log!(String::from(format!("[+] Database {} created", &filename)));
    } else {
        db = open_db(&filename).unwrap();
        log!(String::from(format!("[+] Database {} opened", &filename)));
    }

    while runnable.load(Ordering::SeqCst) {
        thread::sleep(delay);

        let procs = PROCESSES.lock().unwrap().to_vec();

        let _ret = update_db(&mut db, &procs, date);

        // TODO: save logs to file
    }
}

///
/// Compile BPF code and start the probes.
///
/// * `runnable` - A reference shared by all threads
///
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
    }

    Ok(())
}

fn main() {
    let runnable = Arc::new(AtomicBool::new(true));
    let arc_main = runnable.clone();
    let arc_display = runnable.clone();
    let arc_daemon = runnable.clone();
    let mut th_ui: Option<JoinHandle<()>> = None;
    let mut th_daemon: Option<JoinHandle<()>> = None;
    let mut test = false;
    let mut set_ctrlc = false;
    let mut set_probes = true;

    /*
     * Read options from config file.
     */
    let yaml = clap::load_yaml!("../config.yaml");
    let matches = clap::App::from(yaml).get_matches();

    let freq = matches.value_of("frequency").unwrap();
    let freq: u64 = freq.parse().unwrap();
    let output = String::from( matches.value_of("output").unwrap() );

    // TODO: a config file
    //      -> capture TCP and/or UDP, IPv4 and/or IPv6
    //      -> display IP addresses or domain
    //      -> display or not TCP, UDP
    //      -> how far long ago (date) to display in the UI

    /*
     * Start the program in the selected mode.
     */
    match matches.value_of("mode").unwrap() {
        "daemon" => {
            set_ctrlc = true;

            th_daemon = Some(thread::spawn(move || {
                run_daemon(arc_daemon, output, freq);
            }));
        },
        "test" => {
            test = true;
            set_ctrlc = true;
            println!("[debug] test mode");
        },
        "ui" => {
            let source = matches.value_of("source").unwrap();
            let source = String::from(source);

            if source != "realtime" {
                set_probes = false;
            }

            th_ui = Some(thread::spawn(move || {
                let _ret = tui(arc_display, source);
            }));
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

    if set_probes {
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

    /*
     * Wait for threads to finish.
     */
    match th_ui {
        Some(th) => th.join().unwrap(),
        None     => (),
    }
    match th_daemon {
        Some(th) => th.join().unwrap(),
        None     => (),
    }
}
