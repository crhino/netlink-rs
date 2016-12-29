use super::{nlmsg_length, nlmsg_header_length};
use std::mem::{size_of};
use std::slice::{from_raw_parts};
use std::io::{self, Cursor};

use byteorder::{NativeEndian, ReadBytesExt};

#[derive(Clone, Copy)]
pub enum MsgType {
    /// Request
    Request,
    /// No op
    Noop,
    /// Error
    Error,
    /// End of a dump
    Done,
    /// Data lost
    Overrun,
    /// minimum type, below 10 for reserved control messages
    MinType,
    /// User defined type, passed to the user
    UserDefined(u16),
}

impl Into<u16> for MsgType {
    fn into(self) -> u16 {
        use self::MsgType::*;
        match self {
            Request => 0,
            Noop => 1,
            Error => 2,
            Done => 3,
            Overrun => 4,
            MinType => 10,
            UserDefined(i) => i,
        }
    }
}

impl From<u16> for MsgType {
    fn from(t: u16) -> MsgType {
        use self::MsgType::*;
        match t {
            0 => Request,
            1 => Noop,
            2 => Error,
            3 => Done,
            4 => Overrun,
            10 => MinType,
            i => UserDefined(i),
        }
    }
}

#[derive(Clone, Copy)]
enum Flags {
    /// It is request message.
    Request,
    /// Multipart message, terminated by NLMSG_DONE
    Multi,
    /// Reply with ack, with zero or error code
    Ack,
    /// Echo this request
    Echo,
}

impl Into<u16> for Flags {
    fn into(self) -> u16 {
        use self::Flags::*;
        match self {
            Request =>  1,
            Multi   =>  2,
            Ack     =>  4,
            Echo    =>  8,
        }
    }
}

/// Modifiers to GET request
#[derive(Clone, Copy)]
enum GetFlags {
    /// specify tree root
    Root,
    /// return all matching
    Match,
    /// atomic GET
    Atomic,
    /// (Root|Match)
    Dump,
}

impl Into<u16> for GetFlags {
    fn into(self) -> u16 {
        use self::GetFlags::*;
        match self {
            Root    =>  0x100,
            Match   =>  0x200,
            Atomic  =>  0x400,
            Dump    =>  0x100 | 0x200,
        }
    }
}

/// Modifiers to NEW request
#[derive(Clone, Copy)]
enum NewFlags {
    /// Override existing
    Replace,
    /// Do not touch, if it exists
    Excl,
    /// Create, if it does not exist
    Create,
    /// Add to end of list
    Append,
}

impl Into<u16> for NewFlags {
    fn into(self) -> u16 {
        use self::NewFlags::*;
        match self {
            Replace =>  0x100,
            Excl    =>  0x200,
            Create  =>  0x400,
            Append  =>  0x800,
        }
    }
}

// HEADER FORMAT
// __u32 nlmsg_len;    /* Length of message including header. */
// __u16 nlmsg_type;   /* Type of message content. */
// __u16 nlmsg_flags;  /* Additional flags. */
// __u32 nlmsg_seq;    /* Sequence number. */
// __u32 nlmsg_pid;    /* Sender port ID. */
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct NlMsgHeader {
    msg_length: u32,
    nl_type: u16,
    flags: u16,
    seq: u32,
    pid: u32,
}

impl NlMsgHeader {
    pub fn user_defined(t: u16) -> NlMsgHeader {
        NlMsgHeader {
            msg_length: nlmsg_header_length() as u32,
            nl_type: t,
            flags: Flags::Request.into(),
            seq: 0,
            pid: 0,
        }
    }

    pub fn request() -> NlMsgHeader {
        NlMsgHeader {
            msg_length: nlmsg_header_length() as u32,
            nl_type: MsgType::Request.into(),
            flags: Flags::Request.into(),
            seq: 0,
            pid: 0,
        }
    }

    pub fn done() -> NlMsgHeader {
        NlMsgHeader {
            msg_length: nlmsg_header_length() as u32,
            nl_type: MsgType::Done.into(),
            flags: Flags::Multi.into(),
            seq: 0,
            pid: 0,
        }
    }

