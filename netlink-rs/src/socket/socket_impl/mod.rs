#![allow(trivial_casts)]
#![allow(dead_code)]
#![allow(unused_unsafe)]

use std::iter::{repeat};
use std::io::{Error, Result,};
use std::mem;
use std::ptr;
use std::ops::Drop;
use std::vec::{Vec,};

use libc::{
    c_void, size_t, socklen_t, sockaddr,
    socket, setsockopt, bind, send, recv, recvfrom,
    connect, getsockname,
    close,
    listen, sendto, accept,
    sendmsg, msghdr, iovec,
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

    pub fn getsockname(&self) -> Result<sockaddr> {
        let mut sa: sockaddr = unsafe { mem::zeroed() };
        let mut len: socklen_t = mem::size_of::<sockaddr>() as socklen_t;
        _try!(getsockname(self.fd,
              &mut sa as *mut sockaddr, &mut len as *mut socklen_t));
        assert_eq!(len, mem::size_of::<sockaddr>() as socklen_t);

        Ok(sa)
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
    pub fn bind(&self, address: &sockaddr) -> Result<()> {
        _try!(bind(self.fd, address, sockaddr_len()));
        Ok(())
    }

    pub fn sendto(&self, buffer: &[u8], flags: i32, sa: &sockaddr)
            -> Result<usize> {
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

    pub fn sendmsg(&self, msg: &[u8], data: &[u8], flags: i32, sa: &sockaddr)
            -> Result<usize> {
        let msg = unsafe {
            let msg_iovec = iovec {
                iov_base: msg as *const [u8] as *mut c_void,
                iov_len: msg.len() as size_t,
            };
            let data_iovec = iovec {
                iov_base: data as *const [u8] as *mut c_void,
                iov_len: data.len() as size_t,
            };
            let mut iovecs = [msg_iovec, data_iovec];
            msghdr{
                msg_name: sa as *const sockaddr as *mut c_void,
                msg_namelen: sockaddr_len(),
                msg_iov: iovecs.as_mut_ptr() as *mut iovec,
                msg_iovlen: 2,
                msg_control: ptr::null_mut(),
                msg_controllen: 0,
                msg_flags: 0,
            }
        };

        let sent = _try!(sendmsg(self.fd, &msg as *const msghdr, flags));
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
        // sockaddr_nl only has 12 bytes, still fits into 16 byte sockaddr
        assert!(sa_len <= sockaddr_len);
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

    pub fn connect(&self, address: &sockaddr) -> Result<()> {
        _try!(connect(self.fd, address as *const sockaddr, sockaddr_len()));
        Ok(())
    }

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
    use std::mem;
    use std::thread;
    use libc::{AF_NETLINK, SOCK_RAW,};
    use libc::{sa_family_t, in_addr, sockaddr, sockaddr_in, AF_INET,
        SOCK_STREAM, SOCK_DGRAM, SOL_SOCKET, SO_REUSEADDR};
    use std::net::{SocketAddr, ToSocketAddrs};

    fn socketaddr_to_sockaddr<T: ToSocketAddrs + ?Sized>(addr: &T) -> sockaddr {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        unsafe {
            match addr {
                SocketAddr::V4(v4) => {
                    let mut sa: sockaddr_in = mem::zeroed();
                    sa.sin_family = AF_INET as sa_family_t;
                    sa.sin_port = v4.port();
                    sa.sin_addr = *(&v4.ip().octets() as *const u8 as *const in_addr);
                    *(&sa as *const sockaddr_in as *const sockaddr)
                },
                SocketAddr::V6(_) => {
                    panic!("Not supported");
                    /*
                       let mut sa: sockaddr_in6 = mem::zeroed();
                       sa.sin6_family = AF_INET6 as u16;
                       sa.sin6_port = htons(v6.port());
                       (&sa as *const sockaddr_in6 as *const sockaddr)
                       */
                },
            }
        }
    }

    #[test]
    fn netlink_socket_works() {
        Socket::new(AF_NETLINK, SOCK_RAW, 0).unwrap();
    }

    #[test]
    fn some_basic_socket_stuff_works() {
        let socket = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();
        socket.setsockopt(SOL_SOCKET, SO_REUSEADDR, 1).unwrap();
        let sa = socketaddr_to_sockaddr("0.0.0.0:0");
        socket.bind(&sa).unwrap();
    }

    #[test]
    fn getsockname_works() {
        let s = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();
        let sa = socketaddr_to_sockaddr("127.0.0.1:0");
        s.bind(&sa).unwrap();
         assert_eq!(s.getsockname().unwrap().sa_family, sa.sa_family);
         // Skip port part since we are picking a random port.
         assert_eq!(s.getsockname().unwrap().sa_data[2..], sa.sa_data[2..]);
    }

    #[test]
    fn udp_communication_works() {
        let receiver = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();
        let sa = socketaddr_to_sockaddr("0.0.0.0:0");
        receiver.bind(&sa).unwrap();
        let address = receiver.getsockname().unwrap();

        let sender = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();

        assert_eq!(sender.sendto("abcd".as_bytes(), 0, &address).unwrap(), 4);
        let (_, received) = receiver.recvfrom(10, 0).unwrap();
        assert_eq!(received.len(), 4);
        // TODO: test the actual content
    }

    #[test]
    fn tcp_communication_works() {
        let listener = Socket::new(AF_INET, SOCK_STREAM, 0).unwrap();
        let sa = socketaddr_to_sockaddr("0.0.0.0:0");
        listener.bind(&sa).unwrap();
        listener.listen(10).unwrap();

        let address = listener.getsockname().unwrap();

        let thread = thread::spawn(move || {
            let (server, _) = listener.accept().unwrap();
            let data = server.recv(10, 0).unwrap();
            assert_eq!(data.len(), 4);
            // TODO: test the received content
        });

        let client = Socket::new(AF_INET, SOCK_STREAM, 0).unwrap();
        client.connect(&address).unwrap();
        let sent = client.send("abcd".as_bytes(), 0).unwrap();
        assert_eq!(sent, 4);

        thread.join().unwrap();
    }

    #[test]
    fn sendmsg_works() {
        let receiver = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();
        let sa = socketaddr_to_sockaddr("0.0.0.0:0");
        receiver.bind(&sa).unwrap();
        let address = receiver.getsockname().unwrap();

        let sender = Socket::new(AF_INET, SOCK_DGRAM, 0).unwrap();

        assert_eq!(sender.sendmsg(&[1,2,3,4],
                                  &[5,6,7,8],
                                  0,
                                  &address).unwrap(), 8);
        let (_, received) = receiver.recvfrom(10, 0).unwrap();
        assert_eq!(received.len(), 8);
    }
}
