use std::{ptr, fmt, fs};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::fs::File;
use std::io::prelude::*;

use crate::PROCESSES;
use crate::dns::reverse_lookup;

extern crate num;

#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum DataUnit {
    Bytes,
    KBytes,
    MBytes,
    GBytes,
    TBytes,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Prot {
    TCP,
    UDP,
    NONE,
}

impl fmt::Display for Prot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "{}",
            match self {
                Prot::NONE => "NONE",
                Prot::TCP => "TCP",
                Prot::UDP => "UDP",
            },
        )
    }
}

#[derive(Clone)]
pub struct Process {
    pid: u32,
    name: String,
    //command: String,
    tlinks: Vec<Link>,
    ulinks: Vec<Link>,
    rx: isize,
    tx: isize,
    //status: u8, // TODO: enum
}

impl Process {
    pub fn new(pid: u32) -> Self {
        Process {
            pid: pid,
            name: String::new(),
            tlinks: Vec::new(),
            ulinks: Vec::new(),
            rx: 0,
            tx: 0,
        }
    }

    fn add_data(&mut self, size: isize, is_rx: u32) {
        match is_rx {
            0 => self.tx += size,
            1 => self.rx += size,
            _ => (),
        }
    }

    pub fn name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
    }

    pub fn rx(&mut self, rx: isize) -> &mut Self {
        self.rx = rx;
        self
    }

    pub fn tx(&mut self, tx: isize) -> &mut Self {
        self.tx = tx;
        self
    }

    pub fn get_pid(&self) -> u32 {
        self.pid
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_tlinks(&self) -> &Vec<Link> {
        &self.tlinks
    }

    pub fn get_ulinks(&self) -> &Vec<Link> {
        &self.ulinks
    }

    pub fn get_rx_tx(&self) -> (isize, isize) {
        (self.rx, self.tx)
    }

    pub fn print_tlinks(&self) {
        for l in self.tlinks.iter() {
            println!("{}", l);
        }
    }

    pub fn print_ulinks(&self) {
        for l in self.ulinks.iter() {
            println!("{}", l);
        }
    }

    pub fn to_string_with_links(&self) -> String {
        let mut buffer: String = self.to_string();
        buffer.push('\n');

        for l in self.tlinks.iter() {
            buffer.push_str("    ");
            buffer.push_str(l.to_string().as_str());
            buffer.push('\n');
        }

        for l in self.ulinks.iter() {
            buffer.push_str("    ");
            buffer.push_str(l.to_string().as_str());
            buffer.push('\n');
        }
        buffer.push('\n');

        buffer
    }

    pub fn overview_str(&self) -> String {
        format!("{} ({})", self.name, self.pid)
    }

    pub fn data_amount_str(&self) -> String {
        let (rx, rx_unit) = group_bytes(self.rx);
        let (tx, tx_unit) = group_bytes(self.tx);

        format!(" RX:{:.2}{u0} TX:{:.2}{u1}",
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

    pub fn get_all_info(&self) ->
        (u32, &String, &Vec<Link>, &Vec<Link>, isize, isize)
    {
        (self.pid, &self.name, &self.tlinks, &self.ulinks, self.rx, self.tx)
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
            f, "{} ({}) RX:{:.2}{u0} TX:{:.2}{u1}",
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

#[derive(Clone)]
pub struct Link {
    saddr: IpAddr,
    daddr: IpAddr,
    lport: u16,
    dport: u16,
    rx: isize,
    tx: isize,
    prot: Prot,
    domain: String,
}

impl Link {
    pub fn new(saddr: IpAddr, daddr: IpAddr, lport: u16, dport: u16) -> Link {
        Link {
            saddr,
            daddr,
            lport,
            dport,
            rx: 0,
            tx: 0,
            prot: Prot::NONE,
            domain: String::new(),
        }
    }

    pub fn add_data(&mut self, size: isize, is_rx: u32) {
        match is_rx {
            0 => self.tx += size,
            1 => self.rx += size,
            _ => (),
        }
    }

    pub fn rx(&mut self, rx: isize) -> &mut Self {
        self.rx = rx;
        self
    }

    pub fn tx(&mut self, tx: isize) -> &mut Self {
        self.tx = tx;
        self
    }

    pub fn prot(&mut self, prot: Prot) -> &mut Self {
        self.prot = prot;
        self
    }

    pub fn domain(&mut self, name: String) -> &mut Self {
        self.domain = name;
        self
    }

    pub fn get_saddr(&self) -> String {
        String::from(&self.saddr.to_string())
    }

    pub fn get_daddr(&self) -> String {
        String::from(&self.daddr.to_string())
    }

    pub fn get_lport(&self) -> u16 {
        self.lport
    }

    pub fn get_dport(&self) -> u16 {
        self.dport
    }

    pub fn get_rx_tx(&self) -> (isize, isize) {
        (self.rx, self.tx)
    }

    pub fn get_prot(&self) -> u8 {
        self.prot as u8
    }

    pub fn get_domain(&self) -> &String {
        &self.domain
    }

    pub fn get_all_info(&self) ->
        (String, String, u16, u16, isize, isize, u8, &String)
    {
        (self.get_saddr(), self.get_daddr(), self.lport, self.dport, self.rx, self.tx, self.prot as u8, self.get_domain())
    }
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.saddr == other.saddr
            && self.daddr == other.daddr
            && self.lport == other.lport
            && self.dport == other.dport
            && self.prot == other.prot
    }
}
impl Eq for Link {}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (rx, rx_unit) = group_bytes(self.rx);
        let (tx, tx_unit) = group_bytes(self.tx);

        //write!(
        //    f, "\t{p} {}:{} <-> {}:{} RX: {:.2}{u0} TX: {:.2}{u1}",
        //    self.saddr,
        //    self.lport,
        //    self.daddr,
        //    self.dport,
        //    rx,
        //    tx,
        //    u0 = match rx_unit {
        //        DataUnit::Bytes => "B",
        //        DataUnit::KBytes => "KB",
        //        DataUnit::MBytes => "MB",
        //        DataUnit::GBytes => "GB",
        //        DataUnit::TBytes => "TB",
        //    },
        //    u1 = match tx_unit {
        //        DataUnit::Bytes => "B",
        //        DataUnit::KBytes => "KB",
        //        DataUnit::MBytes => "MB",
        //        DataUnit::GBytes => "GB",
        //        DataUnit::TBytes => "TB",
        //    },
        //    p = match self.prot {
        //        Prot::NONE => "NONE",
        //        Prot::TCP => "TCP",
        //        Prot::UDP => "UDP",
        //    }
        //)

        let destination = if self.domain.is_empty() {
            self.daddr.to_string().to_owned()
        } else {
            self.domain.to_owned()
        };

        write!(
            f, "    {p} {}:{} <-> {}:{} RX: {:.2}{u0} TX: {:.2}{u1}",
            self.saddr,
            self.lport,
            destination,//self.domain,
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
            p = match self.prot {
                Prot::NONE => "NONE",
                Prot::TCP => "TCP",
                Prot::UDP => "UDP",
            }
        )
    }
}

// TODO: may need to separate TCP/UDP if we track the connection state or do other specific things
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

pub fn tcp4_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv4(x);

        let p = Process::new(data.pid);

        let mut l = Link::new(
            IpAddr::V4( Ipv4Addr::from(data.saddr.to_be()) ),
            IpAddr::V4( Ipv4Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );
        l.prot(Prot::TCP);

        update_procs_and_links(p, l, data.size as isize, data.is_rx, Prot::TCP);
    })
}

