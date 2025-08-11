use crossbeam_channel::SendError;
use wg_internal::network::NodeId;
use wg_internal::packet::NodeType;
use std::{collections::{HashMap, HashSet, VecDeque}, fmt::Display};

#[derive(Debug)]
pub enum NetworkError {
    TopologyError,
    PathNotFound(u8),
    NodeNotFound(u8),
    NodeIsNotANeighbor(u8),
    SendError(String),
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TopologyError => write!(f, "Topology error"),
            Self::PathNotFound(id) => write!(f, "Path not found for node {}", id),
            Self::NodeNotFound(id) => write!(f, "Node {} not found", id),
            Self::NodeIsNotANeighbor(id) => write!(f, "Node {} is not a neighbor", id),
            Self::SendError(msg) => write!(f, "Send error: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {}


impl<T: Send + std::fmt::Debug> From<SendError<T>> for NetworkError {
    fn from(value: SendError<T>) -> Self {
        NetworkError::SendError(format!("{:?}", value))
    }
}


#[derive(Clone)]
pub struct Node {
    id: NodeId,
    node_type: NodeType,
    adjacents: Vec<NodeId>
}


impl Node {
    pub fn new(id: NodeId, node_type: NodeType, adjacents: Vec<NodeId>) -> Self {
        Self { id, node_type, adjacents }
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub fn get_node_type(&self) -> NodeType {
        self.node_type.clone()
    }

    pub fn get_adjacents(&self) -> &Vec<NodeId> {
        &self.adjacents
    }

    pub fn add_adjacent(&mut self, adj: NodeId) {
        self.adjacents.push(adj);
    }

    pub fn remove_adjacent(&mut self, adj: NodeId) {
        let index_to_remove = self.adjacents.iter().position(|i| *i == adj).expect(&format!("Node with id {} not found in {} adjacents", adj, self.id));
        let _ = self.adjacents.remove(index_to_remove);
    }
}

impl std::fmt::Debug for Node{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ id: {:?}, adjacents: {:?} ]", self.id, self.adjacents)
    }
}

#[derive(Debug, Clone)]
pub struct Network {
    pub nodes: Vec<Node>
}

impl Network {
    pub fn new(root: Node) -> Self {
        let mut nodes = vec![];
        nodes.push(root);
        Self { nodes }
    }

    pub fn add_node(&mut self, new_node: Node) -> Result<(), NetworkError> {
        for adj in new_node.get_adjacents() {
            if let Some(node) = self.nodes.iter_mut().find(|n| n.get_id() == *adj) {
                match (new_node.get_node_type(), node.get_node_type()) {
                    (_, NodeType::Drone) | (NodeType::Drone, _) => {
                        node.add_adjacent(*adj);
                    }
                    _ => {
                        return Err(NetworkError::TopologyError);
                    }
                }
            }
        }

        self.nodes.push(new_node);
        Ok(())
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), NetworkError> {
        if let Some(_) = self.nodes.iter().find(|n| n.get_id() == node_id) {
            for n in self.nodes.iter_mut() {
                if n.get_adjacents().contains(&node_id){
                    n.remove_adjacent(node_id);
                }
            }
            let index_to_remove = self.nodes.iter().position(|n| n.get_id() == node_id).expect(&format!("Node {} is not a node of the network", node_id));
            let _ = self.nodes.remove(index_to_remove);
            return Ok(());
        } else {
            return Err(NetworkError::NodeNotFound(node_id));
        }
    }

    pub fn update_node(&mut self, node_id: NodeId, adjacents: Vec<NodeId>) -> Result<(), NetworkError> {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.get_id() == node_id) {
            for adj in adjacents {
                if !node.get_adjacents().contains(&adj) {
                    node.add_adjacent(adj);
                }
            }

            // teoretically no need to update neighbors of the node since they should update
            // automatically by the protocol

            return Ok(());
        } else {
            return Err(NetworkError::NodeNotFound(node_id));
        }
    }

    pub fn change_node_type(&mut self, id: NodeId, new_type: NodeType) -> Result<(), NetworkError>{
        if let Some(node) = self.nodes.iter_mut().find(|n| n.get_id() == id) {
            if node.get_node_type() != new_type {
                node.node_type = new_type;
            } else {
                return Err(NetworkError::TopologyError)
            }
        }

        Ok(())
    }

    pub fn find_path(&self, destination: NodeId) -> Result<Vec<NodeId>, NetworkError> {
        let start = self.nodes[0].id;
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent_map = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == destination {
                let mut path = vec![destination];
                let mut current = destination;
                while let Some(&parent) = parent_map.get(&current) {
                    path.push(parent);
                    current = parent;
                }
                path.reverse();
                return Ok(path);
            }

            if let Some(node) = self.nodes.iter().find(|n| n.get_id() == current) {
                for neighbor in node.get_adjacents().iter() {
                    if !visited.contains(neighbor) {
                        visited.insert(*neighbor);
                        parent_map.insert(neighbor, current);
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        Err(NetworkError::PathNotFound(destination))
    }
}
