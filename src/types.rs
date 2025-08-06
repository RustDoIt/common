use tokio::sync::{oneshot, Mutex, RwLock};
use wg_internal::{network::NodeId, packet::{Packet, PacketType}};
use std::{collections::HashMap, sync::Arc};
use crossbeam_channel::Sender;

pub type PendingQueue = Arc<Mutex<HashMap<u64, oneshot::Sender<PacketType>>>>;
pub type SendingMap = Arc<RwLock<HashMap<NodeId, Sender<Packet>>>>;

pub enum NodeEvent {
    PacketSent(Packet),
    FloodStarted(u64)
}

pub enum NodeCommand {
    AddSender(NodeId, Sender<Packet>),
    StartFlood,
}
