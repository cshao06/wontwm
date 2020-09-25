// use std::{cell::Cell, collections::HashMap, convert::TryFrom, convert::TryInto};
use anyhow::{Result, Context, anyhow};

use xcb::{Window, Atom};
use xcb_util::{ewmh, icccm};

// Mask out the most significant bit, which indicates if it's a send_event
// Bit mask to find event type regardless of event source.
// Each event in the X11 protocol contains an 8-bit type code. The most-significant bit in this code is set if the event was generated from a SendEvent request. This mask can be used to determine the type of event regardless of how the event was generated. See the X11R6 protocol specification for details.
const XCB_RESPONSE_TYPE_MASK: u8 = 0x7F;
const GRAB_MODE_ASYNC: u8 = xcb::GRAB_MODE_ASYNC as u8;
const ROOT_EVENT_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_PROPERTY_CHANGE
        | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
        | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY,
)];
const NEW_WINDOW_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW | xcb::EVENT_MASK_PROPERTY_CHANGE,
)];
const INPUT_FOCUS_PARENT: u8 = xcb::INPUT_FOCUS_PARENT as u8;
const PROP_MODE_REPLACE: u8 = xcb::PROP_MODE_REPLACE as u8;

const CONFIG_WINDOW_BORDER_WIDTH: u16 = xcb::CONFIG_WINDOW_BORDER_WIDTH as u16;
const CONFIG_WINDOW_HEIGHT: u16 = xcb::CONFIG_WINDOW_HEIGHT as u16;
const CONFIG_WINDOW_WIDTH: u16 = xcb::CONFIG_WINDOW_WIDTH as u16;
const CONFIG_WINDOW_X: u16 = xcb::CONFIG_WINDOW_X as u16;
const CONFIG_WINDOW_Y: u16 = xcb::CONFIG_WINDOW_Y as u16;

macro_rules! atoms {
    ( $( $name:ident ),+ ) => {
        #[allow(non_snake_case)]
        pub struct InternedAtoms {
            $(
                pub $name: xcb::Atom
            ),*
        }

        impl InternedAtoms {
            pub fn new(conn: &xcb::Connection) -> Result<InternedAtoms> {
                Ok(InternedAtoms {
                    $(
                        $name: xcb::intern_atom(conn, false, stringify!($name)).get_reply()?.atom()
                    ),*
                })
            }
        }
    };
    // Allow trailing comma:
    ( $( $name:ident ),+ , ) => (atoms!($( $name ),+);)
}

// Intern atoms that are not built-in in icccm or ewmh
atoms!(WM_DELETE_WINDOW, UTF8_STRING);

/// An X key-code along with a modifier mask
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct XcbKey {
    /// Modifier key bit mask
    pub mod_mask: u16,
    /// X key code
    pub code: xcb::Keycode,
}

impl XcbKey {
    /// Build a new XcbKey from an XCB KeyPressEvent
    pub fn from_key_press(k: &xcb::KeyPressEvent) -> XcbKey {
        XcbKey {
            mod_mask: k.state(),
            code: k.detail(),
        }
    }
}

/// An x,y coordinate pair
#[derive(Debug, Copy, Clone)]
pub struct Point {
    /// An absolute x coordinate relative to the root window
    pub x: u32,
    /// An absolute y coordinate relative to the root window
    pub y: u32,
}

impl Point {
    /// Create a new Point.
    pub fn new(x: u32, y: u32) -> Point {
        Point { x, y }
    }
}

