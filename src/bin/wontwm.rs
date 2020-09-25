/// A default 'anyhow' based result type
type Result<T> = anyhow::Result<T>;
// pub type Result<T> = std::result::Result<T, Error>;
use wontwm::{WindowManager, XcbConnection};
use simplelog::{LevelFilter, SimpleLogger};

fn main() -> Result<()> {
    // -- logging --
    // SimpleLogger::init(LevelFilter::Info, simplelog::Config::default())?;
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default())?;

    // let mut config = Config::default();

    let conn = XcbConnection::new()?;
    let mut wm = WindowManager::new(&conn)?;

    // spawn(format!("{}/bin/scripts/penrose-startup.sh", home));
    // wm.grab_keys_and_run(key_bindings);
    wm.run();

    Ok(())
}
