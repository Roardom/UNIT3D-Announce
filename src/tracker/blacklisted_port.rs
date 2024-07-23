use std::ops::{Deref, DerefMut};

use indexmap::IndexSet;

pub struct Set(IndexSet<u16>);

impl Default for Set {
    #[rustfmt::skip]
    fn default() -> Set {
        let mut set = IndexSet::from([
            // Hyper Text Transfer Protocol (HTTP) - port used for web traffic
            8080,
            8081,
            // Kazaa - peer-to-peer file sharing, some known vulnerabilities, and at least one worm (Benjamin) targeting it.
            1214,
            // IANA registered for Microsoft WBT Server, used for Windows Remote Desktop and Remote Assistance connections
            3389,
            // eDonkey 2000 P2P file sharing service. http://www.edonkey2000.com/
            4662,
            // Gnutella (FrostWire, Limewire, Shareaza, etc.), BearShare file sharing app
            6346,
            6347,
            // Port used by p2p software, such as WinMX, Napster.
            6699,
        ]);

        // Block system-reserved ports (requires root for clients to listen on these ports)
        for system_reserved_port in 0..1024 {
            set.insert(system_reserved_port);
        }

        Set(set)
    }
}

impl Deref for Set {
    type Target = IndexSet<u16>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Set {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
