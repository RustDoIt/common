use tokio::sync::{oneshot, Mutex, RwLock};
use wg_internal::{network::NodeId, packet::{Packet, PacketType}};
use std::{collections::HashMap, sync::Arc};
use crossbeam_channel::Sender;

pub type PendingQueue = Arc<Mutex<HashMap<u64, oneshot::Sender<PacketType>>>>;
pub type SendingMap = Arc<RwLock<HashMap<NodeId, Sender<Packet>>>>;

pub enum NodeEvent {
    PacketSent(Packet),
    FloodStarted(u64),
    NodeRemoved(NodeId)
}

#[derive(Debug)]
pub enum NodeCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
    Shutdown,
}

impl NodeCommand {
    pub fn as_add_sender(self) -> Option<(NodeId, Sender<Packet>)> {
        match self {
            NodeCommand::AddSender(id, sender) => Some((id, sender)),
            _ => None,
        }
    }

    pub fn is_add_sender(&self) -> bool {
        matches!(self, Self::AddSender(_, _))
    }
}

