// based on the list output example from the smithay client toolkit
#[macro_use]
pub(crate) mod protocol_macro;
pub(crate) mod ext;
mod workspace_manager;
pub(crate) mod workspace_state;
pub(crate) mod workspace_protocol_ext_v0;
mod workspace_protocol_ext_v1;
mod workspace_protocol_cosmic_v1;
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
