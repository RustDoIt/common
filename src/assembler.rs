use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex, RwLock}};

use crossbeam_channel::Sender;
use wg_internal::{network::NodeId, packet::Packet};

use crate::{network::Network, types::SendingMap};



pub struct Assembler {
    node_id: NodeId,
    neighbors: SendingMap,
    network_view: Arc<RwLock<Network>>,
    session_counter: u64,
    flood_counter: u64,
    seen_floods: HashSet<(u64, NodeId)>,
}

impl Assembler {
    pub fn new(node_id: NodeId, neighbors: SendingMap, network_view: Arc<RwLock<Network>>)-> Self {
        Self {
            node_id,
            neighbors,
            network_view,
            flood_counter: 0,
            session_counter: 0,
            seen_floods: HashSet::new()
        }
    }
}
