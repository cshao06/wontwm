use xcb::Window;

/**
 * Meta-data around a client window that we are handling.
 *
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone)]
pub struct Client {
    id: Window,
    wm_name: String,
    wm_class: String,
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

impl Client {
    /// Track a new client window on a specific workspace
    pub fn new(
        id: Window,
        wm_name: String,
        wm_class: String,
        // workspace: usize,
        floating: bool,
    ) -> Client {
        Client {
            id,
            wm_name,
            wm_class,
            // workspace,
            floating,
            fullscreen: false,
            mapped: false,
            // wm_managed: true,
        }
    }

    /// The X window ID of this client
    pub fn id(&self) -> Window {
        self.id
    }

    /// The WM_CLASS property of this client
    pub fn wm_class(&self) -> &str {
        &self.wm_class
    }
}

