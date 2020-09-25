use std::{ptr, fmt};
use std::net::{Ipv4Addr};

use crate::PROCESSES;

pub struct Process {
    pid: u32,
    name: String,
    //command: String,
    conns: Vec<Connection>,
    //nb_conns: u32,
    //status: u8, // TODO: enum
}

impl Process {
    // TODO
    //fn new() -> Process {
    //    Process {
    //        pid: 0,
    //        name: "none",
    //        conns: Vec::new(),
    //    }
    //}

    pub fn print_connections(&self) {
        for c in self.conns.iter() {
            println!("{}", c);
        }
    }
}

/*
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

pub fn ipv4_tcp_cb() -> Box<dyn FnMut(&[u8]) + Send> {
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

        let c = Connection {
            saddr: data.saddr,
            daddr: data.daddr,
            lport: data.lport,
            dport: data.dport,
            rx: 0,
            tx: 0,
        };

        if procs.contains(&p) {
            let p = procs.iter_mut().find(|x| x.pid == data.pid).unwrap();

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

fn parse_struct(addr: &[u8]) -> ipv4_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv4_data_t) }
}
