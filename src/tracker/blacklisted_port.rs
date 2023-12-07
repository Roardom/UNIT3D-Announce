use std::ops::{Deref, DerefMut};

use ahash::RandomState;
use scc::HashIndex;

pub struct Set(HashIndex<u16, (), RandomState>);

impl Default for Set {
    #[rustfmt::skip]
    fn default() -> Set {
        let blacklisted_ports = HashIndex::with_hasher(RandomState::new());

        // SSH Port
        let _ = blacklisted_ports.insert(22, ());

        // DNS queries
        let _ = blacklisted_ports.insert(53, ());

        // Hyper Text Transfer Protocol (HTTP) - port used for web traffic
        let _ = blacklisted_ports.insert(80, ());
        let _ = blacklisted_ports.insert(81, ());
        let _ = blacklisted_ports.insert(8080, ());
        let _ = blacklisted_ports.insert(8081, ());

        // 	Direct Connect Hub (unofficial)
        let _ = blacklisted_ports.insert(411, ());
        let _ = blacklisted_ports.insert(412, ());
        let _ = blacklisted_ports.insert(413, ());

        // HTTPS / SSL - encrypted web traffic, also used for VPN tunnels over HTTPS.
        let _ = blacklisted_ports.insert(443, ());

        // Kazaa - peer-to-peer file sharing, some known vulnerabilities, and at least one worm (Benjamin) targeting it.
        let _ = blacklisted_ports.insert(1214, ());

        // IANA registered for Microsoft WBT Server, used for Windows Remote Desktop and Remote Assistance connections
        let _ = blacklisted_ports.insert(3389, ());

        // eDonkey 2000 P2P file sharing service. http://www.edonkey2000.com/
        let _ = blacklisted_ports.insert(4662, ());

        // Gnutella (FrostWire, Limewire, Shareaza, etc.), BearShare file sharing app
        let _ = blacklisted_ports.insert(6346, ());
        let _ = blacklisted_ports.insert(6347, ());

        // Port used by p2p software, such as WinMX, Napster.
        let _ = blacklisted_ports.insert(6699, ());

        Set(blacklisted_ports)
    }
}

impl Deref for Set {
    type Target = HashIndex<u16, (), RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Set {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
