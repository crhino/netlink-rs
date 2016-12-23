mod socket_impl;

mod address;
pub use self::address::*;

mod msg;
pub use self::msg::*;

use socket::socket_impl::Socket as SocketImpl;

use std::mem::{size_of};

use libc::{AF_NETLINK, SOCK_RAW};

use std::convert::Into;
use std::io::{self, Write, Cursor};
use std::iter::{repeat};

use byteorder::{NativeEndian, WriteBytesExt, ReadBytesExt};

// #define NLMSG_ALIGNTO   4
const NLMSG_ALIGNTO: usize = 4;


/// Converts a value from host byte order to network byte order.
#[inline]
fn htons(hostshort: u16) -> u16 {
    hostshort.to_be()
}


/// Converts a value from network byte order to host byte order.
// #[inline]
// fn ntohs(netshort: u16) -> u16 {
//     u16::from_be(netshort)
// }

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

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Payload<'a> {
    None,
    Data(&'a [u8]),
    Ack(NlMsgHeader),
    Err(NlMsgHeader),
}

impl<'a> Payload<'a> {
    fn data(bytes: &'a [u8], len: usize) -> io::Result<(Payload<'a>, usize)> {
        use std::io::{ErrorKind, Error};

        let l = bytes.len();
        if l < len {
            Err(Error::new(ErrorKind::InvalidData, "length of bytes too small"))
        } else {
            Ok((Payload::Data(&bytes[..len]), len))
        }
    }

    fn nlmsg_error(bytes: &'a [u8]) -> io::Result<(Payload<'a>, usize)> {
        let mut cursor = Cursor::new(bytes);
        let err = try!(cursor.read_u32::<NativeEndian>());
        let n = cursor.position() as usize;
        let (hdr, n2) = try!(NlMsgHeader::from_bytes(&bytes[n..]));
        let num = n + n2;
        if err == 0 {
            Ok((Payload::Ack(hdr), num))
        } else {
            Ok((Payload::Err(hdr), num))
        }
    }

    fn bytes(&self) -> io::Result<Vec<u8>> {
        match self {
            &Payload::None => {
                Ok(vec!())
            },
            &Payload::Data(b) => {
                Ok(b.into())
            },
            &Payload::Ack(h) => {
                let mut vec = vec![];
                try!(vec.write_u32::<NativeEndian>(0));
                try!(vec.write(h.bytes()));
                Ok(vec)
            },
            &Payload::Err(h) => {
                let mut vec = vec![];
                try!(vec.write_u32::<NativeEndian>(1));
                try!(vec.write(h.bytes()));
                Ok(vec)
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Msg<'a> {
    header: NlMsgHeader,
    payload: Payload<'a>,
}

impl<'a> Msg<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> io::Result<(Msg<'a>, usize)> {
        let (hdr, n) = try!(NlMsgHeader::from_bytes(bytes));
        let (payload, n2) = match hdr.msg_type() {
            MsgType::Done => {
                (Payload::None, 0)
            },
            MsgType::Error => {
                try!(Payload::nlmsg_error(&bytes[n..]))
            },
            _ => {
                let msg_len = hdr.msg_length() as usize - nlmsg_header_length();
                try!(Payload::data(&bytes[n..], msg_len))
            },
        };

        Ok((Msg{
            header: hdr,
            payload: payload,
        }, n + n2))
    }

    pub fn new(hdr: NlMsgHeader, payload: Payload<'a>) -> Msg<'a> {
        Msg{
            header: hdr,
            payload: payload,
        }
    }

    pub fn bytes(&self) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = self.header.bytes().into();
        let mut payload = try!(self.payload.bytes());
        bytes.append(&mut payload);
        Ok(bytes)
    }

    pub fn header(&self) -> NlMsgHeader {
        self.header
    }

    pub fn payload(&self) -> &Payload<'a> {
        &self.payload
    }
}

// #[repr(C)]
// #[derive(Clone, Copy, Eq, PartialEq, Debug)]
// struct NlErr {
//     /// 0 if used as acknowledgement
//     err: u32,
//     /// Msg header that caused the error
//     hdr: NlMsgHeader,
// }

pub struct Socket {
    inner: SocketImpl,
    buf: Vec<u8>,
}

impl Socket {
    pub fn new<P: Into<i32>>(protocol: P) -> io::Result<Socket> {
        let s = try!(SocketImpl::new(AF_NETLINK, SOCK_RAW, protocol.into()));
        let bytes = 4096;
        let mut buf = Vec::with_capacity(bytes);
        buf.extend(repeat(0u8).take(bytes));
        Ok(Socket {
            inner: s,
            buf: buf,
        })
    }

