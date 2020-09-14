use std::{
    collections::{HashMap, VecDeque},
    process::{Command, Stdio},
};

// use penrose::{XcbConnection};
// use penrose::xconnection::XConn;
use crate::{
    xconnection::{XcbConnection, XcbKey},
    wm::WindowManager,
};

/// Some Action to be run by a user key binding
// struct Action {
//     cmd: String,
//     args: String,
// }
// type Action = String;
// pub type Command = Rc<dyn Fn(&mut Lanta) -> Result<()>>;
// type Action = Box<dyn FnMut(&mut WindowManager) -> ()>;
// type Action = Box<dyn Fn(&mut WindowManager) -> ()>;
use std::rc::Rc;
type Action = Rc<dyn Fn(&mut WindowManager)>;
// type Action = Box<dyn Fn() -> ()>;
// /// Some action to be run by a user key binding
// pub type FireAndForget = Box<dyn FnMut(&mut WindowManager) -> ()>;

// /// User defined key bindings
// pub type KeyBindings = HashMap<KeyCode, FireAndForget>;

type KeyBindings = HashMap<XcbKey, Action>;
// pub type KeyBindings = HashMap<XcbKey, Action>;

struct UserFmtBinding {
    key: &'static str,
    // action: &'static str,
    action: &'static str,
    args: Option<&'static str>,
}

static DEFAULT_BINDINGS: &'static [UserFmtBinding] = &[
    UserFmtBinding {key: "S-Return", action: "exec", args: Some("alacritty")},
    UserFmtBinding {key: "S-e",      action: "exec", args: Some("spacefm")},
    UserFmtBinding {key: "S-q",      action: "kill_client", args: None},
];

pub struct KeyManager {
    bindings: KeyBindings,
    keycodes: KeymapTable,
    // conn: &'a XcbConnection,
}
// pub struct KeyManager<T: &XConn> {
//     bindings: KeyBindings,
//     conn: T,
// }

impl KeyManager {
    pub fn new() -> Self {
        // let km = KeyManager {
        KeyManager {
            bindings: KeyBindings::new(),
            keycodes: keycodes_from_xmodmap(),
        }
        // };
        // default_bindings = [
        //     UserFmtBinding {key: "M-Return", action: "exec alacritty"},
        //     UserFmtBinding {key: "M-q", action: "close_window"},
        // ];
        // km
    }

    // pub fn set_default_bindings(&mut self, conn: &impl XConn) {
    // TODO: combine with new()
    pub fn set_default_bindings(&mut self, conn: &XcbConnection) {
    // pub fn set_default_bindings(bindings: &mut KeyBindings, conn: &XcbConnection) {
        for binding in DEFAULT_BINDINGS.iter() {
            match parse_key_binding(binding.key, &self.keycodes) {
                None => panic!("invalid key binding: {}", binding.key),
                // UserFmtBinding {key: "S-Return", action: 
                //     Box::new(move |_: &mut WindowManager| {
                //         spawn("alacritty");
                //     }) as Action
                // },
                Some(key_code) => {
                    // let action = &binding.action;
                    // let mut iter = action.split_whitespace();
                    if binding.action == "exec" {
                        self.bindings.insert(key_code, 
                        // bindings.insert(key_code, 
                            // Box::new(move |ref mut wm| {
                            Rc::new(move |ref mut _wm| {
                                spawn(binding.args.unwrap());
                            })
                            // }) as Action
                        );
                    } else {
                        self.bindings.insert(key_code, 
                        // bindings.insert(key_code, 
                            // Box::new(move |ref mut wm| {
                            Rc::new(move |ref mut wm| {
                                wm.kill_client();
                            })
                            // }) as Action
                        );
                    }
                    // match iter.next() {
                    //     Some(s) => if s == "exec" {spawn("alacritty");},
                    //     None => {
                    //         debug!("keypress error");
                    //         return;
                    //     }
                    // }

                    // self.bindings.insert(key_code, binding.action.to_string()),
                },
            };
        }
        for key in self.bindings.keys() {
        // for key in bindings.keys() {
            conn.grab_key(key);
        }
    }