    pub fn error() -> NlMsgHeader {
        NlMsgHeader {
            msg_length: nlmsg_length(nlmsg_header_length() + 4) as u32, // nlmsgerr
            nl_type: MsgType::Error.into(),
            flags: 0,
            seq: 0,
            pid: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<(NlMsgHeader, usize)> {
        let mut cursor = Cursor::new(bytes);
        let len = try!(cursor.read_u32::<NativeEndian>());
        let nl_type = try!(cursor.read_u16::<NativeEndian>());
        let flags = try!(cursor.read_u16::<NativeEndian>());
        let seq = try!(cursor.read_u32::<NativeEndian>());
        let pid = try!(cursor.read_u32::<NativeEndian>());

        Ok((NlMsgHeader{
            msg_length: len,
            nl_type: nl_type,
            flags: flags,
            seq: seq,
            pid: pid,
        }, cursor.position() as usize))
    }

    pub fn bytes(&self) -> &[u8] {
        let size = size_of::<NlMsgHeader>();
        unsafe {
            let head = self as *const NlMsgHeader as *const u8;
            from_raw_parts(head, size)
        }
    }

    pub fn msg_type(&self) -> MsgType {
        self.nl_type.into()
    }

    pub fn msg_length(&self) -> u32 {
        self.msg_length
    }

    /// Set message length
    pub fn data_length(&mut self, len: u32) -> &mut NlMsgHeader {
        self.msg_length = nlmsg_length(len as usize) as u32;
        self
    }

    /// Multipart message
    pub fn multipart(&mut self) -> &mut NlMsgHeader {
        self.flags |= Flags::Multi.into();
        self
    }

    /// Request acknowledgement
    pub fn ack(&mut self) -> &mut NlMsgHeader {
        self.flags |= Flags::Ack.into();
        self
    }

    /// Echo message
    pub fn echo(&mut self) -> &mut NlMsgHeader {
        self.flags |= Flags::Echo.into();
        self
    }

    /// Set sequence number
    pub fn seq(&mut self, n: u32) -> &mut NlMsgHeader {
        self.seq = n;
        self
    }

    /// Set PID number
    pub fn pid(&mut self, n: u32) -> &mut NlMsgHeader {
        self.pid = n;
        self
    }

    /// Override existing
    pub fn replace(&mut self) -> &mut NlMsgHeader {
        self.flags |= NewFlags::Replace.into();
        self
    }

    /// Do not touch, if it exists
    pub fn excl(&mut self) -> &mut NlMsgHeader {
        self.flags |= NewFlags::Excl.into();
        self
    }

    /// Create, if it does not exist
    pub fn create(&mut self) -> &mut NlMsgHeader {
        self.flags |= NewFlags::Create.into();
        self
    }

    /// Add to end of list
    pub fn append(&mut self) -> &mut NlMsgHeader {
        self.flags |= NewFlags::Append.into();
        self
    }

    /// specify tree root
    pub fn root(&mut self) -> &mut NlMsgHeader {
        self.flags |= GetFlags::Root.into();
        self
    }

    /// return all matching
    pub fn match_provided(&mut self) -> &mut NlMsgHeader {
        self.flags |= GetFlags::Match.into();
        self
    }

    /// atomic GET
    pub fn atomic(&mut self) -> &mut NlMsgHeader {
        self.flags |= GetFlags::Atomic.into();
        self
    }

    /// (Root|Match)
    pub fn dump(&mut self) -> &mut NlMsgHeader {
        self.flags |= GetFlags::Dump.into();
        self
    }
}

/*
http://linux.die.net/include/linux/netlink.h
/* Flags values */

#define NLM_F_REQUEST       1   /* It is request message.   */
#define NLM_F_MULTI     2   /* Multipart message, terminated by NLMSG_DONE */
#define NLM_F_ACK       4   /* Reply with ack, with zero or error code */
#define NLM_F_ECHO      8   /* Echo this request        */

/* Modifiers to GET request */
#define NLM_F_ROOT  0x100   /* specify tree root    */
#define NLM_F_MATCH 0x200   /* return all matching  */
#define NLM_F_ATOMIC    0x400   /* atomic GET       */
#define NLM_F_DUMP  (NLM_F_ROOT|NLM_F_MATCH)

/* Modifiers to NEW request */
#define NLM_F_REPLACE   0x100   /* Override existing        */
#define NLM_F_EXCL  0x200   /* Do not touch, if it exists   */
#define NLM_F_CREATE    0x400   /* Create, if it does not exist */
#define NLM_F_APPEND    0x800   /* Add to end of list       */

/*
   4.4BSD ADD       NLM_F_CREATE|NLM_F_EXCL
   4.4BSD CHANGE    NLM_F_REPLACE

   True CHANGE      NLM_F_CREATE|NLM_F_REPLACE
   Append       NLM_F_CREATE
   Check        NLM_F_EXCL
 */

#define NLMSG_NOOP      0x1 /* Nothing.     */
#define NLMSG_ERROR     0x2 /* Error        */
#define NLMSG_DONE      0x3 /* End of a dump    */
#define NLMSG_OVERRUN       0x4 /* Data lost        */

#define NLMSG_MIN_TYPE      0x10    /* < 0x10: reserved control messages */
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding() {
        // Little endian only right now
        let expected = [20, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 0, 9, 0, 0, 0];
        let mut hdr = NlMsgHeader::request();
        let bytes = hdr.data_length(4).pid(9).seq(1).dump().bytes();

        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_decoding() {
        // Little endian only right now
        let bytes = [20, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 0, 9, 0, 0, 0, 1, 1, 1];
        let mut h = NlMsgHeader::request();
        let expected = h.data_length(4).pid(9).seq(1).dump();

        let (hdr, n) = NlMsgHeader::from_bytes(&bytes).unwrap();
        assert_eq!(hdr, *expected);
        assert_eq!(n, 16);
    }
}
