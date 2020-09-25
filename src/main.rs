use bcc::{BPF, Kprobe, BccError};
use bcc::table::Table;
use bcc::perf_event::init_perf_map;

use std::{mem, ptr, thread, time, fmt};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use core::sync::atomic::{AtomicBool, Ordering};
use ctrlc;
use lazy_static::lazy_static;

enum ExitCode {
    Success,
    Failure,
}

struct Process {
    pid: u32,
    name: String,
    //command: String,
    conns: Vec<Connection>,
    //nb_conns: u32,
    //status: u8, // TODO: enum
}

struct Connection {
    saddr: u32,
    daddr: u32,
    lport: u16,
    dport: u16,
    rx: u64,
    tx: u64,
    // TODO: unit
    //total_size: u64, // TODO: is it really the size or nb packets?
}

/*
impl Process {
    // TODO
    fn new() -> Process {
        Process {
        }
    }
}

impl Connection {
    // TODO
    fn new() -> Connection {
        Connection {
        }
    }
}
*/

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
impl Eq for Process {}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "PID: {} | NAME: {} | NB CONN: {}",
            self.pid,
            self.name,
            self.conns.len(),
        )
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.saddr == other.saddr
            && self.daddr == other.daddr
            && self.lport == other.lport
            && self.dport == other.dport
    }
}
impl Eq for Connection {}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "\t{}:{} <-> {}:{} RX: {} TX: {}",
            Ipv4Addr::from(self.saddr.to_be()),
            self.lport,
            Ipv4Addr::from(self.daddr.to_be()),
            self.dport,
            self.rx,
            self.tx,
        )
    }
}

lazy_static! {
    static ref PROCESSES: Mutex<Vec<Process>> = Mutex::new(Vec::new());
}

#[repr(C)]
struct ipv4_data_t {
    pid: u32,
    saddr: u32,
    daddr: u32,
    lport: u16,
    dport: u16,
    size: u32,
    is_rx: u32,
}

fn ipv4_tcp_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct(x);

        let mut procs = PROCESSES.lock().unwrap();

        // TODO: make a builder for the struct
        let mut p = Process {
            pid: data.pid,
            // TODO
            name: String::from("placeholder"),
            conns: Vec::new(),
        };

        let mut c = Connection {
            saddr: data.saddr,
            daddr: data.daddr,
            lport: data.lport,
            dport: data.dport,
            rx: 0,
            tx: 0,
            //total_size: 0,
        };

        if procs.contains(&p) {
            let mut p = procs.iter_mut().find(|x| x.pid == data.pid).unwrap();

            if p.conns.contains(&c) {
                let mut c = p.conns.iter_mut().find(|x| **x == c).unwrap();

                if data.is_rx == 1 {
                    c.rx += data.size as u64;
                } else {
                    c.tx += data.size as u64;
                }
            } else {
                p.conns.push(c);
            }
        } else {
            p.conns.push(c);
            procs.push(p);
        }
    })
}

fn display(runnable: Arc<AtomicBool>) {
    while runnable.load(Ordering::SeqCst) {
        thread::sleep(time::Duration::new(1, 0));
        let procs = PROCESSES.lock().unwrap();

        //println!("PID  |    SADDR    |    DADDR    | LPORT | DPORT | SIZE");

        // clear the screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        for p in procs.iter() {
            println!("{}", p);

            for c in p.conns.iter() {
                println!("{}", c);
            }
        }
    }
}

fn to_map(table: &mut Table) -> HashMap<(u32, u32), u64> {
    let mut map = HashMap::new();

    for entry in table.iter() {
        let key = parse_struct(&entry.key);
        let value = parse_u64(entry.value);

        // TODO: review this, key not uniq
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
    let _ipv4_map = filter.init_perf_map(ipv4_table, ipv4_tcp_cb)?;

    println!("[+] All done! Running...");

    while runnable.load(Ordering::SeqCst) {
        //ipv4_recv_map.poll(200);
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

    let th_display = thread::spawn(move || {
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
