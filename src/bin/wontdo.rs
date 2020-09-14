#[macro_use]
extern crate log;
// #[macro_use]
// extern crate lazy_static;

// #[macro_use]
// extern crate wontwm;

use std::{
    env,
};
use anyhow::{Result, Context, anyhow};
use xcb_util::{ewmh, icccm};

use wontwm::wm::Commands;

// use wontwm::{
//     XcbConnection,
// };

// /// A default 'anyhow' based result type
// type Result<T> = anyhow::Result<T>;

fn main() -> Result<()> {
    // TODO: use clap to build cmd arguments
    let cmd = env::args().nth(1).context(format!("No command specified"))?;
    let cmd = match cmd.as_str() {
        "bindkey"       => Commands::BindKey,
        "unbindkey"     => Commands::UnbindKey,
        "list_bindings" => Commands::ListBindings,
        "quit"          => Commands::Quit,
        "reload"        => Commands::Reload,
        "set"           => Commands::Set,
        _               => return Err(anyhow!("Invalid command {}", cmd)),
    };

    let args: Vec<String> = env::args().skip(2).collect();
    send_command(cmd, args).expect("Failed to send command to wm");
    Ok(())
}

fn send_command(cmd: Commands, args: Vec<String>) -> Result<()> {

    // let conn = XcbConnection::new()?;

    let (conn, preferred_screen) = xcb::Connection::connect(None).context(format!("Unable to connection to X server"))?;
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

    let atom = xcb::intern_atom(&conn, false, "_CUSTOM_CLIENT_COMMAND").get_reply()?.atom();
    let mut a: [u8; 20] = [0; 20];
    a[0] = cmd as u8;
    let c = args.join(" ");
    let b = c.as_bytes();
    a[1..b.len()+1].copy_from_slice(b);
    let data = xcb::ClientMessageData::from_data8(a);
    // print!("{}", std::str::from_utf8(&a).unwrap());
    let event = xcb::ClientMessageEvent::new(8, root, atom, data);
    // xcb::send_event(&conn, false, id, xcb::EVENT_MASK_NO_EVENT, &event);
    // xcb::send_event(&conn, false, root, xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT, &event)
    // TODO: why NOTIFY? Why does REDIRECT not work?
    xcb::send_event(&conn, false, root, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY, &event)
    // xcb::send_event(&conn, false, root, xcb::EVENT_MASK_NO_EVENT, &event)
        .request_check()
        .context("Could not register SUBSTRUCTURE_NOTIFY/REDIRECT")?;
    Ok(())
}

// IPC_ATOM_COMMAND "_"__NAME_UPPERCASE__"_CLIENT_COMMAND"
// IPC_ATOM_STATE "_"__NAME_UPPERCASE__"_STATE"
// IPC_ATOM_INSETS "_"__NAME_UPPERCASE__"_INSETS"
