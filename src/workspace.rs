use crate::{
    view::View,
    wm::TagId,
};
use xcb::Window;

pub struct Workspace {
    name: String,
    // main_view: View,
    views: Vec<View>,
    active_view: usize,
}

impl Workspace {
    pub fn new(name: impl Into<String>, default_view: View) -> Workspace {
        Workspace {
            name: name.into(),
            // main_view: View::new(vec!(main_tag)),
            // Create a view with tag 0
            views: vec!(default_view),
            active_view: 0,
        }
    }

    pub fn active_tag(&self) -> TagId {
        self.active_view().active_tag()
    }
    
    pub fn active_view(&self) -> &View {
        &self.views[self.active_view]
    }
}
