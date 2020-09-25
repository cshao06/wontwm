// use anyhow::{anyhow, Context};
use crate::{
    xconnection::{XcbConnection, XEvent, XcbKey},
    bindings::KeyManager,
    client::Client,
    ipc,
};

use std::{
    // io::Read,
    process::Command,
    collections::HashMap,
};
use anyhow::{Result, Context};

use xcb::{Window, Atom};
use xcb_util::{ewmh, icccm};

// macro_rules! atoms {
//     ( $( $name:ident ),+ ) => {
//         #[allow(non_snake_case)]
//         pub struct InternedAtoms {
//             $(
//                 pub $name: xcb::Atom
//             ),*
//         }

//         impl InternedAtoms {
//             pub fn new(conn: &XcbConnection) -> Result<InternedAtoms> {
//                 Ok(InternedAtoms {
//                     $(
//                         // $name: XcbConnection::intern_atom(conn, stringify!($name))?
//                         $name: conn.intern_atom(stringify!($name))?
//                     ),*
//                 })
//             }
//         }
//     };
//     // Allow trailing comma:
//     ( $( $name:ident ),+ , ) => (atoms!($( $name ),+);)
// }

// atoms!(WM_DELETE_WINDOW);


// pub struct WindowManager<T: 'a XConn> {
//     conn: &'a dyn XConn,
// }
pub struct WindowManager {
    conn: XcbConnection,
    config: Config,
    // key_bindings: KeyBindings,
    keyman: KeyManager,
    clients: HashMap<Window, Client>,
    focused_client: Option<Window>,
    // atoms: InternedAtoms,
    ipc_atom_command: Atom,
    ipc_atom_state: Atom,
}

impl WindowManager {
    pub fn new() -> Result<Self> {
        let conn = XcbConnection::new()?;
        // let atoms = InternedAtoms::new(&conn).context("Failed to intern atoms")?;
        conn.register_wm()?;
        let ipc_atom_command = conn.intern_atom(ipc::IPC_COMMAND_ATOM)?;
        let ipc_atom_state = conn.intern_atom(ipc::IPC_STATE_ATOM)?;

        let mut keyman = KeyManager::new();
        keyman.set_default_bindings(&conn);
        let mut wm = WindowManager {
            conn,
            config: Config::default(),
            // key_bindings: KeyBindings::new(),
            // keyman: KeyManager::new(),
            keyman,
            clients: HashMap::new(),
            focused_client: None,
            // atoms,
            ipc_atom_command,
            ipc_atom_state,
        };

        // bindings::set_default_bindings(&mut self.key_bindings, &self.conn);

        wm.detect_screens();

        Ok(wm)
    }


    // pub fn run(&mut self, mut bindings: KeyBindings) {
    pub fn run(&mut self) {
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                debug!("got XEvent: {:?}", event);
                match event {
                    // XEvent::ClientCommand { format, window, atom, data} => self.handle_client_message(format, window, atom, data),
                    // XEvent::CreateNotify { id } => self.handle_create_notify(id),
                    XEvent::KeyPress { code } => self.handle_key_press(code),
                    XEvent::MapRequest { id, ignore } => self.handle_map_request(id, ignore),
                    // XEvent::Enter { id, rpt, wpt } => self.handle_enter_notify(id, rpt, wpt),
                    // XEvent::Leave { id, rpt, wpt } => self.handle_leave_notify(id, rpt, wpt),
                    XEvent::Destroy { id } => self.handle_destroy_notify(id),
                    // XEvent::ScreenChange => self.handle_screen_change(),
                    // XEvent::RandrNotify => self.detect_screens(),
                    // XEvent::ConfigureNotify { id, r, is_root } => {
                        // self.handle_configure_notify(id, is_root)
                    // }
                    XEvent::ConfigureRequest { win } => {
                        self.handle_configure_request(win)
                    }
                    XEvent::PropertyNotify { id, atom, is_root } => {
                        self.handle_property_notify(id, atom, is_root)
                    }
                    // XEvent::ClientMessage { id, dtype, data } => {
                    //     self.handle_client_message(id, &dtype, &data)
                    // }
                    _ => (),
                }
                // run_hooks!(event_handled, self,);
            }

