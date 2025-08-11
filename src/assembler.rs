use std::collections::HashMap;

use wg_internal::{network::NodeId, packet::Fragment};


#[derive(Debug)]
pub struct FragmentAssembler {
    pub fragments: HashMap<(u64, NodeId), Vec<Fragment>>, // session_id -> data buffer
    pub expected_fragments: HashMap<(u64, NodeId), u64>, // session_id -> total_fragments
    pub received_fragments: HashMap<(u64, NodeId), Vec<bool>>, // session_id -> received status
}

impl FragmentAssembler {
    pub fn new() -> Self {
        Self {
            fragments: HashMap::new(),
            expected_fragments: HashMap::new(),
            received_fragments: HashMap::new()
        }
    }

    pub fn add_fragment(&mut self, fragment: Fragment, session_id: u64, sender: NodeId) -> Option<Vec<u8>> {
        let communication_id = ( session_id, sender );
        if !self.fragments.contains_key(&communication_id) {
            self.fragments.insert(communication_id, vec![fragment.clone()]);
            self.expected_fragments.insert(communication_id, fragment.total_n_fragments);
            self.received_fragments.insert(communication_id, vec![false; fragment.total_n_fragments as usize]);
        }

        {
            let received = self.received_fragments.get_mut(&communication_id).unwrap();
            received[fragment.fragment_index as usize] = true;
        }

        let expected = self.expected_fragments.get(&communication_id).unwrap();
        let received = self.received_fragments.get(&communication_id).unwrap();
        let fragments = self.fragments.get(&communication_id).unwrap();

        if fragments.len() as u64 == *expected && received.iter().all(|f| *f){
            let fragments = self.fragments.get(&communication_id).unwrap();
            let fragments = FragmentAssembler::sort_by_id(fragments);
            let mut data = vec![];
            for f in fragments.iter() {
                data.copy_from_slice(&f.data);
            }
            let _ = self.fragments.remove(&communication_id);
            let _ = self.received_fragments.remove(&communication_id);
            let _ = self.expected_fragments.remove(&communication_id);
            return Some(data);
        }
        None

    }

    fn sort_by_id(fragments: &Vec<Fragment>) -> Vec<Fragment> {
        let mut result = fragments.clone();
        for f in fragments.iter() {
            result[f.fragment_index as usize] = f.clone();
        }
        result
    }
}
