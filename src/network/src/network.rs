/*
  Copyright (C) 2018-2020 The Purple Core Developers.
  This file is part of the Purple Core Library.

  The Purple Core Library is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  The Purple Core Library is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with the Purple Core Library. If not, see <http://www.gnu.org/licenses/>.
*/

use crate::bootstrap::cache::BootstrapCache;
use crate::connection::*;
use crate::downloader::Downloader;
use crate::error::NetworkErr;
use crate::interface::NetworkInterface;
use crate::packet::Packet;
use crate::packets::connect::Connect;
use crate::peer::ConnectionType;
use crate::priority::NetworkPriority;
use crate::validation::sender::Sender as SenderTrait;
use crate::Peer;
use chain::*;
use crypto::NodeId;
use crypto::SecretKey as Sk;
use dashmap::DashMap;
use hashbrown::HashSet;
use mempool::Mempool;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time;
use triomphe::Arc;

#[cfg(test)]
use crossbeam_channel::Sender;

#[cfg(not(test))]
use flume::Sender;

#[derive(Clone)]
pub struct Network {
    /// Mapping between connected ips and peer information
    pub(crate) peers: Arc<DashMap<SocketAddr, Peer>>,

    /// Our node id
    pub(crate) node_id: NodeId,

    /// Our secret key
    pub(crate) secret_key: Sk,

    /// Reference to the `PowChain`
    pow_chain_ref: PowChainRef,

    /// Sender to `PowChain` block buffer
    pow_chain_sender: Sender<(SocketAddr, Arc<PowBlock>)>,

    /// Reference to the `Downloader`
    downloader: Downloader,

    /// The port we are accepting external TCP connections on.
    port: u16,

    /// The name of the network we are on
    pub(crate) network_name: String,

    /// Maximum number of allowed peers, default is 8
    pub(crate) max_peers: usize,

    /// Bootstrap cache
    pub(crate) bootstrap_cache: BootstrapCache,

    /// Accept connections boolean reference
    pub(crate) accept_connections: Arc<AtomicBool>,

    /// Reference to the mempool
    pub(crate) mempool_ref: Option<Arc<RwLock<Mempool>>>,

    #[cfg(feature = "miner")]
    /// Our retrieved ip address
    pub(crate) our_ip: SocketAddr,
}

impl Network {
    pub fn new(
        node_id: NodeId,
        port: u16,
        network_name: String,
        secret_key: Sk,
        max_peers: usize,
        pow_chain_sender: Sender<(SocketAddr, Arc<PowBlock>)>,
        pow_chain_ref: PowChainRef,
        bootstrap_cache: BootstrapCache,
        mempool_ref: Option<Arc<RwLock<Mempool>>>,
        accept_connections: Arc<AtomicBool>,
        our_ip: Option<SocketAddr>,
    ) -> Network {
        Network {
            peers: Arc::new(DashMap::with_capacity(max_peers)),
            node_id,
            port,
            network_name,
            secret_key,
            max_peers,
            pow_chain_sender,
            pow_chain_ref,
            bootstrap_cache,
            mempool_ref,
            accept_connections,
            downloader: Downloader::new(),

            #[cfg(feature = "miner")]
            our_ip: our_ip.unwrap(),
        }
    }

    pub fn add_peer(&mut self, addr: SocketAddr, peer: Peer) -> Result<(), NetworkErr> {
        if self.peer_count() < self.max_peers {
            self.peers.insert(addr, peer);
            Ok(())
        } else {
            Err(NetworkErr::MaximumPeersReached)
        }
    }

    /// Returns the number of listed peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Returns a reference to the stored secret key.
    pub fn skey(&self) -> &Sk {
        &self.secret_key
    }

    /// Sets the node id of the peer with the given address.
    ///
    /// This function will panic if there is no entry for the given address.
    pub fn set_node_id(&self, addr: &SocketAddr, node_id: NodeId) {
        match self.peers.get_mut(addr) {
            Some(mut peer) => peer.set_id(node_id),
            None => panic!("There is no listed peer with the given address!"),
        };
    }

    /// Removes the peer entry with the given address.
    pub fn remove_peer_with_addr(&self, addr: &SocketAddr) {
        self.peers.remove(addr);
    }

    /// Returns true if the peer with the given address has a `None` id field.
    ///
    /// This function will panic if there is no entry for the given address.
    pub fn is_none_id(&self, addr: &SocketAddr) -> bool {
        match self.peers.get(addr) {
            Some(peer) => peer.id.is_none(),
            None => panic!("There is no listed peer with the given address!"),
        }
    }
}

impl NetworkInterface for Network {
    fn connect(&mut self, address: &SocketAddr) -> Result<(), NetworkErr> {
        info!("Connecting to {}", address);

        connect_to_peer(self.clone(), self.accept_connections.clone(), address);

        Ok(())
    }

    fn connect_to_known(&self, peer: &NodeId) -> Result<(), NetworkErr> {
        unimplemented!();
    }

    fn is_connected_to(&self, address: &SocketAddr) -> bool {
        self.peers.get(address).is_some()
    }

    fn disconnect(&mut self, peer: &NodeId) -> Result<(), NetworkErr> {
        unimplemented!();
    }