            self.conn.flush();
        }
    }

    /// Reset the current known screens based on currently detected outputs
    fn detect_screens(&mut self) {
        // let screens: Vec<Screen> = self
        //     .conn
        //     .current_outputs()
        //     .into_iter()
        //     .enumerate()
        //     .map(|(i, mut s)| {
        //         s.update_effective_region(self.bar_height, self.top_bar);
        //         s.wix = i;
        //         s
        //     })
        //     .collect();

        // info!("updating known screens: {} screens detected", screens.len());
        // for (i, s) in screens.iter().enumerate() {
        //     info!("screen ({}) :: {:?}", i, s);
        // }

        // if screens == self.screens.as_vec() {
        //     return;
        // }

        // self.screens = Ring::new(screens);
        // let visible_workspaces: Vec<_> = self.screens.iter().map(|s| s.wix).collect();
        // visible_workspaces
        //     .iter()
        //     .for_each(|wix| self.apply_layout(*wix));

        // let regions = self.screens.iter().map(|s| s.region(false)).collect();
        // run_hooks!(screens_updated, self, &regions);
    }

    // fn handle_create_notify(&self, win: Window) {
    fn handle_configure_request(&self, win: Window) {
        match self.conn.get_wm_class(win) {
            Ok(c) => {
                debug!("class: {}", c);
                if c == ipc::IPC_WINDOW_CLASS {
                    // TODO: handle error
                    self.conn.register_events(win, xcb::EVENT_MASK_PROPERTY_CHANGE);
                    self.conn.set_text_property(win, self.ipc_atom_state, ipc::IPC_STATE_SERVER_READY);
                    debug!("Ready for property change on the ipc client window");
                }
            },
            Err(e) => debug!("handle_configure_request: Failed to get window class {:?}", e)
        }
    }

    // fn handle_client_message(&mut self, format: u8, window: Window, atom: Atom, data: [u8; 20]) {
    //     let command = data[0];
    //     let args = String::from_utf8(data[1..].to_vec()).unwrap();
    //     // TODO: fix enum from primitive
    //     match command {
    //         0 => { // BindKey
    //             let mut args: Vec<&str> = args.split(' ').collect();
    //             let key_str = args.remove(0);
    //             // let args_str = args.collect().join(' ');
    //             self.keyman.bind_key(&self.conn, key_str, args);
    //         },
    //         _ => return
    //     } 

    // }

    // fn handle_configure_request(&self, event: &xcb::ConfigureRequestEvent) -> Option<Event> {
    //     // This request is not interesting for us: grant it unchanged.
    //     // Build a request with all attributes set, then filter out to only include
    //     // those from the original request.
    //     let values = vec![
    //         (xcb::CONFIG_WINDOW_X as u16, event.x() as u32),
    //         (xcb::CONFIG_WINDOW_Y as u16, event.y() as u32),
    //         (xcb::CONFIG_WINDOW_WIDTH as u16, u32::from(event.width())),
    //         (xcb::CONFIG_WINDOW_HEIGHT as u16, u32::from(event.height())),
    //         (
    //             xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
    //             u32::from(event.border_width()),
    //         ),
    //         (xcb::CONFIG_WINDOW_SIBLING as u16, event.sibling() as u32),
    //         (
    //             xcb::CONFIG_WINDOW_STACK_MODE as u16,
    //             u32::from(event.stack_mode()),
    //         ),
    //     ];
    //     let filtered_values: Vec<_> = values
    //         .into_iter()
    //         .filter(|&(mask, _)| mask & event.value_mask() != 0)
    //         .collect();
    //     xcb::configure_window(&self.connection.conn, event.window(), &filtered_values);

    //     // There's no value in propogating this event.
    //     None
    // }

    /*
     * X Event handler functions
     * These are called in response to incoming XEvents so calling them directly should
     * only be done if the intent is to act as if the corresponding XEvent had been
     * received from the X event loop (i.e. to avoid emitting and picking up the event
     * ourselves)
     */
    fn handle_key_press(&mut self, key: XcbKey) {
        if let Some(action) = self.keyman.get_action(&key) {
        // if let Some(action) = self.key_bindings.get(&key).cloned() {
            debug!("handling key code: {:?}", key);
            // action(self); // ignoring Child handlers and SIGCHILD
            action(self); // ignoring Child handlers and SIGCHILD
            // let mut iter = action.split_whitespace();
            // match iter.next() {
            //     Some(s) => if s == "exec" {spawn("alacritty");},
            //     None => {
            //         debug!("keypress error");
            //         return;
            //     }
            // }
        }
    }

    fn handle_property_notify(&mut self, win: Window, atom: Atom, is_root: bool) {
        if atom == self.ipc_atom_command {
            match self.conn.get_text_property(win, atom) {
                Ok(c) => self.handle_command(c, win),
                Err(e) => debug!("Failed to get changed property {:?}", e),
            }
        }
        // if atom ==  || atom == "_NET_WM_NAME" {
        //     if let Ok(name) = self.conn.str_prop(id, atom) {
        //         self.client_map.get_mut(&id).map(|c| c.set_name(&name));
        //         run_hooks!(client_name_updated, self, id, &name, is_root);
        //     }
        // }
    }

    fn handle_destroy_notify(&mut self, id: Window) {
        debug!("DESTROY_NOTIFY for {}", id);
        self.remove_client(id);
        // self.apply_layout(self.active_ws_index());
    }

    fn handle_map_request(&mut self, id: Window, override_redirect: bool) {
        // if override_redirect || self.client_map.contains_key(&id) {
        if override_redirect {
            return;
        }

        let wm_name = self.conn.get_wm_name(id).unwrap_or(String::new());

        let wm_class = self.conn.get_wm_class(id).unwrap_or(String::new());

        // let floating = self.conn.window_should_float(id, self.floating_classes);
        // let wix = self.active_ws_index();
        let client = Client::new(id, wm_name, wm_class, false);
        // let mut client = Client::new(id, wm_name, wm_class, wix, floating);
        // run_hooks!(new_client, self, &mut client);

        // if client.wm_managed && !floating {
        //     self.add_client_to_workspace(wix, id);
        // }

        self.clients.insert(id, client);
        self.conn.mark_new_window(id);
        // self.conn.focus_client(id);
        self.change_focus(id);

        // self.conn.set_client_workspace(id, wix);
        // self.apply_layout(wix);
        // self.map_window_if_needed(id);
        self.conn.map_window(id);

        // let s = self.screens.focused().unwrap();
        // self.conn.warp_cursor(Some(id), s);
    }

    // fn map_window_if_needed(&mut self, id: Window) {
    //     if let Some(c) = self.client_map.get_mut(&id) {
    //         if !c.mapped {
    //             c.mapped = true;
    //             self.conn.map_window(id);
    //         }
    //     }
    // }

    /// Kill the focused client window.
    pub fn kill_client(&mut self) {
        // let id = self.conn.focused_client();
        if let Some(id) = self.focused_client() {
            self.conn.delete_window(id);
            self.conn.flush();

            self.remove_client(id);
            // self.apply_layout(self.active_ws_index());
        }
    }

    fn remove_client(&mut self, id: Window) {
        match self.clients.get(&id) {
            Some(client) => {
                // self.workspaces
                //     .get_mut(client.workspace())
                //     .and_then(|ws| ws.remove_client(id));
                self.clients.remove(&id).map(|c| {
                    debug!("removing ref to client {} ({})", c.id(), c.wm_class());
                });

                if self.focused_client == Some(id) {
                    self.focused_client = None;
                }
                // run_hooks!(remove_client, self, id);
            }
            None => warn!("attempt to remove unknown client {}", id),
        }
    }

    // fn focused_client(&self) -> Option<&Client> {
    fn focused_client(&self) -> Option<Window> {
        // self.focused_client
            // .or_else(|| {
            //     self.workspaces
            //         .get(self.active_ws_index())
            //         .and_then(|ws| ws.focused_client())
            // })
            // .and_then(|id| self.client_map.get(&id))
        // self.focused_client.and_then(|id| self.clients.get(&id))
        self.focused_client
    }

    // fn client_lost_focus(&self, id: Window) {
    //     let color = self.unfocused_border;
    //     self.conn.set_client_border_color(id, color);
    // }

    fn change_focus(&mut self, id: Window) {
        // let prev_focused = self.focused_client().map(|c| c.id());
        // prev_focused.map(|id| self.client_lost_focus(id));

        // self.conn.set_client_border_color(id, self.focused_border);
        self.conn.focus_client(id);

        // if let Some(wix) = self.workspace_index_for_client(id) {
        //     if let Some(ws) = self.workspaces.get_mut(wix) {
        //         ws.focus_client(id);
        //         let prev_was_in_ws = prev_focused.map_or(false, |id| ws.clients().contains(&id));
        //         if ws.layout_conf().follow_focus && prev_was_in_ws {
        //             self.apply_layout(wix);
        //         }
        //     }
        // }

        self.focused_client = Some(id);
        // run_hooks!(focus_change, self, id);
    }

    fn handle_command(&mut self, command: String, win: Window) {
        let mut command: Vec<&str> = command.split(' ').collect();
        let cmd = command.remove(0);
        match cmd {
            "bindkey" => {
                let key_str = command.remove(0);
                // let args_str = args.collect().join(' ');
                self.keyman.bind_key(&self.conn, key_str, command);
                self.conn.set_text_property(win, self.ipc_atom_state, ipc::IPC_STATE_SUCCESS);
            }
            _ => {
            }
        }
    }
}


