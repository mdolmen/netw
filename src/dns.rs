use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs, SocketAddr};
use std::mem::{transmute, size_of_val};
use std::ffi::CStr;
use socket2::SockAddr;

use libc::{sockaddr, getnameinfo, c_char, c_int, socklen_t};

const MAX_HOST_LEN: socklen_t = 256;
const MAX_SERV_LEN: socklen_t = 256;

#[derive(Copy, Clone)]
enum Flags {
    NiNumerichost  = 1,     // Don't try to look up hostname.
    NiNumericser   = 2,     // Don't convert port number to name.
    NiNofqdn       = 4,     // Only return nodename portion.
    NiNamereqd     = 8,     // Don't return numeric addresses.
    NiDgram        = 16,	// Look up UDP service rather than TCP.
}

///
/// Get the domain name associated to an IP address.
///
pub fn reverse_lookup(addr: IpAddr, port: u16) -> (String, String) {
    let socket = SocketAddr::new(addr, port);
    let sock: SockAddr = socket.into();

    let mut c_host = [0 as c_char; MAX_HOST_LEN as usize];
    let mut c_serv = [0 as c_char; MAX_SERV_LEN as usize];

    unsafe {
        getnameinfo(
            sock.as_ptr(),
            sock.len() as socklen_t,
            c_host.as_mut_ptr(), MAX_HOST_LEN,
            c_serv.as_mut_ptr(), MAX_SERV_LEN,
            Flags::NiNofqdn as c_int,
        )
    };

    let h = unsafe { CStr::from_ptr(c_host.as_ptr()) };
    let s = unsafe { CStr::from_ptr(c_serv.as_ptr()) };

    let host = h.to_string_lossy().into_owned();
    let serv = s.to_string_lossy().into_owned();

    (host, serv)
}

/*
 * TESTS
 */

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn reverse_lookup_ipv4() {
        let ipv4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let (host, serv) = reverse_lookup(ipv4, 443);

        assert_eq!(host, "localhost".to_string());
        assert_eq!(serv, "https".to_string());
    }

    #[test]
    fn reverse_lookup_ipv6() {
        let ipv6 = IpAddr::V6(Ipv6Addr::new(0x2606,0x4700,0x3033,0,0,0,0x681f,0x4bdf));

        let (host, serv) = reverse_lookup(ipv6, 443);

        assert_eq!(host, "2606:4700:3033::681f:4bdf".to_string());
        assert_eq!(serv, "https".to_string());
    }
}