/**
 * Wrapper around the low level XCB event types that require casting to work with.
 * Not all event fields are extracted so check the XCB documentation and update
 * accordingly if you need access to something that isn't currently passed through
 * to the WindowManager event loop.
 *
 * https://tronche.com/gui/x/xlib/events/types.html
 * https://github.com/rtbo/rust-xcb/xml/xproto.xml
 *
 * ### XCB Level events
 *
 * *MapNotify* - a window was mapped
 *   - _event_ (Window):
 *     The window which was mapped or its parent, depending on
 *     whether `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (Window):
 *     The window that was mapped.
 *   - _override_redirect_ (bool):
 *     We should ignore this window if true
 *
 * *UnmapNotify* - a window was unmapped
 *   - _event_ (Window):
 *     The window which was unmapped or its parent, depending on
 *     whether `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (Window):
 *     The window that was unmapped.
 *   - _from-configure_ (bool):
 *     - 'true' if the event was generated as a result of a resizing of
 *       the window's parent when `window` had a win_gravity of `UnmapGravity`.
 *
 * *EnterNotify* - the pointer is now in a different window
 *   - _event_ (Window):
 *     The window on which the event was generated.
 *   - _child_ (Window):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (Window):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *   - _mode_ (NotifyMode enum)
 *     - Normal, Grab, Ungrab, WhileGrabbed
 *
 * *LeaveNotify* - the pointer has left a window
 *   - Same fields as *EnterNotify*
 *
 * *DestroyNotify* - a window has been destroyed
 *   - _event_ (Window):
 *     The reconfigured window or its parent, depending on whether
 *     `StructureNotify` or `SubstructureNotify` was selected.
 *   - _window_ (Window):
 *     The window that was destroyed.
 *
 * *KeyPress* - a keyboard key was pressed / released
 *   - _detail_ (u8):
 *     Keycode of the key that was pressed
 *   - _event_ (u16):
 *     The modifier masks being held when the key was pressed
 *   - _child_ (Window):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (Window):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *
 * *ButtonPress* - a mouse button was pressed
 *   - _detail_ (u8):
 *     The button that was pressed
 *   - _event_ (u16):
 *     The modifier masks being held when the button was pressed
 *   - _child_ (Window):
 *     If the window has sub-windows then this is the ID of the window
 *     that the pointer ended on, XCB_WINDOW_NONE otherwise.
 *   - _root_ (Window):
 *     The root window for the final cursor position.
 *   - _root-x, root-y_ (i16, i16):
 *     The coordinates of the pointer relative to 'root's origin.
 *   - _event-x, event-y_ (i16, i16):
 *     The coordinates of the pointer relative to the event window's origin.
 *
 * *ButtonRelease* - a mouse button was released
 *   - same fields as *ButtonPress*
 */
#[derive(Debug, Clone)]
pub enum XEvent {
    // CreateNotify {
    //     id: Window,
    // },

    ClientCommand {
        format: u8,
        window: Window,
        atom: Atom,
        data: [u8; 20],
    },

    // /// xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // ButtonPress,

    // /// xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // ButtonRelease,

    /// xcb docs: https://www.mankier.com/3/xcb_input_device_key_press_event_t
    KeyPress {
        /// The X11 key code that was received along with any modifiers that were held
        code: XcbKey,
    },

    /// xcb docs: https://www.mankier.com/3/xcb_map_request_event_t
    MapRequest {
        /// The ID of the window that wants to be mapped
        id: Window,
        /// Whether or not the WindowManager should handle this window.
        ignore: bool,
    },

    // /// xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    // Enter {
    //     /// The ID of the window that was entered
    //     id: Window,
    //     /// Absolute coordinate of the event
    //     rpt: Point,
    //     /// Coordinate of the event relative to top-left of the window itself
    //     wpt: Point,
    // },

    // /// xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    // Leave {
    //     /// The ID of the window that was left
    //     id: Window,
    //     /// Absolute coordinate of the event
    //     rpt: Point,
    //     /// Coordinate of the event relative to top-left of the window itself
    //     wpt: Point,
    // },

    // /// xcb docs: https://www.mankier.com/3/xcb_focus_in_event_t
    // FocusIn {
    //     /// The ID of the window that gained focus
    //     id: Window,
    // },

