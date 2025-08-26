use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;

use wg_internal::{network::NodeId, packet::Fragment};


#[derive(Debug, Default)]
pub struct FragmentAssembler {
    pub fragments: HashMap<(u64, NodeId), (u64, Vec<Fragment>)>, // session_id -> data buffer
}

impl FragmentAssembler {
    pub fn add_fragment(&mut self, fragment: Fragment, session_id: u64, sender: NodeId) -> Option<Vec<u8>> {
        let communication_id = ( session_id, sender );
        if let Vacant(entry) = self.fragments.entry(communication_id) {
            
            entry.insert((fragment.total_n_fragments, vec![fragment]));
        }
        
        let fragments = self.fragments.get(&communication_id)?;

        // check if all fragments has been received
        if fragments.0 == fragments.1.len() as u64 {
            let fragments = self.fragments.get_mut(&communication_id)?;
            fragments.1.sort_by(|t, n| t.fragment_index.cmp(&n.fragment_index));
            let mut data = vec![];
            for f in &fragments.1 {
                data.extend_from_slice(&f.data);
            }
            if let Some(pos) = data.iter().position(|&b| b == 0) {
                data.truncate(pos);
            }
            
            let _ = self.fragments.remove(&communication_id);
            return Some(data);
        }
        None
    }
}

