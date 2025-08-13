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
    ControllerDisconnected,
    NoDestination,
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TopologyError => write!(f, "Topology error"),
            Self::PathNotFound(id) => write!(f, "Path not found for node {}", id),
            Self::NodeNotFound(id) => write!(f, "Node {} not found", id),
            Self::NodeIsNotANeighbor(id) => write!(f, "Node {} is not a neighbor", id),
            Self::SendError(msg) => write!(f, "Send error: {}", msg),
            Self::ControllerDisconnected => write!(f, "Controller disconnected"),
            Self::NoDestination => write!(f, "Packet has no destination specified"),
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

    pub fn add_node(&mut self, new_node: Node) {
        for adj in new_node.get_adjacents() {
            if let Some(node) = self.nodes.iter_mut().find(|n| n.get_id() == *adj) {
                match (new_node.get_node_type(), node.get_node_type()) {
                    (_, NodeType::Drone) | (NodeType::Drone, _) => {
                        node.add_adjacent(*adj);
                    }
                    _ => {}
                }
            }
        }

        self.nodes.push(new_node);
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        for n in self.nodes.iter_mut() {
            if n.get_adjacents().contains(&node_id){
                n.remove_adjacent(node_id);
            }
        }
        if let Some(index_to_remove) = self.nodes.iter().position(|n| n.get_id() == node_id) {
            let _ = self.nodes.remove(index_to_remove);
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

    pub fn change_node_type(&mut self, id: NodeId, new_type: NodeType) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.get_id() == id) {
            if node.get_node_type() != new_type {
                node.node_type = new_type;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let root = Node::new(1, NodeType::Client, vec![2, 3]);
        let mut network = Network::new(root);

        let new_node = Node::new(2, NodeType::Client, vec![1]);
        network.add_node(new_node);

        assert_eq!(network.nodes.len(), 2);
        assert!(network.nodes.iter().any(|n| n.get_id() == 2));
    }

    #[test]
    fn test_remove_node() {
        let root = Node::new(1, NodeType::Client, vec![2, 3]);
        let mut network = Network::new(root);

        let new_node = Node::new(2, NodeType::Client, vec![1]);
        network.add_node(new_node);

        network.remove_node(2);

        assert_eq!(network.nodes.len(), 1);
        assert!(!network.nodes.iter().any(|n| n.get_id() == 2));
    }

    #[test]
    fn test_update_node() {
        let root = Node::new(1, NodeType::Client, vec![2]);
        let mut network = Network::new(root);

        let new_node = Node::new(2, NodeType::Client, vec![1]);
        network.add_node(new_node);

        network.update_node(1, vec![3]).unwrap();

        assert!(network.nodes[0].get_adjacents().contains(&3));
    }

    #[test]
    fn test_change_node_type() {
        let root = Node::new(1, NodeType::Client, vec![2]);
        let mut network = Network::new(root);

        network.change_node_type(1, NodeType::Drone);

        assert_eq!(network.nodes[0].get_node_type(), NodeType::Drone);
    }

    #[test]
    fn test_find_path() {
        let root = Node::new(1, NodeType::Client, vec![2, 3]);
        let mut network = Network::new(root);

        let node2 = Node::new(2, NodeType::Client, vec![1, 4]);
        let node3 = Node::new(3, NodeType::Client, vec![1]);
        let node4 = Node::new(4, NodeType::Client, vec![2]);

        network.add_node(node2);
        network.add_node(node3);
        network.add_node(node4);

        let path = network.find_path(4).unwrap();

        assert_eq!(path, vec![1, 2, 4]);
    }

    #[test]
    fn test_find_path_not_found() {
        let root = Node::new(1, NodeType::Client, vec![2]);
        let mut network = Network::new(root);

        let node2 = Node::new(2, NodeType::Client, vec![1]);
        network.add_node(node2);

        let result = network.find_path(3);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Path not found for node 3");
    }
}