    pub fn bind(&self, addr: NetlinkAddr) -> io::Result<()> {
        self.inner.bind(&addr.as_sockaddr())
    }

    pub fn close(&self) -> io::Result<()> {
        self.inner.close()
    }

    pub fn send<'a>(&self, messages: Vec<Msg<'a>>, addr: &NetlinkAddr)
        -> io::Result<usize> {
            let mut bytes = vec![];
            for m in messages {
                let mut b = try!(m.bytes());
                bytes.append(&mut b);
            }

            self.inner.sendto(bytes.as_slice(), 0, &addr.as_sockaddr())
        }

    pub fn recv<'a>(&'a mut self) -> io::Result<(NetlinkAddr, Vec<Msg<'a>>)> {
        let buffer = &mut self.buf[..];
        let (saddr, _) = try!(self.inner.recvfrom_into(buffer, 0));
        let addr = try!(sockaddr_to_netlinkaddr(&saddr));
        let mut messages = vec![];

        let mut n = 0;
        while let Ok((msg, num_bytes)) = Msg::from_bytes(&buffer[n..]) {
            n += num_bytes;
            let t = msg.header().msg_type();
            match t {
                MsgType::Done => {
                    break
                },
                _ => {
                    messages.push(msg);
                },
            }
        }

        Ok((addr, messages))
    }

// #define NLMSG_DATA(nlh)  ((void*)(((char*)nlh) + NLMSG_LENGTH(0)))
// #define NLMSG_NEXT(nlh,len)  ((len) -= NLMSG_ALIGN((nlh)->nlmsg_len), \
//                   (struct nlmsghdr*)(((char*)(nlh)) + NLMSG_ALIGN((nlh)->nlmsg_len)))
// #define NLMSG_PAYLOAD(nlh,len) ((nlh)->nlmsg_len - NLMSG_SPACE((len)))
}

/*
       NLMSG_DATA()
              Return a pointer to the payload associated with the passed
              nlmsghdr.

       NLMSG_NEXT()
              Get the next nlmsghdr in a multipart message.  The caller must
              check if the current nlmsghdr didn't have the NLMSG_DONE set—
              this function doesn't return NULL on end.  The len argument is
              an lvalue containing the remaining length of the message
              buffer.  This macro decrements it by the length of the message
              header.

       NLMSG_PAYLOAD()
              Return the length of the payload associated with the nlmsghdr.*

*/

// NLMSG_OK()
//        Return true if the netlink message is not truncated and is in
//        a form suitable for parsing.
// #define NLMSG_OK(nlh,len) ((len) >= (int)sizeof(struct nlmsghdr) && \
//                (nlh)->nlmsg_len >= sizeof(struct nlmsghdr) && \
//                (nlh)->nlmsg_len <= (len))
// #[inline(always)]
// fn nlmsg_ok(hdr: NlMsgHeader, len: usize) -> bool {
//     let hdrsize = size_of::<NlMsgHeader>() as u32;
//     let msglen = hdr.msg_length();
//     let len32 = len as u32;

//     len32 >= hdrsize && msglen >= hdrsize && msglen <= len32
// }

// NLMSG_ALIGN()
//       Round the length of a netlink message up to align it properly.
// #define NLMSG_ALIGN(len) ( ((len)+NLMSG_ALIGNTO-1) & ~(NLMSG_ALIGNTO-1) )
#[inline(always)]
fn nlmsg_align(len: usize) -> usize {
    (len + (NLMSG_ALIGNTO - 1)) & !(NLMSG_ALIGNTO - 1)
}

// #define NLMSG_HDRLEN     ((int) NLMSG_ALIGN(sizeof(struct nlmsghdr)))
#[inline(always)]
fn nlmsg_header_length() -> usize {
    nlmsg_align(size_of::<NlMsgHeader>())
}

// NLMSG_LENGTH()
//        Given the payload length, len, this macro returns the aligned
//        length to store in the nlmsg_len field of the nlmsghdr.
// #define NLMSG_LENGTH(len) ((len)+NLMSG_ALIGN(NLMSG_HDRLEN))
#[inline(always)]
fn nlmsg_length(len: usize) -> usize {
    len + nlmsg_align(nlmsg_header_length())
}

