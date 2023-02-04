use std::ops::Deref;

use dashmap::DashSet;

pub struct Set(DashSet<u16>);

impl Default for Set {
    fn default() -> Set {
        let ports = Set(DashSet::with_capacity(16));
        // SSH Port
        ports.insert(22);
        // DNS queries
        ports.insert(53);
        // Hyper Text Transfer Protocol (HTTP) - port used for web traffic
        ports.insert(80);
        ports.insert(81);
        ports.insert(8080);
        ports.insert(8081);
        // 	Direct Connect Hub (unofficial)
        ports.insert(411);
        ports.insert(412);
        ports.insert(413);
        // HTTPS / SSL - encrypted web traffic, also used for VPN tunnels over HTTPS.
        ports.insert(443);
        // Kazaa - peer-to-peer file sharing, some known vulnerabilities, and at least one worm (Benjamin) targeting it.
        ports.insert(1214);
        // IANA registered for Microsoft WBT Server, used for Windows Remote Desktop and Remote Assistance connections
        ports.insert(3389);
        // eDonkey 2000 P2P file sharing service. http://www.edonkey2000.com/
        ports.insert(4662);
        // Gnutella (FrostWire, Limewire, Shareaza, etc.), BearShare file sharing app
        ports.insert(6346);
        ports.insert(6347);
        // Port used by p2p software, such as WinMX, Napster.
        ports.insert(6699);

        ports
    }
}

impl Deref for Set {
    type Target = DashSet<u16>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
