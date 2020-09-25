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

use wontwm::{
    ipc::{IpcClient, Command}
    // xconnection::{XcbConnection, XEvent},
};

use simplelog::{LevelFilter, SimpleLogger};

fn main() -> Result<()> {
    // -- logging --
    // SimpleLogger::init(LevelFilter::Info, simplelog::Config::default())?;
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default())?;

    // TODO: use clap to build cmd arguments
    let cmd = env::args().nth(1).context("No command specified")?;
    let cmd = match cmd.as_str() {
        "bindkey"       => Command::BindKey,
        "unbindkey"     => Command::UnbindKey,
        "list_bindings" => Command::ListBindings,
        "quit"          => Command::Quit,
        "reload"        => Command::Reload,
        "set"           => Command::Set,
        _               => return Err(anyhow!("Invalid command {}", cmd)),
    };
    // let args: Vec<String> = env::args().skip(2).collect();
    let args: Vec<String> = env::args().skip(1).collect();

    // let ipc = IpcClient::new().context(format!("Failed to init ipc client"))?;
    let ipc = IpcClient::new()?;
    // ipc.send_command(cmd, args).context("Failed to send command to wm")?;
    ipc.send_command(args).context("Failed to send command to wm")?;
    let reply = ipc.get_reply()?;
    println!("{}", reply);

    Ok(())
}

