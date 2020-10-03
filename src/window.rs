use xcb::Window;
use crate::wm::TagId;

/**
 * Meta-data around a window that we are handling.
 *
 * Primarily state flags and information used when determining which windows
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone)]
pub struct WindowInfo {
    id: Window,
    wm_name: String,
    wm_class: String,
    tag: TagId,
    // workspace: usize,
    // state flags
    floating: bool,
    // pub(crate) fullscreen: bool,
    // pub(crate) mapped: bool,
    // pub(crate) wm_managed: bool,
    fullscreen: bool,
    mapped: bool,
    // wm_managed: bool,
}

impl WindowInfo {
    /// Track a new window on a specific workspace
    pub fn new(
        id: Window,
        wm_name: String,
        wm_class: String,
        tag: TagId,
        // workspace: usize,
        floating: bool,
    ) -> WindowInfo {
        WindowInfo {
            id,
            wm_name,
            wm_class,
            tag,
            // workspace,
            floating,
            fullscreen: false,
            mapped: false,
            // wm_managed: true,
        }
    }

    /// The X window ID of this window
    pub fn id(&self) -> Window {
        self.id
    }

    /// The WM_CLASS property of this window
    pub fn wm_class(&self) -> &str {
        &self.wm_class
    }

    pub fn tag(&self) -> TagId {
        self.tag
    }
}