/// The main user facing configuration details
pub struct Config {
    /// Default workspace names to use when initialising the WindowManager. Must have at least one element.
    pub workspaces: Vec<String>,
    /// _NET_WM_WINDOW_TYPE_XXX values that should always be treated as floating.
    pub floating_window_types: &'static [&'static str],
    /// Focused boder color
    pub focused_border_color: u32,
    /// Unfocused boder color
    pub unfocused_border_color: u32,
    /// The width of window borders in pixels
    pub border_width_px: u32,
    /// The size of gaps between windows in pixels.
    pub gap_px: u32,
    /// The percentage change in main_ratio to be applied when increasing / decreasing.
    pub main_ratio_step: f32,
    /// Whether or not space should be reserved for a status bar
    pub show_bar: bool,
    /// True if the status bar should be at the top of the screen, false if it should be at the bottom
    pub top_bar: bool,
    /// Height of space reserved for status bars in pixels
    pub bar_height: u32,
}

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

impl Config {
    /// Initialise a default Config, giving sensible (but minimal) values for all fields.
    pub fn default() -> Config {
        Config {
            workspaces: vec_of_strings!["1", "2", "3", "4", "5", "6", "7", "8", "9"],
            floating_window_types: &["DIALOG", "UTILITY", "SPLASH"],
            focused_border_color: 0xcc241d,   // #cc241d
            unfocused_border_color: 0x3c3836, // #3c3836
            border_width_px: 2,
            gap_px: 5,
            main_ratio_step: 0.05,
            show_bar: true,
            top_bar: true,
            bar_height: 18,
        }
    }
}

// hc rule windowtype~'_NET_WM_WINDOW_TYPE_(DIALOG|UTILITY|SPLASH)' floating=on
// hc rule windowtype='_NET_WM_WINDOW_TYPE_DIALOG' focus=on
// hc rule windowtype~'_NET_WM_WINDOW_TYPE_(NOTIFICATION|DOCK|DESKTOP)' manage=off
