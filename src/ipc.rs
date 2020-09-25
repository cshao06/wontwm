use anyhow::{Result, Context, anyhow};
use xcb::{Window, Atom};
use xcb_util::{ewmh, icccm};

use crate::{
    xconnection::{XcbConnection, XEvent},
};

const IPC_WINDOW_EVENT_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_PROPERTY_CHANGE,
)];
const COPY_FROM_PARENT: u8 = xcb::COPY_FROM_PARENT as u8;
const WINDOW_CLASS_INPUT_ONLY: u16 = xcb::WINDOW_CLASS_INPUT_ONLY as u16;
const CONFIG_WINDOW_X: u16 = xcb::CONFIG_WINDOW_X as u16;


pub const IPC_WINDOW_NAME: &str = "WONTWM_IPC";
pub const IPC_WINDOW_CLASS: &str = "WONTWM_IPC";
pub const IPC_COMMAND_ATOM: &str = "_WONTWM_IPC_COMMAND";
pub const IPC_STATE_ATOM: &str = "_WONTWM_IPC_STATE";
pub const IPC_STATE_SERVER_READY: &str = "server_ready";
pub const IPC_STATE_REPLY_READY: &str = "reply_ready";
pub const IPC_STATE_SUCCESS: &str = "success";
pub const IPC_STATE_ERROR: &str = "error";

pub enum Command {
    BindKey,
    UnbindKey,
    ListBindings,
    Exit,
    Reload,
    Set,
    // Invalid,
}

pub struct IpcClient {
    conn: XcbConnection,
    root: Window,
    ipc_win: Window, 
    atom_command: Atom,
    atom_state: Atom,
}

impl IpcClient {
    pub fn new() -> Result<IpcClient> {
        let conn = XcbConnection::new()?;

        // TODO: avoid raw_conn, implement deref
        let raw_conn = conn.raw_conn();

        // Handle error?
        // xcb::grab_server(raw_conn);

        let root = conn.root();
        let ipc_win = raw_conn.generate_id();
        xcb::create_window_checked(
            raw_conn,                   // xcb connection to X11
            COPY_FROM_PARENT,   // new window's depth
            ipc_win,                 // ID to be used for referring to the window
            root,                    // parent window
            0,                       // x-coordinate
            0,                       // y-coordinate
            1,                       // width, can't be 0
            1,                       // height, can't be 0
            0,                       // border width
            WINDOW_CLASS_INPUT_ONLY, // class
            xcb::COPY_FROM_PARENT,   // visual
            IPC_WINDOW_EVENT_MASK,                     // value list
        ).request_check().context("Failed to create a window for IPC")?;
        
        // TODO: handle error?
        ewmh::set_wm_name_checked(raw_conn, ipc_win, IPC_WINDOW_NAME).request_check()?;
        icccm::set_wm_class_checked(raw_conn, ipc_win, IPC_WINDOW_CLASS, IPC_WINDOW_CLASS).request_check()?;
        conn.flush();

        // Notify the wm that the WM_CLASS of the client IPC window is ready
        xcb::configure_window(raw_conn, ipc_win, &[(CONFIG_WINDOW_X, 0)]);
        // xcb::ungrab_server_checked(raw_conn).request_check()?;
        // print!("Ungrabbed server");

        let atom_command = conn.intern_atom(IPC_COMMAND_ATOM)?;
        let atom_state = conn.intern_atom(IPC_STATE_ATOM)?;

        Ok(IpcClient {
            conn,
            root,
            ipc_win,
            atom_command,
            atom_state,
        })
    }

    // pub fn send_command(&self, cmd: Command, args: Vec<String>) -> Result<()> {
    pub fn send_command(&self, command: Vec<String>) -> Result<()> {
        // while !self.is_server_ready() {
        //     std::thread::sleep(std::time::Duration::from_millis(10));
        // }
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                // debug!("got XEvent: {:?}", event);
                match event {
                    XEvent::PropertyNotify { id, atom, is_root } => {
                        assert!(id == self.ipc_win);
                        if atom == self.atom_state {
                            if self.conn.get_text_property(id, atom)? == IPC_STATE_SERVER_READY {
                                debug!("ipc client: IPC server ready");
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // TODO: checked?
        self.conn.set_text_property(self.ipc_win, self.atom_command, &command.join(" "));
        self.conn.flush();
        Ok(())
    }

    // pub fn is_server_ready(&self) -> bool {
    //     // debug!("checking server");
    //     if let Ok(state) = self.conn.get_text_property(self.ipc_win, self.atom_state) {
    //         // debug!("state: {:?}", state);
    //         state == IPC_STATE_SERVER_READY
    //     } else {
    //         false
    //     }
        
    // }

    // pub fn is_reply_ready(&self) -> bool {
    //     // debug!("checking server");
    //     if let Ok(state) = self.conn.get_text_property(self.ipc_win, self.atom_state) {
    //         // debug!("state: {:?}", state);
    //         state == IPC_STATE_REPLY_READY
    //     } else {
    //         false
    //     }
        
    // }

    pub fn get_reply(&self) -> Result<String> {
        loop {
            // std::thread::sleep(std::time::Duration::from_millis(1000));
            if let Some(event) = self.conn.wait_for_event() {
                debug!("got XEvent: {:?}", event);
                match event {
                    XEvent::PropertyNotify { id, atom, is_root } => {
                        assert!(id == self.ipc_win);
                        if atom == self.atom_state {
                            match self.conn.get_text_property(id, atom)?.as_str() {
                                // No reply required
                                IPC_STATE_SUCCESS => return Ok(IPC_STATE_SUCCESS.to_string()),
                                IPC_STATE_REPLY_READY => {
                                    return self.conn.get_text_property(id, self.atom_command);
                                }
                                IPC_STATE_ERROR => {
                                    return Ok(IPC_STATE_ERROR.to_string())
                                }
                                _ => return Err(anyhow!("Got an invalid state")),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

}

pub struct IpcServer<'a> {
    conn: &'a XcbConnection,
    atom_command: Atom,
    atom_state: Atom,
}

impl<'a> IpcServer<'a> {
    pub fn new(conn: &XcbConnection) -> Result<IpcServer> {

        let atom_command = conn.intern_atom(IPC_COMMAND_ATOM)?;
        let atom_state = conn.intern_atom(IPC_STATE_ATOM)?;
        Ok(IpcServer {
            conn,
            atom_command,
            atom_state,
        })
    }

    pub fn is_ipc_client(&self, win: Window) -> bool {
        if let Ok(class) = self.conn.get_wm_class(win) {
            if class == IPC_WINDOW_CLASS {
                return true
            }
        }
        // debug!("class: {}", c);
        // debug!("handle_configure_request: Failed to get window class {:?}", e)
        false
    }

    pub fn listen_client(&self, win: Window) {
        // TODO: handle error
        self.conn.register_events(win, xcb::EVENT_MASK_PROPERTY_CHANGE);
        self.conn.set_text_property(win, self.atom_state, IPC_STATE_SERVER_READY);
        debug!("Ready for property change on the ipc client window");
    }

    pub fn get_command(&self, win: Window, atom: Atom) -> Option<String> {
        if atom == self.atom_command {
            match self.conn.get_text_property(win, atom) {
                Ok(c) => Some(c),
                Err(e) => {
                    debug!("Failed to get ipc command property {:?}", e);
                    None
                }
            }
        } else {
            None
        }
    }


    pub fn send_reply(&self, win: Window, data: &str) {
        self.conn.set_text_property(win, self.atom_state, data);
    }
}
