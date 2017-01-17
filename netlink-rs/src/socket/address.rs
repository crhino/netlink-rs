use libc::{AF_NETLINK, sa_family_t, sockaddr, c_ushort};

use std::{fmt, mem};
use std::io::{self, ErrorKind};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct sockaddr_nl {
    pub nl_family: sa_family_t,
    nl_pad: c_ushort,
    pub nl_pid: u32,
    pub nl_groups: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NetlinkAddr(sockaddr_nl);

impl NetlinkAddr {
    pub fn new(pid: u32, groups: u32) -> NetlinkAddr {
        NetlinkAddr(sockaddr_nl {
            nl_family: AF_NETLINK as sa_family_t,
            nl_pad: 0,
            nl_pid: pid,
            nl_groups: groups,
        })
    }

    pub fn pid(&self) -> u32 {
        self.0.nl_pid
    }

    pub fn groups(&self) -> u32 {
        self.0.nl_groups
    }

    pub fn as_sockaddr(&self) -> sockaddr {
        let sa = self.0;
        unsafe { *(&sa as *const sockaddr_nl as *const sockaddr) }
    }
}

impl fmt::Debug for NetlinkAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "<NetlinkAddr "));

        // only report unusual values for nl_family and nl_pad
        if self.0.nl_family != AF_NETLINK as sa_family_t {
            try!(write!(f, "[nl_family: {}]", self.0.nl_family));
        }

        if self.0.nl_pad != 0 {
            try!(write!(f, "[nl_pad: {}]", self.0.nl_pad));
        }

        try!(write!(f, "pid={} groups={}>", self.0.nl_pid, self.0.nl_groups));

        Ok(())
    }
}

pub fn sockaddr_to_netlinkaddr(sa: &sockaddr) -> io::Result<NetlinkAddr> {
    match sa.sa_family as i32 {
        AF_NETLINK => {
            let snl: &sockaddr_nl = unsafe { mem::transmute(sa) };
            let pid = snl.nl_pid;
            let groups = snl.nl_groups;
            Ok(NetlinkAddr::new(pid, groups))
        },
        _ => {
            Err(io::Error::new(ErrorKind::InvalidInput, "sockaddr is not Netlink family"))
        }
    }
}

#[cfg(test)]
mod tests {
    use libc::{AF_NETLINK, sa_family_t};
    use super::*;

    #[test]
    fn netlink_addr_and_sockaddr() {
        let nladdr = NetlinkAddr::new(0, 10);
        let sockaddr = nladdr.as_sockaddr();
        assert_eq!(sockaddr.sa_family, AF_NETLINK as sa_family_t);
        let nl2 = sockaddr_to_netlinkaddr(&sockaddr).unwrap();
        assert_eq!(nladdr.pid(), nl2.pid());
        assert_eq!(nladdr.groups(), nl2.groups());
    }
}