pub fn tcp6_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv6(x);

        let p = Process::new(data.pid);

        let mut l = Link::new(
            IpAddr::V6( Ipv6Addr::from(data.saddr.to_be()) ),
            IpAddr::V6( Ipv6Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );
        l.prot(Prot::TCP);

        update_procs_and_links(p, l, data.size as isize, data.is_rx, Prot::TCP);
    })
}

pub fn udp4_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv4(x);

        let p = Process::new(data.pid);

        let mut l = Link::new(
            IpAddr::V4( Ipv4Addr::from(data.saddr.to_be()) ),
            IpAddr::V4( Ipv4Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );
        l.prot(Prot::UDP);

        update_procs_and_links(p, l, data.size as isize, data.is_rx, Prot::UDP);
    })
}

pub fn udp6_cb() -> Box<dyn FnMut(&[u8]) + Send> {
    Box::new(|x| {
        let data = parse_struct_ipv6(x);

        let p = Process::new(data.pid);

        let mut l = Link::new(
            IpAddr::V6( Ipv6Addr::from(data.saddr.to_be()) ),
            IpAddr::V6( Ipv6Addr::from(data.daddr.to_be()) ),
            data.lport,
            data.dport,
        );
        l.prot(Prot::UDP);

        update_procs_and_links(p, l, data.size as isize, data.is_rx, Prot::UDP);
    })
}

