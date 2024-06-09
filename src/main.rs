// based on the list output example from the smithay client toolkit
pub(crate) mod ext;
mod workspace_manager;
mod workspace_state;
pub(crate) mod cli;

use clap::Parser;
use cli::Cli;
use workspace_manager::WorkspaceManager;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Cli::parse();
    WorkspaceManager::exec(&args)?;

    Ok(())
}