    // /// xcb docs: https://www.mankier.com/3/xcb_focus_out_event_t
    // FocusOut {
    //     /// The ID of the window that lost focus
    //     id: Window,
    // },

    /// MapNotifyEvent
    /// xcb docs: https://www.mankier.com/3/xcb_destroy_notify_event_t
    Destroy {
        /// The ID of the window being destroyed
        id: Window,
    },

    // /// xcb docs: https://www.mankier.com/3/xcb_randr_screen_change_notify_event_t
    // ScreenChange,

    // /// xcb docs: https://www.mankier.com/3/xcb_randr_notify_event_t
    // RandrNotify,

    // /// xcb docs: https://www.mankier.com/3/xcb_configure_notify_event_t
    // ConfigureNotify {
    //     /// The ID of the window that had a property changed
    //     id: Window,
    //     /// The new window size
    //     r: Region,
    //     /// Is this window the root window?
    //     is_root: bool,
    // },

    ConfigureRequest {
        win: Window
    },

    /// xcb docs: https://www.mankier.com/3/xcb_property_notify_event_t
    PropertyNotify {
        /// The ID of the window that had a property changed
        id: Window,
        /// The property that changed
        // atom: String,
        atom: Atom,
        /// Is this window the root window?
        is_root: bool,
    },

    // /// https://www.mankier.com/3/xcb_client_message_event_t
    // ClientMessage {
    //     /// The ID of the window that sent the message
    //     id: Window,
    //     /// The data type being set
    //     dtype: String,
    //     /// The data itself
    //     data: Vec<usize>,
    // },
}

/// Handles communication with an X server via xcb
pub struct XcbConnection {
    // conn: xcb::Connection,
    conn: ewmh::Connection,
    preferred_screen: i32,
    root: Window,
    atoms: InternedAtoms,
    // check_win: Window,
    // auto_float_types: Vec<&'static str>,
    // randr_base: u8,
}

impl XcbConnection {
    pub fn new() -> Result<XcbConnection> {
        let (conn, preferred_screen) = xcb::Connection::connect(None).context(format!("Unable to connection to X server"))?;
        //TODO: handle error using anyhow
        let conn = ewmh::Connection::connect(conn).map_err(|(e, _)| e)?;
        // let root = conn
        //     .get_setup()
        //     .roots()
        //     .nth(preferred_screen as usize)
        //     .ok_or_else(|| format_err!("Invalid screen"))?
        //     .root();

        let root = conn
            .get_setup()
            .roots()
            .nth(preferred_screen as usize)
            .context(format!("Unable to get handle for the preferred screen"))?
            .root();

        let atoms = InternedAtoms::new(&conn).context("Failed to intern atoms")?;

        // xcb docs: https://www.mankier.com/3/xcb_create_window
        // xcb::create_window(
        //     &conn,                   // xcb connection to X11
        //     0,                       // new window's depth
        //     check_win,               // ID to be used for referring to the window
        //     root,                    // parent window
        //     0,                       // x-coordinate
        //     0,                       // y-coordinate
        //     1,                       // width
        //     1,                       // height
        //     0,                       // border width
        //     WINDOW_CLASS_INPUT_ONLY, // class (i _think_ 0 == COPY_FROM_PARENT?)
        //     0,                       // visual (i _think_ 0 == COPY_FROM_PARENT?)
        //     &[],                     // value list? (value mask? not documented either way...)
        // );
        Ok(XcbConnection {
            conn,
            preferred_screen,
            root,
            // check_win,
            atoms,
            // auto_float_types,
            // randr_base,
        })
    }

    pub fn raw_conn(&self) -> &ewmh::Connection {
        &self.conn
    }

    pub fn root(&self) -> Window {
        self.root
    }

