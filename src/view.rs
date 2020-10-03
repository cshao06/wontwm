use crate::{
    xconnection::Rectangle,
    wm::TagId
};

// pub type VMonId = usize;

pub struct View {
    // monitors: Vec<Monitor>,
    // tags: Vec<TagId>,
    vmons: Vec<VirtualMonitor>,
    tags: Vec<TagId>,
    active_tag: TagId,
}

impl View {
    /// Create a view with the default monitors
    /// Use tags starting from 0 to fill the monitors
    pub fn default(vmons: Vec<VirtualMonitor>) -> View {
        let mut tags = Vec::new();
        for i in 0..vmons.len() {
            tags.push(i);
        }
        debug!("default tags: {:?}", tags);
        View {
            vmons,
            tags,
            active_tag: 0,
        }
    }

    // pub fn new(tags: Vec<TagId>) -> View {
    //     View {
    //         tags,
    //     }
    // }
    pub fn active_tag(&self) -> TagId {
        self.active_tag
    }
    
    pub fn has_tag(&self, tag: TagId) -> bool {
        self.tags.contains(&tag)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct VirtualMonitor {
    region: Rectangle,
}

impl VirtualMonitor {
    pub fn new(region: Rectangle) -> VirtualMonitor {
        VirtualMonitor {
            region,
        }
    }
}
