use std::{ptr, fmt, fs};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::fs::File;
use std::io::prelude::*;

use crate::PROCESSES;

extern crate num;

#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum DataUnit {
    Bytes,
    KBytes,
    MBytes,
    GBytes,
    TBytes,
}

pub struct Process {
    pid: u32,
    name: String,
    //command: String,
    // TODO: vec v4 + vec v6?
    tlinks: Vec<TCPLink>,
    //nb_tlinks: u32,
    // TODO: Vec for UDP
    rx: u64,
    tx: u64,
    //status: u8, // TODO: enum
}

impl Process {
    fn new(pid: u32) -> Self {
        Process {
            pid: pid,
            name: String::new(),
            tlinks: Vec::new(),
            rx: 0,
            tx: 0,
        }
    }

    fn get_tlinks(&self) -> &Vec<TCPLink> {
        &self.tlinks
    }

    fn add_data(&mut self, size: u64, is_rx: u32) {
        match is_rx {
            0 => self.tx += size,
            1 => self.rx += size,
            _ => (),
        }
    }

    fn name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
    }

    pub fn print_links(&self) {
        for l in self.tlinks.iter() {
            println!("{}", l);
        }
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
impl Eq for Process {}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (rx, rx_unit) = group_bytes(self.rx);
        let (tx, tx_unit) = group_bytes(self.tx);

        write!(
            f, "{} ({}) RX:{:.2}{u0} TX:{:.2}{u1}:",
            self.name,
            self.pid,
            rx,
            tx,
            u0 = match rx_unit {
                DataUnit::Bytes => "B",
                DataUnit::KBytes => "KB",
                DataUnit::MBytes => "MB",
                DataUnit::GBytes => "GB",
                DataUnit::TBytes => "TB",
            },
            u1 = match tx_unit {
                DataUnit::Bytes => "B",
                DataUnit::KBytes => "KB",
                DataUnit::MBytes => "MB",
                DataUnit::GBytes => "GB",
                DataUnit::TBytes => "TB",
            },
        )
    }
}

struct TCPLink {
    saddr: IpAddr,
    daddr: IpAddr,
    lport: u16,
    dport: u16,
    rx: u64,
    tx: u64,
    // TODO: unit
    //total_size: u64, // TODO: is it really the size or nb packets?
}

// TODO: struct for UPD and its implem

impl TCPLink {
    fn new(saddr: IpAddr, daddr: IpAddr, lport: u16, dport: u16) -> TCPLink {
        TCPLink {
            saddr,
            daddr,
            lport,
            dport,
            rx: 0,
            tx: 0,
        }
    }

    fn add_data(&mut self, size: u64, is_rx: u32) {
        match is_rx {
            0 => self.tx += size,
            1 => self.rx += size,
            _ => (),
        }
    }
}

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
        let (rx, rx_unit) = group_bytes(self.rx);
        let (tx, tx_unit) = group_bytes(self.tx);

        write!(
            f, "\t{}:{} <-> {}:{} RX: {:.2}{u0} TX: {:.2}{u1}",
            self.saddr,
            self.lport,
            self.daddr,
            self.dport,
            rx,
            tx,
            u0 = match rx_unit {
                DataUnit::Bytes => "B",
                DataUnit::KBytes => "KB",
                DataUnit::MBytes => "MB",
                DataUnit::GBytes => "GB",
                DataUnit::TBytes => "TB",
            },
            u1 = match tx_unit {
                DataUnit::Bytes => "B",
                DataUnit::KBytes => "KB",
                DataUnit::MBytes => "MB",
                DataUnit::GBytes => "GB",
                DataUnit::TBytes => "TB",
            },
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

#[repr(C)]
struct ipv6_data_t {
    saddr: u128,
    daddr: u128,
    pid: u32,
    lport: u16,
    dport: u16,
    size: u32,
    is_rx: u32,
}

pub fn ipv4_tcp_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv4(x);

        let p = Process::new(data.pid);

        let l = TCPLink::new(
            IpAddr::V4( Ipv4Addr::from(data.saddr.to_be()) ),
            IpAddr::V4( Ipv4Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );

        update_procs_and_links(p, l, data.size as u64, data.is_rx);
    })
}

pub fn ipv6_tcp_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv6(x);

        let p = Process::new(data.pid);

        let l = TCPLink::new(
            IpAddr::V6( Ipv6Addr::from(data.saddr.to_be()) ),
            IpAddr::V6( Ipv6Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );

        update_procs_and_links(p, l, data.size as u64, data.is_rx);
    })
}

fn update_procs_and_links(mut p: Process, mut l: TCPLink, packets_size: u64, is_rx: u32) {
    let mut procs = PROCESSES.lock().unwrap();

    if procs.contains(&p) {
        let known_p = procs.iter_mut().find(|x| x.pid == p.pid).unwrap();

        known_p.add_data(packets_size, is_rx);

        if known_p.tlinks.contains(&l) {
            let known_link = known_p.tlinks.iter_mut().find(|x| **x == l).unwrap();

            known_link.add_data(packets_size, is_rx);
        } else {
            l.add_data(packets_size, is_rx);
            known_p.tlinks.push(l);
        }
    } else {
        let path_comm = format!("/proc/{}/comm", p.pid);
        let content_comm = fs::read_to_string(path_comm);
        //let path_cmdline = format!("/proc/{}/cmdline", data.pid);
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

        p.name(name);
        l.add_data(packets_size, is_rx);
        p.add_data(packets_size, is_rx);
        p.tlinks.push(l);
        procs.push(p);
    }
}

fn parse_struct_ipv4(addr: &[u8]) -> ipv4_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv4_data_t) }
}

