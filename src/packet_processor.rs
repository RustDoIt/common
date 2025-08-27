use std::sync::{Arc, Barrier};

use crate::{FragmentAssembler, RoutingHandler, network::NetworkError, types::Command};

use crossbeam_channel::{Receiver, select_biased};
use wg_internal::{
    network::NodeId,
    packet::{Packet, PacketType},
};

pub trait Processor: Send {
    fn controller_recv(&self) -> &Receiver<Box<dyn Command>>;
    fn packet_recv(&self) -> &Receiver<Packet>;
    fn assembler(&mut self) -> &mut FragmentAssembler;
    fn routing_handler(&mut self) -> &mut RoutingHandler;

    fn handle_msg(&mut self, msg: Vec<u8>, from: NodeId, session_id: u64);
    fn handle_command(&mut self, cmd: Box<dyn Command>) -> bool;

    /// Handles a packet in a standard way
    /// # Errors
    /// returns an Errors if handling fails
    fn handle_packet(&mut self, pkt: Packet) -> Result<(), NetworkError> {
        let router = self.routing_handler();
        match pkt.pack_type {
            PacketType::MsgFragment(fragment) => {
                let idx = fragment.fragment_index;
                let mut shr = pkt.routing_header.clone();
                shr.reverse();
                shr.increase_hop_index();
                assert!(
                    shr.hop_index == 1,
                    "hop_index should be 1, got {}",
                    shr.hop_index
                );
                self.routing_handler().send_ack(shr, pkt.session_id, idx)?;
                if let Some(msg) = self.assembler().add_fragment(
                    fragment,
                    pkt.session_id,
                    pkt.routing_header.hops[0],
                ) {
                    self.handle_msg(msg, pkt.routing_header.hops[0], pkt.session_id);
                }
            }
            PacketType::Ack(ack) => {
                router.handle_ack(&ack, pkt.session_id, pkt.routing_header.hops[0]);
            }
            PacketType::Nack(nack) => {
                router.handle_nack(&nack, pkt.session_id, pkt.routing_header.hops[0])?;
            }
            PacketType::FloodRequest(flood_request) => {
                router.handle_flood_request(flood_request, pkt.session_id)?;
            }
            PacketType::FloodResponse(flood_response) => {
                let _ = router.handle_flood_response(&flood_response);
            }
        }
        Ok(())
    }

    fn run(&mut self, barrier: Arc<Barrier>) {
        barrier.wait();
        let _ = self.routing_handler().start_flood(None);
        loop {
            select_biased! {
                recv(self.controller_recv()) -> cmd => {
                    if let Ok(cmd) = cmd {
                        if self.handle_command(cmd) {
                            // Terminate if handle_command returns true
                            println!("Terminating");
                            return;
                        }
                    }
                }

                recv(self.packet_recv()) -> pkt => {
                    if let Ok(pkt) = pkt {
                        if self.handle_packet(pkt).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
}
