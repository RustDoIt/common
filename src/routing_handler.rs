use std::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use wg_internal::{network::{NodeId, SourceRoutingHeader}, packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet}};
use crate::{network::{Network, NetworkError, Node}, types::NodeEvent};


#[derive(Debug, Clone)]
struct Buffer {
    // represents packets which reached the destination
    packets_received: HashMap<(u64, NodeId), Vec<(bool, Packet)>>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            packets_received: HashMap::new()
        }
    }

    fn insert(&mut self, packet: Packet, session_id: u64, from: NodeId) {
        let id = (session_id, from);
        if let Some(v) = self.packets_received.get_mut(&id) {
            v.push((false, packet));
        } else {
            let _ = self.packets_received.insert(id, vec![(false, packet)]);
        }
    }

    fn get_not_received(&self, session_id: u64, from: NodeId) -> Option<Vec<Packet>> {
        let id = (session_id, from);
        let result: Vec<Packet> = self.packets_received
            .get(&id)?
            .iter()
            .filter_map(|(r, p)| {
                if !r {
                    Some(p.clone())
                } else {
                    None
                }
            })
        .collect();

        Some(result)
    }

    fn mark_as_received(&mut self, session_id: u64, fragment_index: u64, form: NodeId) {
        let id = (session_id, form);
        if let Some(f) = self.packets_received.get_mut(&id) {
            let ( _received, frag  )= &f[fragment_index as usize];
            f[fragment_index as usize] = (true, frag.clone());

            if f.iter().all(|(r, _)| *r) {
                // If all fragments are received, remove the session
                self.packets_received.remove(&id);
            }
        }
    }

    fn get_fragment_by_id(&mut self, session_id: u64, fragment_index: u64, from: NodeId) -> Option<Packet> {
        let id = (session_id, from);
        if let Some(session) = self.packets_received.get(&id) {
            session.iter().nth(fragment_index as usize).map(|(r, p)| if !r { Some(p.clone()) } else { None })?
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoutingHandler {
    id: NodeId,
    network_view: Network,
    neighbors: HashMap<NodeId, Sender<Packet>>,
    flood_seen: HashSet<(u64, NodeId)>,
    session_counter: u64,
    flood_counter: u64,
    controller_send: Sender<NodeEvent>,
    buffer: Buffer
}

impl RoutingHandler {
    pub fn new(id: NodeId, node_type: NodeType, neighbors: HashMap<NodeId, Sender<Packet>>, controller_send: Sender<NodeEvent>) -> Self {
        Self {
            id,
            network_view: Network::new(Node::new(id, node_type, vec![])),
            neighbors,
            session_counter: 0,
            flood_counter: 0,
            flood_seen: HashSet::new(),
            controller_send,
            buffer: Buffer::new()
        }
    }

    fn send(&self, neighbor: &Sender<Packet>, packet: Packet) -> Result<(), NetworkError> {
        neighbor.send(packet.clone())?;
        self.controller_send.send(NodeEvent::PacketSent(packet)).map_err(
            |_e| NetworkError::ControllerDisconnected
        )?;
        Ok(())
    }

    pub fn start_flood(&mut self) -> Result<(), NetworkError> {
        self.session_counter += 1;
        self.flood_counter += 1;
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.session_counter,
            FloodRequest::new(self.flood_counter, self.id )
        );
        self.controller_send.send(NodeEvent::FloodStarted(self.flood_counter, self.id))?;
        for (node_id, sender) in self.neighbors.clone().iter() {
            if let Err(_) = sender.send(packet.clone()) {
                self.remove_neighbor(*node_id);
            }
        }
        Ok(())
    }


    /// Tries to remove the neighbor from the neighbors map and network view
    pub fn remove_neighbor(&mut self, node_id: NodeId) {
        let _ = self.neighbors.remove(&node_id);
        let _ = self.network_view.remove_node(node_id);
    }


    /// Adds a new neighbor to the neighbors map and updates the network view
    pub fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) {
        let _ = self.neighbors.insert(node_id, sender);
        let _ = self.network_view.update_node(self.id,vec![node_id]);
    }

    pub fn handle_flood_response(&mut self, flood_response: FloodResponse) {
        if flood_response.flood_id == self.flood_counter {
            for (i, &(node_id, node_type)) in flood_response.path_trace.iter().enumerate() {
                let mut neighbors = Vec::new();

                // Add previous node as neighbor
                if i > 0 {
                    neighbors.push(flood_response.path_trace[i - 1].0);
                }

                // Add next node as neighbor
                if i + 1 < flood_response.path_trace.len() {
                    neighbors.push(flood_response.path_trace[i + 1].0);
                }

                // Try to update existing node or add new one
                if let Err(_) = self.network_view.update_node(node_id, neighbors.clone()) {
                    let new_node = Node::new(node_id, node_type, neighbors.clone());
                    self.network_view.add_node(new_node);
                }
            }
        }
    }

    pub fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: u64) -> Result<(), NetworkError> {
        let prev_hop = flood_request.path_trace
            .last()
            .map(|x| x.0)
            .unwrap_or(flood_request.initiator_id);

        flood_request.path_trace.push((self.id, NodeType::Drone));

        let flood_session = (flood_request.flood_id, flood_request.initiator_id);

        if !self.flood_seen.insert(flood_session) || self.neighbors.len() == 1 {
            // generate flood response
            let route= if let Ok(path) = self.network_view.find_path(flood_request.initiator_id) {
                SourceRoutingHeader::new(path, 1)
            } else {
                let mut route: Vec<_> = flood_request.path_trace
                    .clone()
                    .iter()
                    .map(|(id, _)| *id)
                    .rev()
                    .collect::<Vec<_>>();


                if route.last() != Some(&flood_request.initiator_id){
                    route.push(flood_request.initiator_id);
                }

                SourceRoutingHeader::new(route, 1)
            };

            let flood_response = FloodResponse {
                flood_id: flood_request.flood_id,
                path_trace: flood_request.path_trace,
            };

            let packet = Packet::new_flood_response(route, session_id, flood_response);

            self.try_send(packet)?;

            return Ok(());
        }

        let srh = SourceRoutingHeader::new(vec![], 0);

        let new_flood_request = Packet::new_flood_request(
            srh,
            session_id,
            flood_request,
        );

        for (neighbor_id, neighbor) in self.neighbors.iter() {
            if *neighbor_id != prev_hop {
                neighbor.send(new_flood_request.clone())?;
            }
        }
        Ok(())
    }

    pub fn handle_nack(&mut self, nack: Nack, session_id: u64, source_id: NodeId) -> Result<(), NetworkError> {
        match nack.nack_type {
            NackType::ErrorInRouting(id) => {
                self.remove_neighbor(id);
                self.start_flood()?;
                if let Some(packet) = self.buffer.get_fragment_by_id(session_id, nack.fragment_index, source_id) {
                    self.try_send(packet)?;
                }

            },
            NackType::Dropped => {
                self.network_view.remove_node(source_id);
                self.start_flood()?;
            },
            NackType::DestinationIsDrone => self.network_view.change_node_type(source_id, NodeType::Drone),
            _ => {}
        }

        Ok(())
    }

    /// Send a packet to the first hop in its route
    fn send_packet_to_first_hop(&self, packet: Packet) -> Result<(), NetworkError> {
        if packet.routing_header.hops.len() > 1 {
            let first_hop = packet.routing_header.hops[1];
            if let Some(sender) = self.neighbors.get(&first_hop) {
                self.send(sender, packet)?;
            } else {
                return Err(NetworkError::NodeIsNotANeighbor(first_hop));
            }
        }
        Ok(())
    }


    /// Tries to send a packet to next hop until it succeeds or there are no more neighbors.
    /// If sending fails, it removes the neighbor, finds a new route and tries again.
    fn try_send(&mut self, mut packet: Packet) -> Result<(), NetworkError> {
        // A packet must have a destination
        let destination = packet
            .routing_header
            .destination()
            .ok_or(NetworkError::NoDestination)?;

        let mut packet_sent = false;
        while !packet_sent && self.neighbors.len() > 0 {
            match self.send_packet_to_first_hop(packet.clone()) {
                Ok(_) => {
                    packet_sent = true;
                },
                Err(NetworkError::SendError(t)) => {
                    // If the first hop is not a neighbor, remove it and try again
                    if let Some(first_hop) = packet.routing_header.hops.get(1) {
                        self.remove_neighbor(*first_hop);
                        let route = self.network_view
                            .find_path(destination)
                            .ok_or(NetworkError::PathNotFound(destination))?;
                        packet.routing_header = SourceRoutingHeader::new(route, 1).without_loops();
                    }
                },
                Err(e) => return Err(e),
            }
        }

        if self.neighbors.is_empty() {
            return Err(NetworkError::NoNeighborAssigned);
        }

        Ok(())

    }


    /// Sends a message by fragmenting it into 128-byte chunks and sending each chunk as a separate packet.
    pub fn send_message(&mut self, message: &Vec<u8>, destination: NodeId) -> Result<(), NetworkError> {
        let chunks: Vec<&[u8]> = message.chunks(128).collect();
        let total_n_fragments = chunks.len();

        self.session_counter += 1;
        let shr = SourceRoutingHeader::new(
            self.network_view.find_path(destination).ok_or(NetworkError::PathNotFound(destination))?,
            1,
        ).without_loops();

        for (i, chunk) in chunks.into_iter().enumerate() {
            // Pad/truncate to exactly 128 bytes
            let mut arr = [0u8; 128];
            arr[..chunk.len()].copy_from_slice(chunk);

            let fragment = Fragment::new(
                i as u64,
                total_n_fragments as u64,
                arr,
            );

            let packet = Packet::new_fragment(shr.clone(), self.session_counter, fragment);

            self.try_send(packet.clone())?;
            self.buffer.insert(packet, self.session_counter, self.id);
        }

        Ok(())
    }


    pub fn handle_ack(&mut self, ack: Ack, session_id: u64, from: NodeId) {
        self.buffer.mark_as_received(session_id, ack.fragment_index, from);
    }

    pub fn retry_send_all(&mut self, session_id: u64, from: NodeId) -> Result<(), NetworkError> {
        if let Some(packets) = self.buffer.get_not_received(session_id, from) {
            for packet in packets {
                self.try_send(packet)?;
            }
        }
        Ok(())
    }

    pub fn retry_send(&mut self, session_id: u64, fragment_index: u64, from: NodeId) -> Result<(), NetworkError> {
        if let Some(packet) = self.buffer.get_fragment_by_id(session_id, fragment_index, from) {
            self.try_send(packet)?;
        }
        Ok(())
    }

    pub fn send_ack(&mut self, shr: SourceRoutingHeader, session_id: u64, fragment_index: u64) -> Result<(), NetworkError> {
        let packet = Packet::new_ack(shr, session_id, fragment_index);
        self.try_send(packet)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use wg_internal::packet::PacketType;

    #[test]
    fn test_add_neighbor() {
        let (sender, _receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);

        let (neighbor_sender, _neighbor_receiver) = unbounded();
        handler.add_neighbor(2, neighbor_sender);

        assert!(handler.neighbors.contains_key(&2));
        assert!(handler.network_view.nodes[0].get_adjacents().contains(&2));
    }

    #[test]
    fn test_remove_neighbor() {
        let (sender, _receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);

        let (neighbor_sender, _neighbor_receiver) = unbounded();
        handler.add_neighbor(2, neighbor_sender);
        handler.remove_neighbor(2);

        assert!(!handler.neighbors.contains_key(&2));
        assert!(!handler.network_view.nodes[0].get_adjacents().contains(&2));
    }

    #[test]
    fn test_start_flood() {
        let (sender, receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);

        let (neighbor_sender, neighbor_receiver) = unbounded();
        handler.add_neighbor(2, neighbor_sender);

        handler.start_flood().unwrap();

        let packet = receiver.try_recv().unwrap();
        assert!(matches!(packet, NodeEvent::FloodStarted(_, _)));

        let neighbor_packet = neighbor_receiver.try_recv().unwrap();
        assert!(matches!(neighbor_packet.pack_type, PacketType::FloodRequest(_)));
    }

    #[test]
    fn test_handle_flood_response() {
        let (sender, _receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);
        handler.flood_counter = 1;

        let flood_response = FloodResponse {
            flood_id: 1,
            path_trace: vec![(2, NodeType::Drone), (3, NodeType::Client)],
        };

        handler.handle_flood_response(flood_response);

        assert!(handler.network_view.nodes.iter().any(|n| n.get_id() == 2));
        assert!(handler.network_view.nodes.iter().any(|n| n.get_id() == 3));
    }

    #[test]
    fn test_send_message() {
        let (sender, _receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);

        let (neighbor_sender, neighbor_receiver) = unbounded();
        handler.add_neighbor(2, neighbor_sender);

        let message= b"Hello world".to_vec(); // 128 bytes total

        handler.send_message(&message, 2).unwrap();

        let packet = neighbor_receiver.try_recv().unwrap();
        assert!(matches!(packet.pack_type, PacketType::MsgFragment(_)));
    }

    #[test]
    fn test_handle_ack() {
        let (sender, _receiver) = unbounded();
        let mut handler = RoutingHandler::new(1, NodeType::Client, HashMap::new(), sender);

        let (neighbor_sender, _neighbor_receiver) = unbounded();
        handler.add_neighbor(2, neighbor_sender);

        let message = b"Hello, world!".to_vec();
        handler.send_message(&message, 2).unwrap();

        let ack = Ack {
            fragment_index: 0,
        };

        handler.handle_ack(ack, 1, 2);

        assert!(handler.buffer.get_not_received(1, 2).is_none());
    }
}