fn parse_struct_ipv6(addr: &[u8]) -> ipv6_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv6_data_t) }
}

fn group_bytes(bytes: u64) -> (f64, DataUnit) {
    let mut i = 0;
    let mut grouped = bytes as f64;

    while grouped >= 1024.0 {
        i += 1;
        grouped /= 1024.0;
    }

    let unit = match num::FromPrimitive::from_u32(i) {
        Some(x) => x,
        _ => DataUnit::Bytes,
    };

    (grouped, unit)
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
 * to the setup of the communication between client/server. We want to be sure that we intercept
 * the same amount of data reported by to be transferred by iperf. The actual intercepted traffic
 * should be slightly higher du to the packets used to establish the comm (precisely 37 bytes,
 * based on observations => hardcoded in the testing script, works for now...). Note that RX may be
 * different than TX due to packet drops (normal behavior).
 *
 */
pub fn log_iperf_to_file() -> std::io::Result<()> {
    let procs = PROCESSES.lock().unwrap();
    let mut rx4 = 0;
    let mut tx4 = 0;
    let mut rx6 = 0;
    let mut tx6 = 0;

    for p in procs.iter() {
        if p.name == String::from("iperf3") {
            for l in p.get_tlinks() {
                if l.rx == 0 {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { tx4 = l.tx; } else { tx6 = l.tx }
                } else if l.tx == 0 {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { rx4 = l.rx; } else { rx6 = l.rx }
                }
            }
        }
    }

    let output = format!(
        "{{ \"ipv4\": {{ \"rx\": {}, \"tx\": {} }}, \"ipv6\": {{ \"rx\": {}, \"tx\": {} }} }}",
        rx4, tx4, rx6, tx6
    );

    let mut file = File::create("sekhmet.json")?;
    file.write_all(output.to_string().as_bytes())?;

    Ok(())
}

/*
 * TESTS
 */

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
        std::slice::from_raw_parts(
            (p as *const T) as *const u8,
            std::mem::size_of::<T>(),
        )
    }

    // TODO: refactor first?!
    // whatever_cb
    #[test]
    fn tcp4_cb_one_process_multiple_links() {
        // clear the vector of process in case other test executed before
        {
            let mut procs = PROCESSES.lock().unwrap();

            // no elements should match that condition
            procs.retain(|e| e.pid == u32::MAX);
        }

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // 192.168.1.2
            daddr: 3361999370,  // 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // 192.168.1.2
            daddr: 3361999370,  // 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 567890,
            is_rx: 0,
        };
        let mut ptr = ipv4_tcp_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 1, "number of process incorrect");

        let p = procs.iter().next().unwrap();
        let c = p.tlinks.iter().next().unwrap();
        let ip_src = IpAddr::V4( Ipv4Addr::new(192, 168, 1, 2) );
        let ip_dst = IpAddr::V4( Ipv4Addr::new(10, 10, 100, 200) );

        assert_eq!(p.pid, 1234, "pid incorrect");
        assert_ne!(p.name, "", "process name empty");
        assert_eq!(p.rx, 56789, "process rx incorrect");
        assert_eq!(p.tx, 567890, "process tx incorrect");
        assert_eq!(c.saddr, ip_src, "source ip address incorrect");
        assert_eq!(c.daddr, ip_dst, "destination ip address incorrect");
        assert_eq!(c.lport, 4321, "local port incorrect");
        assert_eq!(c.dport, 80, "destination port incorrect");
        assert_eq!(c.rx, 56789, "rx size incorrect");
        assert_eq!(c.tx, 567890, "tx size incorrect");
    }

    #[test]
    fn tcp4_cb_multiple_process_one_link() {
        // clear the vector of process in case other test executed before
        {
            let mut procs = PROCESSES.lock().unwrap();

            // no elements should match that condition
            procs.retain(|e| e.pid == u32::MAX);
        }

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // 192.168.1.2
            daddr: 3361999370,  // 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 5678,
            saddr: 3232235778,  // 192.168.1.2
            daddr: 168453320,   // 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 0,
        };
        let mut ptr = ipv4_tcp_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 2, "number of process incorrect");
    }

    #[test]
    fn group_bytes_bytes() {
        let bytes = 123;

        let (grouped, unit) = group_bytes(bytes);

        assert_eq!(grouped, 123.0);
        assert_eq!(unit as u32, DataUnit::Bytes as u32);
    }

    #[test]
    fn group_bytes_kbytes() {
        let bytes = 1024+512;

        let (grouped, unit) = group_bytes(bytes);

        assert_eq!(grouped, 1.5);
        assert_eq!(unit as u32, DataUnit::KBytes as u32);
    }

    #[test]
    fn group_bytes_mbytes() {
        let bytes = 1024*1024 + 512*1024;

        let (grouped, unit) = group_bytes(bytes);

        assert_eq!(grouped, 1.5);
        assert_eq!(unit as u32, DataUnit::MBytes as u32);
    }

    #[test]
    fn group_bytes_gbytes() {
        let bytes = 1024*1024*1024 + 512*1024*1024;

        let (grouped, unit) = group_bytes(bytes);

        assert_eq!(grouped, 1.5);
        assert_eq!(unit as u32, DataUnit::GBytes as u32);
    }

    #[test]
    fn group_bytes_tbytes() {
        let bytes = 1024*1024*1024*1024 + 512*1024*1024*1024;

        let (grouped, unit) = group_bytes(bytes);

        assert_eq!(grouped, 1.5);
        assert_eq!(unit as u32, DataUnit::TBytes as u32);
    }
}