    pub fn get_action(&self, key: &XcbKey) -> Option<Action> {
        self.bindings.get(key).cloned()
    }

    // pub fn bind_key(&mut self, key: &str, mut action: Vec<&'static str>) {
    pub fn bind_key(&mut self, conn: &XcbConnection, key: &str, mut action: Vec<&str>) {

            match parse_key_binding(key, &self.keycodes) {
                None => panic!("invalid key binding: {}", key),
                Some(key_code) => {
                    // debug!("key: {:?}", key_code);
                    conn.grab_key(&key_code);
                    let action_str = action.remove(0);
                    // remove trailing nul
                    if action_str == "exec" {
                        let action = action.join(" ");
                        // debug!("action: {:?}", action);
                        self.bindings.insert(key_code, 
                            Rc::new(move |ref mut _wm| {
                                spawn(action.trim_end_matches(char::from(0)));
                                // spawn(action);
                            })
                        );
                        // debug!("Map: {:?}", self.bindings.keys());
                    } else {
                        self.bindings.insert(key_code, 
                            Rc::new(move |ref mut wm| {
                                wm.kill_client();
                            })
                        );
                    }
                },
            }
    }
}

// TODO: use xcb_util keysym
/// Map xmodmap key names to their X key code so that we can bind them by name
type KeymapTable = HashMap<String, u8>;

/**
 * Run the xmodmap command to dump the system keymap table.
 *
 * This is done in a form that we can load in and convert back to key
 * codes. This lets the user define key bindings in the way that they
 * would expect while also ensuring that it is east to debug any odd
 * issues with bindings by referring the user to the xmodmap output.
 */
fn keycodes_from_xmodmap() -> KeymapTable {
    match Command::new("xmodmap").arg("-pke").output() {
        Err(e) => panic!("unable to fetch keycodes via xmodmap: {}", e),
        Ok(o) => match String::from_utf8(o.stdout) {
            Err(e) => panic!("invalid utf8 from xmodmap: {}", e),
            Ok(s) => s
                .lines()
                .flat_map(|l| {
                    let mut words = l.split_whitespace(); // keycode <code> = <names ...>
                    let key_code: u8 = words.nth(1).unwrap().parse().unwrap();
                    words.skip(1).map(move |name| (name.into(), key_code))
                })
                .collect::<KeymapTable>(),
        },
    }
}

/**
 * Convert user friendly key bindings into X keycodes.
 *
 * Allows the user to define their keybindings using the gen_keybindings macro
 * which calls through to this. Bindings are of the form '<MOD>-<key name>'
 * with multipple modifiers being allowed, and keynames being taken from the
 * output of 'xmodmap -pke'.
 *
 * Allowed modifiers are:
 *   M - Super
 *   A - Alt
 *   C - Ctrl
 *   S - Shift
 *
 * The user friendly patterns are parsed into a modifier mask and X key code
 * pair that is then grabbed by penrose to trigger the bound action.
 */
fn parse_key_binding(pattern: impl Into<String>, known_codes: &KeymapTable) -> Option<XcbKey> {
    let s = pattern.into();
    let mut parts: Vec<&str> = s.split('-').collect();
    match known_codes.get(parts.remove(parts.len() - 1)) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|s| match s {
                    &"A" => xcb::MOD_MASK_1,
                    &"M" => xcb::MOD_MASK_4,
                    &"S" => xcb::MOD_MASK_SHIFT,
                    &"C" => xcb::MOD_MASK_CONTROL,
                    &_ => panic!("invalid key binding prefix: {}", s),
                })
                .fold(0, |acc, v| acc | v);

            // debug!("binding '{}' as [{}, {}]", s, mask, code);
            Some(XcbKey {
                mod_mask: mask as u16,
                code: *code,
            })
        }
        None => None,
    }
}


/**
 * Run an external command
 *
 * This redirects the process stdout and stderr to /dev/null.
 * Logs a warning if there were any errors in kicking off the process.
 */
fn spawn(cmd: impl Into<String>) {
    let s = cmd.into();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .args(&parts[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new(parts[0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    };

    if let Err(e) = result {
        warn!("error spawning external program: {}", e);
    }
}