    pub fn register_wm(&self) -> Result<()> {
        // Register for substructure redirection
        // https://jichu4n.com/posts/how-x-window-managers-work-and-how-to-write-one-part-i/#substructure-redirection
        xcb::change_window_attributes_checked(&self.conn, self.root, ROOT_EVENT_MASK)
            .request_check()
            .context("Could not register SUBSTRUCTURE_NOTIFY/REDIRECT")?;
        // TODO: necessary?
        self.conn.flush();
        Ok(())
    }

    pub fn flush(&self) -> bool {
        self.conn.flush()
    }

    pub fn grab_key(&self, key: &XcbKey) {
        // xcb docs: https://www.mankier.com/3/xcb_grab_key
        xcb::grab_key(
            &self.conn,      // xcb connection to X11
            false,           // don't pass grabbed events through to the client
            self.root,       // the window to grab: in this case the root window
            key.mod_mask,    // modifiers to grab
            key.code,        // keycode to grab
            GRAB_MODE_ASYNC, // don't lock pointer input while grabbing
            GRAB_MODE_ASYNC, // don't lock keyboard input while grabbing
        );

    }

    pub fn grab_button() {
        // TODO: this needs to be more configurable by the user
        // for mouse_button in &[1, 3] {
        //     // xcb docs: https://www.mankier.com/3/xcb_grab_button
        //     xcb::grab_button(
        //         &self.conn,             // xcb connection to X11
        //         false,                  // don't pass grabbed events through to the client
        //         self.root,              // the window to grab: in this case the root window
        //         MOUSE_MASK,             // which events are reported to the client
        //         GRAB_MODE_ASYNC,        // don't lock pointer input while grabbing
        //         GRAB_MODE_ASYNC,        // don't lock keyboard input while grabbing
        //         xcb::NONE,              // don't confine the cursor to a specific window
        //         xcb::NONE,              // don't change the cursor type
        //         *mouse_button,          // the button to grab
        //         xcb::MOD_MASK_4 as u16, // modifiers to grab
        //     );
        // }

    }

    pub fn register_events(&self, id: Window, events: u32) -> Result<()> {

        xcb::change_window_attributes_checked(&self.conn, id, &[(xcb::CW_EVENT_MASK, events)])
            .request_check()
            .context(format!("Could not register events: {}", events))?;
        Ok(())
    }