    fn disconnect_from_ip(&mut self, ip: &SocketAddr) -> Result<(), NetworkErr> {
        unimplemented!();
    }

    fn has_peer(&self, addr: &SocketAddr) -> bool {
        self.peers.get(addr).is_some()
    }

    fn has_peer_with_id(&self, id: &NodeId) -> bool {
        unimplemented!()
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn send_to_peer<P: Packet>(
        &self,
        peer: &SocketAddr,
        packet: &P,
        priority: NetworkPriority,
    ) -> Result<(), NetworkErr> {
        if let Some(peer) = self.peers.get(peer) {
            if peer.rx.is_some() {
                peer.send_packet(packet, priority)
            } else {
                Err(NetworkErr::CouldNotSend)
            }
        } else {
            Err(NetworkErr::PeerNotFound)
        }
    }

    fn send_to_all<P: Packet>(
        &self,
        packet: &P,
        priority: NetworkPriority,
    ) -> Result<(), NetworkErr> {
        if self.peers.is_empty() {
            return Err(NetworkErr::NoPeers);
        }

        for peer in self.peers.iter() {
            let addr = peer.key();

            if peer.rx.is_some() {
                peer.send_packet(packet, priority)
                    .map_err(|err| warn!("Failed to send packet to {}! Reason: {:?}", addr, err))
                    .unwrap_or(());
            }
        }

        Ok(())
    }

    fn send_to_all_except<P: Packet>(
        &self,
        exception: &SocketAddr,
        packet: &P,
        priority: NetworkPriority,
    ) -> Result<(), NetworkErr> {
        if self.peers.is_empty() {
            return Err(NetworkErr::NoPeers);
        }

        let iter = self.peers.iter().filter(|v| v.key() != exception);

        for peer in self.peers.iter() {
            let addr = peer.key();

            if peer.rx.is_some() {
                peer.send_packet(packet, priority)
                    .map_err(|err| warn!("Failed to send packet to {}! Reason: {:?}", addr, err))
                    .unwrap_or(());
            }
        }

        Ok(())
    }

    fn network_name(&self) -> &str {
        self.network_name.as_str()
    }

    fn pow_chain_ref(&self) -> PowChainRef {
        self.pow_chain_ref.clone()
    }

    fn pow_chain_sender(&self) -> &Sender<(SocketAddr, Arc<PowBlock>)> {
        &self.pow_chain_sender
    }

    fn downloader(&self) -> Downloader {
        self.downloader.clone()
    }

    fn ban_peer(&self, peer: &NodeId) -> Result<(), NetworkErr> {
        unimplemented!();
    }

    fn ban_ip(&self, peer: &SocketAddr) -> Result<(), NetworkErr> {
        unimplemented!();
    }

    fn our_node_id(&self) -> &NodeId {
        &self.node_id
    }

    fn peers(&self) -> Arc<DashMap<SocketAddr, Peer>> {
        self.peers.clone()
    }

    fn secret_key(&self) -> &Sk {
        &self.secret_key
    }

    fn bootstrap_cache(&self) -> BootstrapCache {
        self.bootstrap_cache.clone()
    }

    fn mempool_ref(&self) -> Option<Arc<RwLock<Mempool>>> {
        self.mempool_ref.clone()
    }

    fn after_connect(&self, addr: &SocketAddr) {
        debug!("Executing after connect callback for {}", addr);

        let peers_clone = self.peers.clone();
        let network_clone = self.clone();
        let network_clone2 = self.clone();
        let addr = addr.clone();
        let addr_clone = addr.clone();
        let addr_clone2 = addr.clone();

        // Spawn a repeating task at a given interval for this peer
        let peer_interval = async move {
            let mut peer_interval =
                time::interval(Duration::from_millis(crate::connection::TIMER_INTERVAL));
            let mut times_denied: usize = 0;

            loop {
                if network_clone.has_peer(&addr) {
                    break;
                }

                let peers = peers_clone.clone();
                let addr = addr_clone.clone();
                let peer = peers.get(&addr).unwrap();

                let _ = peer
                    .last_seen
                    .fetch_add(crate::connection::TIMER_INTERVAL, Ordering::SeqCst);
                let last_ping = peer
                    .last_ping
                    .fetch_add(crate::connection::TIMER_INTERVAL, Ordering::SeqCst);

                if last_ping > crate::connection::PING_INTERVAL {
                    let mut sender = peer.validator.ping_pong.sender.lock();

                    if let Ok(ping) = sender.send(()) {
                        peer.last_ping.store(0, Ordering::SeqCst);

                        debug!("Sending Ping packet to {}", addr);

                        network_clone2
                            .send_to_peer(&addr, &ping, NetworkPriority::Low)
                            .map_err(|err| warn!("Could not send ping to {}: {:?}", addr, err))
                            .unwrap_or(());

                        debug!("Sent Ping packet to {}", addr);
                    } else {
                        times_denied += 1;

                        // HACK: Reset sender if it's stuck
                        if times_denied > 10 {
                            times_denied = 0;
                            sender.reset();
                        }
                    }
                }
            }
        };

        tokio::spawn(peer_interval);
    }
}
