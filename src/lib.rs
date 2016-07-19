#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate libc;

pub mod socket;

use std::convert::Into;

pub enum Protocol {
    Route,           /* 0    Routing/device hook              */
    Unused,          /* 1    Unused number                */
    Usersock,        /* 2    Reserved for user mode socket protocols  */
    Firewall,        /* 3    Firewalling hook             */
    INETDiag,        /* 4    INET socket monitoring           */
    Nflog,           /* 5    netfilter/iptables ULOG */
    Xfrm,            /* 6    ipsec */
    SELinux,         /* 7    SELinux event notifications */
    Iscsi,           /* 8    Open-iSCSI */
    Audit,           /* 9    auditing */
    FibLookup,       // 10
    Connector,       // 11
    Netfilter,       /* 12   netfilter subsystem */
    Ip6FW,           // 13
    Dnrtmsg,         /* 14   DECnet routing messages */
    KobjectUevent,   // 15  /* Kernel messages to userspace */
    Generic,         // 16
    SCSITransport,   // 18  /* SCSI Transports */
    Ecryptfs,        // 19
}

impl Into<i32> for Protocol {
    fn into(self) -> i32 {
        use Protocol::*;
        match self {
            Route => 0,
            Unused => 1,
            Usersock => 2,
            Firewall => 3,
            INETDiag => 4,
            Nflog => 5,
            Xfrm => 6,
            SELinux => 7,
            Iscsi => 8,
            Audit => 9,
            FibLookup => 10,
            Connector => 11,
            Netfilter => 12,
            Ip6FW => 13,
            Dnrtmsg => 14,
            KobjectUevent => 15,
            Generic => 16,
            SCSITransport => 18,
            Ecryptfs => 19,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