    pub fn focus_client(&self, id: Window) {
        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            &self.conn,         // xcb connection to X11
            INPUT_FOCUS_PARENT, // focus the parent when focus is lost
            // INPUT_FOCUS_POINTER_ROOT
            id,                 // window to focus
            xcb::CURRENT_TIME,  // current time to avoid network race conditions (0 == current time)
        );
        ewmh::set_active_window(&self.conn, self.preferred_screen, id);
    }

    // pub fn focus_client(&self, id: Window) {
    //     let prop = self.known_atom(Atom::NetActiveWindow);

    //     // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
    //     xcb::set_input_focus(
    //         &self.conn,         // xcb connection to X11
    //         INPUT_FOCUS_PARENT, // focus the parent when focus is lost
    //         id,                 // window to focus
    //         0,                  // current time to avoid network race conditions (0 == current time)
    //     );

    //     // xcb docs: https://www.mankier.com/3/xcb_change_property
    //     xcb::change_property(
    //         &self.conn,        // xcb connection to X11
    //         PROP_MODE_REPLACE, // discard current prop and replace
    //         self.root,         // window to change prop on
    //         prop,              // prop to change
    //         ATOM_WINDOW,       // type of prop
    //         32,                // data format (8/16/32-bit)
    //         &[id],             // data
    //     );
    // }

    // - Release all of the keybindings we are holding on to
    // - destroy the check window
    // - mark ourselves as no longer being the active root window
    pub fn cleanup(&self) {
        // xcb docs: https://www.mankier.com/3/xcb_ungrab_key
        xcb::ungrab_key(
            &self.conn, // xcb connection to X11
            xcb::GRAB_ANY as u8,
            self.root, // the window to ungrab keys for
            xcb::MOD_MASK_ANY as u16,
        );
        // xcb::destroy_window(&self.conn, self.check_win);
        // xcb::delete_property(
        //     &self.conn,
        //     self.root,
        //     self.known_atom(Atom::NetActiveWindow),
        // );
    }

    pub fn get_wm_name(&self, id: Window) -> Result<String> {
        // TODO: ewmh defines _NET_WM_NAME, which should be used in preference to WM_NAME
        // ewmh::get_wm_name probably deal with this for us
        Ok(ewmh::get_wm_name(&self.conn, id).get_reply()?.string().to_string())
    }

    pub fn get_wm_class(&self, win: Window) -> Result<String> {
        Ok(icccm::get_wm_class(&self.conn, win).get_reply()?.class().to_string())
    }

    pub fn get_text_property(&self, win: Window, atom: Atom) -> Result<String> {
        // Ok(icccm::get_text_property(&self.conn, win, atom).get_reply()?.name().to_string())
            // String::from_utf8(cookie.get_reply()?.value().to_vec())?
        let prop = icccm::get_text_property(&self.conn, win, atom).get_reply()?.name().to_string();
        // debug!("got property: {:?}", prop);
        Ok(prop)
    }

    pub fn set_text_property(&self, win: Window, atom: Atom, data: &str) {
        xcb::change_property(
            &self.conn,                        // xcb connection to X11
            PROP_MODE_REPLACE,                 // discard current prop and replace
            win,                    // window to change prop on
            atom,  // prop to change
            self.atoms.UTF8_STRING, // type of prop
            8,                                 // data format (8/16/32-bit)
            data.as_bytes(),                // data
        );
    }

    // fn atom_prop(&self, id: Window, atom: Atom) -> Result<u32> {
    //     // xcb docs: https://www.mankier.com/3/xcb_get_property
    //     let cookie = xcb::get_property(
    //         &self.conn,       // xcb connection to X11
    //         false,            // should the property be deleted
    //         id,               // target window to query
    //         atom, // the property we want
    //         xcb::ATOM_ANY,    // the type of the property
    //         0,                // offset in the property to retrieve data from
    //         1024,             // how many 32bit multiples of data to retrieve
    //     );

    //     let reply = cookie.get_reply()?;
    //     if reply.value_len() <= 0 {
    //         // TODO: fix print
    //         Err(anyhow!("property '{}' was empty for id: {}", atom, id))
    //     } else {
    //         Ok(reply.value()[0])
    //     }
    // }


    pub fn wait_for_event(&self) -> Option<XEvent> {
        self.conn.wait_for_event().and_then(|event| {
            let etype = event.response_type() & XCB_RESPONSE_TYPE_MASK;
            // TODO: Check for error for requests which have no reply
            // https://www.x.org/releases/X11R7.7/doc/man/man3/xcb-requests.3.xhtml#heading5

            // let etype = event.response_type();
            // Need to apply the randr_base mask as well which doesn't seem to work in 'match'
            // if etype == self.randr_base + xcb::randr::NOTIFY {
            //     return Some(XEvent::RandrNotify);
            // }
            // debug!("event {:?}", etype);

            match etype {
                // xcb::CREATE_NOTIFY => {
                //     let e:&xcb::CreateNotifyEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::CreateNotify {
                //         id: e.window(),
                //     })
                // }
                xcb::CLIENT_MESSAGE => {
                    let e:&xcb::ClientMessageEvent = unsafe { xcb::cast_event(&event) };
                    let mut data: [u8; 20] = [0; 20];
                    let data_ref = e.data().data8();
                    data.copy_from_slice(data_ref);
                    Some(XEvent::ClientCommand {
                        format: e.format(),
                        window: e.window(),
                        atom: e.type_(),
                        data: data,
                    })
                }
                // xcb::BUTTON_PRESS => None,

                // xcb::BUTTON_RELEASE => None,

                xcb::KEY_PRESS => {
                    let e: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::KeyPress {
                        code: XcbKey::from_key_press(e),
                    })
                }

                xcb::MAP_REQUEST => {
                    let e: &xcb::MapRequestEvent = unsafe { xcb::cast_event(&event) };
                    let id = e.window();
                    xcb::xproto::get_window_attributes(&self.conn, id)
                        .get_reply()
                        .ok()
                        .and_then(|r| {
                            Some(XEvent::MapRequest {
                                id,
                                ignore: r.override_redirect(),
                            })
                        })
                }

                // xcb::ENTER_NOTIFY => {
                //     let e: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::Enter {
                //         id: e.event(),
                //         rpt: Point::new(e.root_x() as u32, e.root_y() as u32),
                //         wpt: Point::new(e.event_x() as u32, e.event_y() as u32),
                //     })
                // }

                // xcb::LEAVE_NOTIFY => {
                //     let e: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::Leave {
                //         id: e.event(),
                //         rpt: Point::new(e.root_x() as u32, e.root_y() as u32),
                //         wpt: Point::new(e.event_x() as u32, e.event_y() as u32),
                //     })
                // }

                // xcb::FOCUS_IN => {
                //     let e: &xcb::FocusInEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::FocusIn { id: e.event() })
                // }

                // xcb::FOCUS_OUT => {
                //     let e: &xcb::FocusOutEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::FocusOut { id: e.event() })
                // }

                xcb::DESTROY_NOTIFY => {
                    let e: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::Destroy { id: e.window() })
                }

                // xcb::randr::SCREEN_CHANGE_NOTIFY => Some(XEvent::ScreenChange),

                // xcb::CONFIGURE_NOTIFY => {
                //     let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                //     Some(XEvent::ConfigureNotify {
                //         id: e.window(),
                //         r: Region::new(
                //             e.x() as u32,
                //             e.y() as u32,
                //             e.width() as u32,
                //             e.height() as u32,
                //         ),
                //         is_root: e.window() == self.root,
                //     })
                // }

                xcb::CONFIGURE_REQUEST => {
                    let e: &xcb::ConfigureRequestEvent = unsafe { xcb::cast_event(&event) };
                    Some(XEvent::ConfigureRequest {
                        win: e.window(),
                    })
                }

                // xcb::CLIENT_MESSAGE => {
                //     let e: &xcb::ClientMessageEvent = unsafe { xcb::cast_event(&event) };
                //     xcb::xproto::get_atom_name(&self.conn, e.type_())
                //         .get_reply()
                //         .ok()
                //         .map(|a| XEvent::ClientMessage {
                //             id: e.window(),
                //             dtype: a.name().to_string(),
                //             data: match e.format() {
                //                 8 => e.data().data8().iter().map(|&d| d as usize).collect(),
                //                 16 => e.data().data16().iter().map(|&d| d as usize).collect(),
                //                 32 => e.data().data32().iter().map(|&d| d as usize).collect(),
                //                 _ => unreachable!(
                //                     "ClientMessageEvent.format should really be an enum..."
                //                 ),
                //             },
                //         })
                // }

                xcb::PROPERTY_NOTIFY => {
                    let e: &xcb::PropertyNotifyEvent = unsafe { xcb::cast_event(&event) };
                    let atom = e.atom();
                    let is_root = e.window() == self.root;
                    Some(XEvent::PropertyNotify {
                        id: e.window(),
                        atom,
                        is_root,
                    })
                    // xcb::xproto::get_atom_name(&self.conn, e.atom())
                    //     .get_reply()
                    //     .ok()
                    //     .and_then(|a| {
                    //         let atom = a.name().to_string();
                    //         let is_root = e.window() == self.root;
                    //         if is_root && !(atom == "WM_NAME" || atom == "_NET_WM_NAME") {
                    //             None
                    //         } else {
                    //             Some(XEvent::PropertyNotify {
                    //                 id: e.window(),
                    //                 atom,
                    //                 is_root,
                    //             })
                    //         }
                    //     })
                }

                // NOTE: ignoring other event types
                _ => None,
            }
        })
    }

    pub fn mark_new_window(&self, id: Window) {
        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, id, NEW_WINDOW_MASK);
    }

    pub fn map_window(&self, id: Window) {
        xcb::map_window(&self.conn, id);
    }

    pub fn unmap_window(&self, id: Window) {
        xcb::unmap_window(&self.conn, id);
    }

    // fn intern_atom(&self, atom: &str) -> Result<u32> {
    //     self.atom(atom)
    // }
    /// Returns the Atom identifier associated with the atom_name str.
    // pub fn intern_atom(conn: &xcb::Connection, atom_name: &str) -> Result<xcb::Atom> {
    pub fn intern_atom(&self, atom_name: &str) -> Result<Atom> {
        Ok(xcb::intern_atom(&self.conn, false, atom_name).get_reply()?.atom())
    }


    // Return the cached atom if it's one we know, falling back to interning the atom if we need to.
    // fn atom(&self, name: &str) -> Result<u32> {
    //     Ok(match self.atoms.get(name) {
    //         Some(&a) => a,
    //         None => xcb::intern_atom(&self.conn, false, name)
    //             .get_reply()?
    //             .atom(),
    //     })
    // }

    // /// Returns the Atom identifier associated with the atom_name str.
    // pub fn intern_atom(&self, atom_name: &str) -> Result<xcb::Atom> {
    //     Ok(xcb::intern_atom(self.conn.get_raw_conn(), false, atom_name).get_reply()?.atom())
    // }

    /// Queries the WM_PROTOCOLS property of a window, returning a list of the
    /// protocols that it supports.
    fn get_wm_protocols(&self, id: Window) -> Result<Vec<xcb::Atom>> {
        // let reply = icccm::get_wm_protocols(&self.conn, id, self.atoms.WM_PROTOCOLS)
        let reply = icccm::get_wm_protocols(&self.conn, id, self.conn.WM_PROTOCOLS())
            .get_reply()?;
        Ok(reply.atoms().to_vec())
    }

    fn send_client_event(&self, id: Window, atom: Atom) -> Result<()> {
        // TODO: use ewmh
        let wm_protocols = self.conn.WM_PROTOCOLS();
        let data = xcb::ClientMessageData::from_data32([atom, xcb::CURRENT_TIME, 0, 0, 0]);
        let event = xcb::ClientMessageEvent::new(32, id, wm_protocols, data);
        xcb::send_event(&self.conn, false, id, xcb::EVENT_MASK_NO_EVENT, &event);
        Ok(())
    }

    /// Closes a window.
    ///
    /// The window will be closed gracefully using the ICCCM WM_DELETE_WINDOW
    /// protocol if it is supported.
    // TODO: query supported protocols everytime?
    pub fn delete_window(&self, id: Window) {
        let atom = self.atoms.WM_DELETE_WINDOW;
        let has_wm_delete_window = self
            .get_wm_protocols(id)
            .map(|protocols| protocols.contains(&atom))
            .unwrap_or(false);

        if has_wm_delete_window {
            info!("Closing window {} using WM_DELETE", id);
            self.send_client_event(id, self.atoms.WM_DELETE_WINDOW);
            // let data = xcb::ClientMessageData::from_data32([
            //     atom,
            //     xcb::CURRENT_TIME,
            //     0,
            //     0,
            //     0,
            // ]);
            // let event =
            //     xcb::ClientMessageEvent::new(32, id, self.atoms.WM_PROTOCOLS, data);
            // xcb::send_event(
            //     &self.conn,
            //     false,
            //     id,
            //     xcb::EVENT_MASK_NO_EVENT,
            //     &event,
            // );
        } else {
            info!("Closing window {} using xcb::destroy_window()", id);
            xcb::destroy_window(&self.conn, id);
        }
    }
}

