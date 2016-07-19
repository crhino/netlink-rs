#![allow(trivial_casts)]

use std::iter::{repeat, };
use std::io::{Error, Result,};
use std::mem;
use std::ops::Drop;
use std::vec::{Vec,};
use std::convert::{AsRef};

use libc::{
    c_void, size_t, socklen_t, sockaddr,
    socket, setsockopt, bind, send, recv, recvfrom,
    close,
    listen, sendto, accept,
    shutdown,
};

macro_rules! _try {
    ( $x:expr ) => {{
        let value = unsafe { $x };
        if value == -1 {
            return Err(Error::last_os_error());
        }
        value
    }};
}

fn sockaddr_len() -> socklen_t {
    let struct_size = mem::size_of::<sockaddr>();
    let v = struct_size as socklen_t;
    assert_eq!(v as usize, struct_size);
    v
}

#[derive(Debug)]
pub struct Socket {
    fd: i32,
}


impl Socket {
    pub fn new(family: i32, socket_type: i32, protocol: i32) -> Result<Socket> {
        let fd = _try!(socket(family, socket_type, protocol));
        Ok(Socket { fd: fd })
    }

    /// Returns the underlying file descriptor.
    pub fn fileno(&self) -> i32 {
        self.fd
    }

    pub fn setsockopt<T>(&self, level: i32, name: i32, value: T) -> Result<()> {
        unsafe {
            let value = &value as *const T as *const c_void;
            _try!(setsockopt(
                    self.fd, level, name, value, sockaddr_len()));
        }
        Ok(())
    }

    /// Binds socket to an address
    pub fn bind<T: AsRef<sockaddr>>(&self, address: &T) -> Result<()> {
        let sa = address.as_ref();
        _try!(bind(self.fd, sa, sockaddr_len()));
        Ok(())
    }

    pub fn sendto<T: AsRef<sockaddr>>(&self, buffer: &[u8], flags: i32, address: &T)
            -> Result<usize> {
        let sa = address.as_ref();
        let sent = _try!(
            sendto(self.fd, buffer.as_ptr() as *const c_void,
            buffer.len() as size_t, flags, sa as *const sockaddr,
            sockaddr_len()));
        Ok(sent as usize)
    }

    pub fn send(&self, buffer: &[u8], flags: i32)
            -> Result<usize> {
        let sent = _try!(
            send(self.fd, buffer.as_ptr() as *const c_void, buffer.len() as size_t, flags));
        Ok(sent as usize)
    }

    /// Receives data from a remote socket and returns it with the address of the socket.
    pub fn recvfrom(&self, bytes: usize, flags: i32) -> Result<(sockaddr, Box<[u8]>)> {
        let mut a = Vec::with_capacity(bytes);

        // This is needed to get some actual elements in the vector, not just a capacity
        a.extend(repeat(0u8).take(bytes));

        let (socket_addr, received) = try!(self.recvfrom_into(&mut a[..], flags));

        a.truncate(received);
        Ok((socket_addr, a.into_boxed_slice()))
    }

    /// Similar to `recvfrom` but receives to predefined buffer and returns the number
    /// of bytes read.
    pub fn recvfrom_into(&self, buffer: &mut [u8], flags: i32) -> Result<(sockaddr, usize)> {
        let mut sa: sockaddr = unsafe { mem::zeroed() };
        let sockaddr_len = sockaddr_len();
        let mut sa_len: socklen_t = sockaddr_len;
        let received = _try!(
            recvfrom(self.fd, buffer.as_ptr() as *mut c_void, buffer.len() as size_t, flags,
            &mut sa as *mut sockaddr, &mut sa_len as *mut socklen_t));
        assert_eq!(sa_len, sockaddr_len);
        Ok((sa, received as usize))
    }

    /// Returns up to `bytes` bytes received from the remote socket.
    pub fn recv(&self, bytes: usize, flags: i32) -> Result<Box<[u8]>> {
        let mut a = Vec::with_capacity(bytes);

        // This is needed to get some actual elements in the vector, not just a capacity
        a.extend(repeat(0u8).take(bytes));

        let received = try!(self.recv_into(&mut a[..], flags));

        a.truncate(received);
        Ok(a.into_boxed_slice())
    }

    /// Similar to `recv` but receives to predefined buffer and returns the number
    /// of bytes read.
    pub fn recv_into(&self, buffer: &mut [u8], flags: i32) -> Result<usize> {
        let received = _try!(recv(self.fd, buffer.as_ptr() as *mut c_void, buffer.len() as size_t, flags));
        Ok(received as usize)
    }

    // pub fn connect<T: ToSocketAddrs + ?Sized>(&self, toaddress: &T) -> Result<()> {
    //     let address = try!(tosocketaddrs_to_sockaddr(toaddress));
    //     _try!(connect(self.fd, &address as *const sockaddr, sockaddr_len()));
    //     Ok(())
    // }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        _try!(listen(self.fd, backlog));
        Ok(())
    }

    pub fn accept(&self) -> Result<(Socket, sockaddr)> {
        let mut sa: sockaddr = unsafe { mem::zeroed() };
        let sockaddr_len = sockaddr_len();
        let mut sa_len: socklen_t = sockaddr_len;

        let fd = _try!(
            accept(self.fd, &mut sa as *mut sockaddr, &mut sa_len as *mut socklen_t));
        assert_eq!(sa_len, sockaddr_len);
        Ok((Socket { fd: fd }, sa))
    }

    pub fn close(&self) -> Result<()> {
        _try!(close(self.fd));
        Ok(())
    }

    pub fn shutdown(&self, how: i32) -> Result<()> {
        _try!(shutdown(self.fd, how));
        Ok(())
    }
}


impl Drop for Socket {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::{AF_NETLINK, SOCK_RAW,};

    #[test]
    fn netlink_socket_works() {
        let socket = Socket::new(AF_NETLINK, SOCK_RAW, 0).unwrap();
    }
}