// NLMSG_SPACE()
//        Return the number of bytes that a netlink message with payload
//        of len would occupy.
// #define NLMSG_SPACE(len) NLMSG_ALIGN(NLMSG_LENGTH(len))
// #[inline(always)]
// fn nlmsg_space(len: usize) -> usize {
//     nlmsg_align(nlmsg_length(len))
// }

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{NativeEndian, WriteBytesExt};
    use Protocol;
    use std::io::Write;

    #[test]
    fn test_send_recv() {
        let send = Socket::new(Protocol::Usersock).unwrap();
        let mut recv = Socket::new(Protocol::Usersock).unwrap();
        let send_addr = NetlinkAddr::new(99, 0);
        let recv_addr = NetlinkAddr::new(100, 0);

        send.bind(send_addr).unwrap();
        recv.bind(recv_addr).unwrap();

        let bytes = [0,1,2,3,4,5];
        let mut shdr = NlMsgHeader::request();
        shdr.data_length(6).multipart().seq(1).pid(100);
        let msg = Msg::new(shdr, Payload::Data(&bytes));
        let msg2 = msg.clone();


        let mut donehdr = NlMsgHeader::done();
        donehdr.pid(100);
        let donemsg = Msg::new(donehdr, Payload::None);

        send.send(vec![msg, msg2, donemsg], &recv_addr).unwrap();

        let (ref addr, ref vec) = recv.recv().unwrap();
        assert_eq!(vec.len(), 2);

        let ref msg = vec.first().unwrap();
        assert_eq!(addr, &send_addr);
        if let &Payload::Data(b) = msg.payload() {
            assert_eq!(b, &bytes);
        } else {
            panic!("msg is not Data enum");
        }
    }

    #[test]
    fn test_payload_decode() {
        let bytes = [0,1,2,3,4,5];
        let (payload, n) = Payload::data(&bytes, bytes.len()).unwrap();
        assert_eq!(n, bytes.len());

        if let Payload::Data(b) = payload {
            assert_eq!(b, &bytes);
        } else {
            panic!("payload is not Data enum");
        }
    }

    #[test]
    fn test_payload_decode_with_err() {
        let mut bytes = vec![];
        bytes.write_u32::<NativeEndian>(1).unwrap();

        // Little endian only right now
        let expected = [20, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 0, 9, 0, 0, 0];
        let mut hdr = NlMsgHeader::request();
        hdr.data_length(4).pid(9).seq(1).dump();

        bytes.write(&expected).unwrap();

        let (p, n) = Payload::nlmsg_error(&bytes).unwrap();

        assert_eq!(n, bytes.len());
        if let Payload::Err(h) = p {
            assert_eq!(h, hdr);
        } else {
            panic!("payload is not Err enum");
        }
    }

    #[test]
    fn test_payload_decode_with_ack() {
        let mut bytes = vec![];
        bytes.write_u32::<NativeEndian>(0).unwrap();

        let mut hdr = NlMsgHeader::request();
        hdr.data_length(4).pid(9).seq(1).dump();

        bytes.write(&hdr.bytes()).unwrap();

        let (p, n) = Payload::nlmsg_error(&bytes).unwrap();

        assert_eq!(n, bytes.len());
        if let Payload::Ack(h) = p {
            assert_eq!(h, hdr);
        } else {
            panic!("payload is not Ack enum");
        }
    }

    #[test]
    fn test_msg_decode() {
        // Little endian only right now
        let mut hdr = NlMsgHeader::request();
        hdr.data_length(4).pid(9).seq(1).dump();
        let hdr_bytes = hdr.bytes();

        let data = [0,1,2,3];

        let mut bytes = vec![];
        bytes.write(&hdr_bytes).unwrap();
        bytes.write(&data).unwrap();
        // Random data
        bytes.write(&[1,1,1,1,1,1,1]).unwrap();

        let (msg, n) = Msg::from_bytes(&bytes).unwrap();
        assert_eq!(n, hdr_bytes.len() + data.len());
        assert_eq!(hdr, msg.header());

        if let &Payload::Data(b) = msg.payload() {
            assert_eq!(b, &data);
        } else {
            panic!("msg is not Data enum");
        }
    }

    #[test]
    fn test_msg_decode_with_err() {
        let mut hdr = NlMsgHeader::error();
        hdr.pid(9).seq(1);
        let hdr_bytes = hdr.bytes();

        let mut bytes = vec![];
        bytes.write(&hdr_bytes).unwrap();

        bytes.write_u32::<NativeEndian>(1).unwrap();
        let mut err_hdr = NlMsgHeader::request();
        err_hdr.data_length(4).pid(9).seq(1).dump();
        bytes.write(&err_hdr.bytes()).unwrap();

        let (msg, n) = Msg::from_bytes(&bytes).unwrap();
        assert_eq!(n, bytes.len());
        assert_eq!(hdr, msg.header());

        if let &Payload::Err(h) = msg.payload() {
            assert_eq!(h, err_hdr);
        } else {
            panic!("msg is not Err enum");
        }
    }
}