///
/// Record the current network connection.
///
fn update_procs_and_links(
    mut p: Process, mut l: Link, packets_size: isize,
    is_rx: u32, prot: Prot
)
{
    let mut procs = PROCESSES.lock().unwrap();

    if procs.contains(&p) {
        /*
         * We have already seen this process having network connection open.
         */
        let known_p = procs.iter_mut().find(|x| x.pid == p.pid).unwrap();

        known_p.add_data(packets_size, is_rx);

        let links = if prot == Prot::TCP { &mut known_p.tlinks } else { &mut known_p.ulinks };

        if links.contains(&l) {
            let known_link = links.iter_mut().find(|x| **x == l).unwrap();

            known_link.add_data(packets_size, is_rx);
        } else {

            if l.daddr.is_global() {
                let (host, _service) = reverse_lookup(l.daddr, l.dport);
                l.domain(host);
            }

            l.add_data(packets_size, is_rx);

            links.push(l);
        }
    } else {
        /*
         * First time we see this process communicating over the network.
         */
        let path_comm = format!("/proc/{}/comm", p.pid);
        let content_comm = fs::read_to_string(path_comm);
        //let path_cmdline = format!("/proc/{}/cmdline", data.pid);
        //let content_cmdline = fs::read_to_string(path_cmdline);

        // TODO: add date (mm-dd-yy)

        let name = match content_comm {
            Ok(mut content) => { content.pop(); content },
            Err(_error) => String::from("file not found"),
        };
        // TODO: some kind of verbose mode
        //let _cmdline = match content_cmdline {
        //    Ok(mut content) => { content.pop(); content },
        //    Err(error) => String::from("file not found"),
        //};

        if l.daddr.is_global() {
            let (host, _service) = reverse_lookup(l.daddr, l.dport);
            l.domain(host);
        }

        p.name(name);
        p.add_data(packets_size, is_rx);

        l.add_data(packets_size, is_rx);

        let links = if prot == Prot::TCP { &mut p.tlinks } else { &mut p.ulinks };

        links.push(l);
        procs.push(p);
    }
}

fn parse_struct_ipv4(addr: &[u8]) -> ipv4_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv4_data_t) }
}

fn parse_struct_ipv6(addr: &[u8]) -> ipv6_data_t {
    unsafe { ptr::read(addr.as_ptr() as *const ipv6_data_t) }
}

