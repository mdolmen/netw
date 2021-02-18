#![feature(ip)]

use bcc::{BPF, Kprobe, BccError};

use std::{thread, time, env, error::Error, io, time::Duration};
use std::sync::{Arc, Mutex};
use std::mem::drop;

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use ctrlc;

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

use ui::App;
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

fn tui() -> Result<(), Box<dyn Error>> {
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

    let mut app = App::new(" Sekhmet ", enhanced_graphics);

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
            break;
        }
    }

    Ok(())
}

fn do_main(runnable: Arc<AtomicBool>) -> Result<(), BccError> {
    let filters = include_str!("bpf/filters.c");

    let mut logs = LOGS.lock().unwrap();
    logs.push(String::from("[+] Compiling and installing BPF filters..."));
    drop(logs);

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

    let mut logs = LOGS.lock().unwrap();
    logs.push(String::from("[+] All done! Running..."));
    drop(logs);

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
            //display(arc_display);
            tui();
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
