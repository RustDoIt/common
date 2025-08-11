use std::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use wg_internal::{network::{NodeId, SourceRoutingHeader}, packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet}};
use crate::{network::{Network, NetworkError, Node}, types::NodeEvent};


#[derive(Debug, Clone)]
struct Buffer {
    // represents packets which reached the destination
    packets_received: HashMap<u64, Vec<(bool, Packet)>>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            packets_received: HashMap::new()
        }
    }

    fn insert(&mut self, packet: Packet, session_id: u64) {
        if let Some(v) = self.packets_received.get_mut(&session_id) {
            v.push((false, packet));
        } else {
            let _ = self.packets_received.insert(session_id, vec![(false, packet)]);
        }
    }

    fn get_not_received(&self, session_id: u64) -> Option<Vec<Packet>> {
        let result: Vec<Packet> = self.packets_received
            .get(&session_id)?
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

    fn mark_as_received(&mut self, session_id: u64, fragment_index: u64) -> Option<()> {
        if let Some(f) = self.packets_received.get_mut(&session_id) {
            let ( _received, frag  )= &f[fragment_index as usize];
            f[fragment_index as usize] = (true, frag.clone());
        }

        if self.packets_received.get(&session_id)?.iter().all(|(r, _)| *r) {
            return Some(());
        }
        None
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
        self.controller_send.send(NodeEvent::PacketSent(packet))?;
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
        for (node_id, sender) in self.neighbors.clone().iter_mut() {
            if let Err(_) = self.send(sender, packet.clone()) {
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

    pub fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) -> Result<(), NetworkError> {
        let _ = self.neighbors.insert(node_id, sender);
        self.network_view.update_node(self.id,vec![node_id])?;
        Ok(())
    }

    pub fn handle_flood_response(&mut self, flood_response: FloodResponse) -> Result<(), NetworkError> {
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
                    self.network_view.add_node(new_node)?;
                }
            }
        }
        Ok(())
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

            self.send_packet_to_first_hop(packet)?;

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
                // TODO: send to controller
                neighbor.send(new_flood_request.clone())?;
            }
        }
        Ok(())
    }

    pub fn handle_nack(&mut self, nack: Nack, sender_id: NodeId) -> Result<(), NetworkError> {
        match nack.nack_type {
            NackType::ErrorInRouting(id) | NackType::UnexpectedRecipient(id) => {
                self.network_view.remove_node(id)?;
                self.start_flood()?;
            },

            NackType::DestinationIsDrone => self.network_view.change_node_type(sender_id, NodeType::Drone)?,
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

    pub fn send_message(&mut self, message: &Vec<u8>, destination: NodeId) -> Result<(), NetworkError> {
        let chunks: Vec<&[u8]> = message.chunks(128).collect();
        let total_n_fragments = chunks.len();

        self.session_counter += 1;
        for (i, chunk) in chunks.into_iter().enumerate() {
            // Pad/truncate to exactly 128 bytes
            let mut arr = [0u8; 128];
            arr[..chunk.len()].copy_from_slice(chunk);

            let fragment = Fragment::new(
                i as u64,
                total_n_fragments as u64,
                arr,
            );

            let shr = SourceRoutingHeader::new(
                self.network_view.find_path(destination)?,
                1,
            ).without_loops();

            let packet = Packet::new_fragment(shr, self.session_counter, fragment);

            self.send_packet_to_first_hop(packet.clone())?;
            self.buffer.insert(packet, self.session_counter);
        }

        Ok(())
    }


    pub fn handle_ack(&mut self, ack: Ack, session_id: u64) -> Option<()> {
        self.buffer.mark_as_received(session_id, ack.fragment_index)
    }

    pub fn retry_send(&mut self, session_id: u64) -> Result<(), NetworkError> {
        if let Some(packets) = self.buffer.get_not_received(session_id) {
            for packet in packets {
                self.send_packet_to_first_hop(packet)?;
        }
        }
        Ok(())
    }

    pub fn send_ack(&self, shr: SourceRoutingHeader, session_id: u64, fragment_index: u64) -> Result<(), NetworkError> {
        let packet = Packet::new_ack(shr, session_id, fragment_index);
        self.send_packet_to_first_hop(packet)?;
        Ok(())
    }
}
