use std::{collections::HashMap, str::FromStr};
use std::fmt::Display;


use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use wg_internal::{network::NodeId, packet::Packet};
use crossbeam_channel::Sender;
use uuid::Uuid;


pub type Bytes = Vec<u8>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaReference {
    location: NodeId,
    pub id: Uuid
}

impl MediaReference {
    #[must_use]
    pub fn new(location: NodeId) -> Self {
        Self {
            location,
            id: Uuid::new_v4()
        }
    }

    pub fn get_location(&self) -> NodeId {
        self.location
    }
}

impl Display for MediaReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.location, self.id.to_string())
    }
}

impl FromStr for MediaReference {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (location, id) = value.split_at({
            if let Some(c) = value.chars().position(|c| c == '/') {
                c
            } else {
                return Err(anyhow!("Cannot parse media reference"))
            }
        });
        Ok(Self { location: u8::from_str(location)?, id: Uuid::from_str(id)? })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFile<'a>{
    id: Uuid,
    title: String,
    content: &'a str,
    media_refs: Option<Vec<MediaReference>>
}

impl<'a> TextFile<'a> {
    pub fn new(title: String, content: &'a str, media_refs: Option<Vec<MediaReference>>) -> Self {
        Self {
            title,
            id: Uuid::new_v4(),
            content,
            media_refs
        }
    }

    pub fn get_refs(&self) -> Option<Vec<MediaReference>> {
        self.media_refs.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    id: Uuid,
    title: String,
    content: Vec<Bytes>,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "request_type")]
pub enum WebRequest {
    #[serde(rename = "server_type?")]
    ServerTypeQuery,

    #[serde(rename = "files_list?")]
    TextFilesListQuery,

    #[serde(rename = "file?")]
    FileQuery { file_id: String },

    #[serde(rename = "media?")]
    MediaQuery { media_id: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "response_type")]
pub enum WebResponse {
    #[serde(rename = "server_type!")]
    ServerTypeResponse { server_type: String },

    #[serde(rename = "files_list!")]
    TextFilesListResponse { files: Vec<String> },

    #[serde(rename = "file!")]
    FileResponse { file_size: usize, file_data: Vec<u8> },

    #[serde(rename = "media!")]
    MediaResponse { media_data: Vec<u8> },

    #[serde(rename = "error_requested_not_found!")]
    ErrorNotFound,

    #[serde(rename = "error_unsupported_request!")]
    ErrorUnsupportedRequest,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "request_type")]
pub enum ChatRequest {
    #[serde(rename = "server_type?")]
    ServerTypeQuery,

    #[serde(rename = "registration_to_chat")]
    RegistrationToChat { client_id: NodeId },

    #[serde(rename = "client_list?")]
    ClientListQuery,

    #[serde(rename = "message_for?")]
    MessageFor { client_id: NodeId, message: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "response_type")]
pub enum ChatResponse {
    #[serde(rename = "server_type!")]
    ServerType { server_id: NodeId, server_type: ServerType },

    #[serde(rename = "client_list!")]
    ClientList { list_of_client_ids: Vec<NodeId> },

    #[serde(rename = "message_from!")]
    MessageFrom { client_id: NodeId, message: String },

    #[serde(rename = "error_wrong_client_id!")]
    ErrorWrongClientId,

    // Custom response for successful registration
    #[serde(rename = "registration_success")]
    RegistrationSuccess,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub from: NodeId,
    pub to: NodeId,
    pub text: String,
}

impl Message {
    #[must_use]
    pub fn new(from: NodeId, to: NodeId, text: String) -> Self {
        Message { from, to, text }
    }
}

#[derive(Debug, Clone)]
pub enum ChatCommand {
    GetChatsHistory,
    GetRegisteredClients,
    SendMessage(Message)
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    ChatHistory(HashMap<NodeId, Vec<Message>>),
    RegisteredClients(Vec<NodeId>),
    MessageSent,
    MessageReceived(Message)
}


#[derive(Debug, Clone)]
pub enum NodeEvent {
    PacketSent(Packet),
    FloodStarted(u64, NodeId),
    NodeRemoved(NodeId)
}


#[derive(Debug, Clone)]
pub enum NodeCommand {
    AddSender(NodeId, Sender<Packet>),
    RemoveSender(NodeId),
    Shutdown,
}

impl NodeCommand {
    #[must_use]
    pub fn as_add_sender(self) -> Option<(NodeId, Sender<Packet>)> {
        match self {
            NodeCommand::AddSender(id, sender) => Some((id, sender)),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_add_sender(&self) -> bool {
        matches!(self, Self::AddSender(_, _))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientType {
    ChatClient,
    WebBrowser,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ServerType {
    ChatServer,
    MediaServer,
}
