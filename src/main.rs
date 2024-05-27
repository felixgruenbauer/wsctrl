// based on the list output example from the smithay client toolkit
mod ext;
mod workspace_management;
mod workspace_state;

use clap::Parser;
use std::error::Error;
use workspace_management::WorkspaceManagement;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let app = WorkspaceManagement::parse();
    app.exec()?;

    Ok(())
}
