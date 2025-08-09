use std::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use wg_internal::{network::{NodeId, SourceRoutingHeader}, packet::{FloodRequest, FloodResponse, NodeType, Packet}};

use crate::network::{Network, NetworkError, Node};

pub struct RoutingHandler {
    id: NodeId,
    network_view: Network,
    neighbors: HashMap<NodeId, Sender<Packet>>,
    flood_seen: HashSet<(NodeId, u64)>,
    session_counter: u64,
    flood_counter: u64
}

impl RoutingHandler {
    pub fn new(id: NodeId, node_type: NodeType, neighbors: HashMap<NodeId, Sender<Packet>>) -> Self {
        Self {
            id,
            network_view: Network::new(Node::new(id, node_type, vec![])),
            neighbors,
            session_counter: 0,
            flood_counter: 0,
            flood_seen: HashSet::new()
        }
    }

    pub fn start_flood(&mut self) -> Result<(), NetworkError> {
        self.session_counter += 1;
        self.flood_counter += 1;
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.session_counter,
            FloodRequest::new(self.flood_counter, self.id )
        );
        for (node_id, sender) in self.neighbors.clone().iter_mut() {
            if let Err(_) = sender.send(packet.clone()) {
                self.remove_neighbor(*node_id)?;
            }
        }
        Ok(())
    }


    pub fn remove_neighbor(&mut self, node_id: NodeId) -> Result<(), NetworkError> {
        let _ = self.neighbors.remove(&node_id);
        self.network_view.remove_node(node_id)?;
        Ok(())
    }

    pub fn handle_flood_response(&mut self, flood_response: FloodResponse) -> Result<(), NetworkError> {
        unimplemented!()
    }
}
