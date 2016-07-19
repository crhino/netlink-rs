mod socket_impl;

mod address;
pub use self::address::*;

use socket::socket_impl::Socket as SocketImpl;

use libc::{AF_NETLINK, SOCK_RAW};
use Protocol;

use std::convert::Into;
use std::io;

/// Converts a value from host byte order to network byte order.
#[inline]
fn htons(hostshort: u16) -> u16 {
    hostshort.to_be()
}


/// Converts a value from network byte order to host byte order.
#[inline]
fn ntohs(netshort: u16) -> u16 {
    u16::from_be(netshort)
}


/// Converts a value from host byte order to network byte order.
#[inline]
fn htonl(hostlong: u32) -> u32 {
    hostlong.to_be()
}


/// Converts a value from network byte order to host byte order.
#[inline]
fn ntohl(netlong: u32) -> u32 {
    u32::from_be(netlong)
}

pub struct Socket(SocketImpl);

impl Socket {
    pub fn new(protocol: Protocol) -> io::Result<Socket> {
        let s = try!(SocketImpl::new(AF_NETLINK, SOCK_RAW, protocol.into()));
        Ok(Socket(s))
    }
}
