use std::{ptr, fmt, fs};
use std::net::{Ipv4Addr};
use std::fs::File;
use std::io::prelude::*;

use serde_json::json;

use crate::PROCESSES;

pub struct Process {
    pid: u32,
    name: String,
    //command: String,
    tlinks: Vec<TCPLink>,
    // TODO: Vec for UDP
    // TODO: per process total size
    //nb_tlinks: u32,
    //status: u8, // TODO: enum
}

impl Process {
    fn new(pid: u32, name: String) -> Self {
        Process {
            pid: pid,
            name: String::from(name),
            tlinks: Vec::new(),
        }
    }

    fn get_connections(&self) -> &Vec<TCPLink> {
        &self.tlinks
    }

    pub fn print_connections(&self) {
        for l in self.tlinks.iter() {
            println!("{}", l);
        }
    }
}

/*
impl TCPLink {
    // TODO
    fn new() -> TCPLink {
        TCPLink {
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
            f, "{} ({}):",
            self.name,
            self.pid,
        )
    }
}

struct TCPLink {
    // TODO: make addr as enum to diff between v4 ou v6
    saddr: u32,
    daddr: u32,
    lport: u16,
    dport: u16,
    rx: u64,
    tx: u64,
    // TODO: unit
    //total_size: u64, // TODO: is it really the size or nb packets?
}

// TODO: struct for UPD and its implem

impl PartialEq for TCPLink {
    fn eq(&self, other: &Self) -> bool {
        self.saddr == other.saddr
            && self.daddr == other.daddr
            && self.lport == other.lport
            && self.dport == other.dport
    }
}
impl Eq for TCPLink {}

impl fmt::Display for TCPLink {
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

        let path_comm = format!("/proc/{}/comm", data.pid);
        //let path_cmdline = format!("/proc/{}/cmdline", data.pid);
        let content_comm = fs::read_to_string(path_comm);
        //let content_cmdline = fs::read_to_string(path_cmdline);

        let name = match content_comm {
            Ok(mut content) => { content.pop(); content },
            Err(_error) => String::from("file not found"),
        };
        // TODO: some kind of verbose mode
        //let _cmdline = match content_cmdline {
        //    Ok(mut content) => { content.pop(); content },
        //    Err(error) => String::from("file not found"),
        //};

        let mut p = Process::new(data.pid, name);

        // TODO: make a builder for the struct
        let l = TCPLink {
            saddr: data.saddr,
            daddr: data.daddr,
            lport: data.lport,
            dport: data.dport,
            rx: 0,
            tx: 0,
        };

        if procs.contains(&p) {
            let p = procs.iter_mut().find(|x| x.pid == data.pid).unwrap();

            if p.tlinks.contains(&l) {
                let mut l = p.tlinks.iter_mut().find(|x| **x == l).unwrap();

                if data.is_rx == 1 {
                    l.rx += data.size as u64;
                } else {
                    l.tx += data.size as u64;
                }
            } else {
                p.tlinks.push(l);
            }
        } else {
            p.tlinks.push(l);
            procs.push(p);
        }
    })
}

fn parse_struct(addr: &[u8]) -> ipv4_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv4_data_t) }
}

/*
 * Output only a limited amount information for testing purposes.
 *
 * Typical traffic interception for iperf:
 *
 * iperf3 (221251):
 *        10.0.10.100:5201 <-> 10.0.10.200:49289 RX: 411 TX: 299
 *        10.0.10.100:5201 <-> 10.0.10.200:47159 RX: 5368709120 TX: 0
 * iperf3 (222684):
 *        10.0.10.200:49289 <-> 10.0.10.100:5201 RX: 299 TX: 411
 *        10.0.10.200:47159 <-> 10.0.10.100:5201 RX: 0 TX: 5368709120
 *
 * We care only about the link with a lot of data and either TX or RX at 0. The other corresponds
 * to setup of the communication. We want to be sure that we intercept as much data as reported by
 * iperf. Note that RX may be different than TX due to packet drops (normal behavior).
 *
 */
pub fn log_iperf_to_file() -> std::io::Result<()> {
    let procs = PROCESSES.lock().unwrap();
    let mut rx = 0;
    let mut tx = 0;

    for p in procs.iter() {
        if p.name == String::from("iperf3") {
            for l in p.get_connections() {
                if l.rx == 0 {
                    tx = l.tx;
                } else if l.tx == 0 {
                    rx = l.rx;
                }
            }
        }
    }

    let output = json!({
        "rx": rx,
        "tx": tx
    });
    let mut file = File::create("sekhmet.json")?;
    file.write_all(output.to_string().as_bytes())?;

    Ok(())
}
