use libc::{AF_NETLINK, sa_family_t, sockaddr, c_ushort};

use std::mem;

use socket::{ntohl, htonl};

#[repr(C)]
#[derive(Clone, Copy)]
struct sockaddr_nl {
    pub nl_family: sa_family_t,
    nl_pad: c_ushort,
    pub nl_pid: u32,
    pub nl_groups: u32,
}

#[derive(Clone, Copy)]
pub struct NetlinkAddr(sockaddr_nl);

impl NetlinkAddr {
    pub fn new(pid: u32, groups: u32) -> NetlinkAddr {
        NetlinkAddr(sockaddr_nl {
            nl_family: AF_NETLINK as sa_family_t,
            nl_pad: 0,
            nl_pid: htonl(pid),
            nl_groups: htonl(groups),
        })
    }

    pub fn pid(&self) -> u32 {
        ntohl(self.0.nl_pid)
    }

    pub fn groups(&self) -> u32 {
        ntohl(self.0.nl_groups)
    }
}

impl NetlinkAddr {
    fn as_sockaddr(&self) -> sockaddr {
        let sa = self.0;
        unsafe {
            *(&sa as *const sockaddr_nl as *const sockaddr)
        }
    }
}

pub fn sockaddr_to_netlinkaddr(sa: &sockaddr) -> NetlinkAddr {
    match sa.sa_family as i32 {
        AF_NETLINK => {
            let snl: &sockaddr_nl = unsafe { mem::transmute(sa) };
            let pid = ntohl(snl.nl_pid);
            let groups = ntohl(snl.nl_groups);
            NetlinkAddr::new(pid, groups)
        },
        _ => {
            unreachable!("Not supported")
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
        let nl2 = sockaddr_to_netlinkaddr(&sockaddr);
        assert_eq!(nladdr.pid(), nl2.pid());
        assert_eq!(nladdr.groups(), nl2.groups());
    }
}