fn group_bytes(bytes: isize) -> (f64, DataUnit) {
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
 * Typical TCP traffic interception (Sekhmet output) for iperf:
 *     iperf3 (221251):
 *            10.0.10.100:5201 <-> 10.0.10.200:49289 RX: 411 TX: 299
 *            10.0.10.100:5201 <-> 10.0.10.200:47159 RX: 5368709120 TX: 0
 *     iperf3 (222684):
 *            10.0.10.200:49289 <-> 10.0.10.100:5201 RX: 299 TX: 411
 *            10.0.10.200:47159 <-> 10.0.10.100:5201 RX: 0 TX: 5368709120
 *
 * We care only about the link with a lot of data and either TX or RX at 0. The other corresponds
 * to the setup of the communication between client/server. We want to be sure that we intercept
 * the same amount of data reported by to be transferred by iperf. The actual intercepted traffic
 * should be slightly higher du to the packets used to establish the comm (precisely 37 bytes,
 * based on observations => hardcoded in the testing script, works for now...). Note that RX may be
 * different than TX due to packet drops (normal behavior).
 *
 * For UDP:
 *     iperf3 (314305) RX:303.00B TX:500.00MB:
 *           UDP 10.0.10.100:57922 <-> 10.0.10.100:5201 RX: 4.00B TX: 500.00MB
 *     iperf3 (314133) RX:498.78MB TX:303.00B:
 *           UDP 10.0.10.100:5201 <-> 0.0.0.0:0 RX: 4.00B TX: 0.00B
 *           UDP 10.0.10.100:5201 <-> 10.0.10.100:57922 RX: 498.78MB TX: 4.00B
 *
 */
pub fn log_iperf_to_file() -> std::io::Result<()> {
    let procs = PROCESSES.lock().unwrap();
    let mut tcp4_rx = 0;
    let mut tcp4_tx = 0;
    let mut tcp6_rx = 0;
    let mut tcp6_tx = 0;
    let mut udp4_rx = 0;
    let mut udp4_tx = 0;
    let mut udp6_rx = 0;
    let mut udp6_tx = 0;

    for p in procs.iter() {
        if p.name == String::from("iperf3") {
            // TCP
            for l in p.get_tlinks() {
                if l.rx == 0 {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { tcp4_tx = l.tx; } else { tcp6_tx = l.tx }
                } else if l.tx == 0 {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { tcp4_rx = l.rx; } else { tcp6_rx = l.rx }
                }
            }

            // UDP
            for l in p.get_ulinks() {
                if l.rx == 4 && !l.daddr.is_unspecified() {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { udp4_tx = l.tx; } else { udp6_tx = l.tx }
                } else if l.tx == 4 && !l.daddr.is_unspecified() {
                    println!("{}", l);
                    if l.saddr.is_ipv4() { udp4_rx = l.rx; } else { udp6_rx = l.rx }
                }
            }
        }
    }

    let output = format!(
        "{{
            \"tcp4\": {{ \"rx\": {}, \"tx\": {} }},
            \"tcp6\": {{ \"rx\": {}, \"tx\": {} }},
            \"udp4\": {{ \"rx\": {}, \"tx\": {} }},
            \"udp6\": {{ \"rx\": {}, \"tx\": {} }}
        }}",
        tcp4_rx, tcp4_tx, tcp6_rx, tcp6_tx,
        udp4_rx, udp4_tx, udp6_rx, udp6_tx
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

    fn remove_all_procs() {
        // clear the vector of process in case other test executed before
        let procs = PROCESSES.lock();

        match procs {
            Ok(mut x) => {
                // no elements should match that condition
                x.retain(|e| e.pid == u32::MAX);
            },
            _ => (),
        }
    }

    #[test]
    fn tcp4_cb_one_process_multiple_links() {
        remove_all_procs();

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 567890,
            is_rx: 0,
        };
        let mut ptr = tcp4_cb();

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
        assert_eq!(c.prot, Prot::TCP);
    }

    #[test]
    fn tcp4_cb_multiple_process_one_link() {
        remove_all_procs();

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 5678,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 0,
        };
        let mut ptr = tcp4_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 2, "number of process incorrect");
    }

    #[test]
    fn tcp6_cb_one_process_multiple_links() {
        remove_all_procs();

        let data0 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 567890,
            is_rx: 0,
        };
        let mut ptr = tcp6_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 1, "number of process incorrect");

        let p = procs.iter().next().unwrap();
        let c = p.tlinks.iter().next().unwrap();
        let ip = IpAddr::V6( Ipv6Addr::new(0xfe80, 0, 0, 0, 0x4c9f, 0x5cff, 0xfedc, 0x82c9) );

        assert_eq!(p.pid, 1234, "pid incorrect");
        assert_ne!(p.name, "", "process name empty");
        assert_eq!(p.rx, 56789, "process rx incorrect");
        assert_eq!(p.tx, 567890, "process tx incorrect");
        assert_eq!(c.saddr, ip, "source ip address incorrect");
        assert_eq!(c.daddr, ip, "destination ip address incorrect");
        assert_eq!(c.lport, 4321, "local port incorrect");
        assert_eq!(c.dport, 80, "destination port incorrect");
        assert_eq!(c.rx, 56789, "rx size incorrect");
        assert_eq!(c.tx, 567890, "tx size incorrect");
        assert_eq!(c.prot, Prot::TCP);
    }

    #[test]
    fn tcp6_cb_multiple_process_one_link() {
        remove_all_procs();

        let data0 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv6_data_t {
            pid: 5678,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 0,
        };
        let mut ptr = tcp6_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 2, "number of process incorrect");
    }

    #[test]
    fn udp4_cb_one_process_multiple_links() {
        remove_all_procs();

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 567890,
            is_rx: 0,
        };
        let mut ptr = udp4_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 1, "number of process incorrect");

        let p = procs.iter().next().unwrap();
        let c = p.ulinks.iter().next().unwrap();
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
        assert_eq!(c.prot, Prot::UDP, "protocol is incorrect");
    }

    #[test]
    fn udp4_cb_multiple_process_one_link() {
        remove_all_procs();

        let data0 = ipv4_data_t {
            pid: 1234,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv4_data_t {
            pid: 5678,
            saddr: 33663168,    // (little endian) 192.168.1.2
            daddr: 3361999370,  // (little endian) 10.10.100.200
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 0,
        };
        let mut ptr = udp4_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 2, "number of process incorrect");
    }

    #[test]
    fn udp6_cb_one_process_multiple_links() {
        remove_all_procs();

        let data0 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 567890,
            is_rx: 0,
        };
        let mut ptr = udp6_cb();

        ptr( unsafe {any_as_u8_slice(&data0)} );
        ptr( unsafe {any_as_u8_slice(&data1)} );

        let procs = PROCESSES.lock().unwrap();

        assert_eq!(procs.len(), 1, "number of process incorrect");

        let p = procs.iter().next().unwrap();
        let c = p.ulinks.iter().next().unwrap();
        let ip = IpAddr::V6( Ipv6Addr::new(0xfe80, 0, 0, 0, 0x4c9f, 0x5cff, 0xfedc, 0x82c9) );

        assert_eq!(p.pid, 1234, "pid incorrect");
        assert_ne!(p.name, "", "process name empty");
        assert_eq!(p.rx, 56789, "process rx incorrect");
        assert_eq!(p.tx, 567890, "process tx incorrect");
        assert_eq!(c.saddr, ip, "source ip address incorrect");
        assert_eq!(c.daddr, ip, "destination ip address incorrect");
        assert_eq!(c.lport, 4321, "local port incorrect");
        assert_eq!(c.dport, 80, "destination port incorrect");
        assert_eq!(c.rx, 56789, "rx size incorrect");
        assert_eq!(c.tx, 567890, "tx size incorrect");
        assert_eq!(c.prot, Prot::UDP, "protocol is incorrect");
    }

    #[test]
    fn udp6_cb_multiple_process_one_link() {
        remove_all_procs();

        let data0 = ipv6_data_t {
            pid: 1234,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 1,
        };
        let data1 = ipv6_data_t {
            pid: 5678,
            saddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            daddr: 267854308077474350974080065079001252094, // (LE) fe80::4c9f:5cff:fedc:82c9
            lport: 4321,
            dport: 80,
            size: 56789,
            is_rx: 0,
        };
        let mut ptr = udp6_cb();

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
