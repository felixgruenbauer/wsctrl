use clap::{Args, Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(author = "fg", version = "0.1", about = "Manage workspaces via the wayland protocol extension 'ext-workspace-v1'.", long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[command(flatten)]
    pub global_opts: GlobalOpts,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug)]
pub struct GlobalOpts {
    #[clap(long)]
    pub protocol_version: Option<u8>
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[clap(
        visible_alias = "a",
        about = "Activate selected workspace. Some options require an output selection."
    )]
    Activate(WorkspaceArgs),
    #[clap(
        visible_alias = "d",
        about = "Deactivate selected workspace. Some options require an output selection."
    )]
    Deactivate(WorkspaceArgs),
    #[clap(visible_alias = "s", about = "Assign workspace to selected output.")]
    Assign{
        #[command(flatten)]
        workspace_args: WorkspaceArgs,
        #[command(flatten)]
        target: TargetOutput 
    },
    #[clap(
        visible_alias = "r",
        about = "Remove selected workspace. Some options require an output selection."
    )]
    Remove(WorkspaceArgs),
    #[clap(visible_alias = "cw", about = "Create workspace on selected output.")]
    CreateWorkspace {
        #[clap(long, requires = "output")]
        workspace_name: String,
        #[command(flatten)]
        output: OutputSelector,
    },
    #[clap(
        visible_alias = "ls",
        about = "List workspaces. Global or on selected output."
    )]
    List(ListArgs),
    #[clap(hide = true)]
    Listen,
}

#[derive(Args, Debug, Clone)]
pub struct ListArgs {
    #[command(flatten)]
    pub output: Option<OutputSelector>,
    #[clap(long, conflicts_with = "output")]
    pub outputs_only: bool,
    #[clap(short, long)]
    pub json: bool
}

#[derive(Args, Debug, Clone)]
pub struct WorkspaceArgs {
    #[command(flatten)]
    pub workspace: WorkspaceSelector,
    #[command(flatten)]
    pub output: Option<OutputSelector>,
}

const WORKSPACE_SELECTION_HELP_HEADING: &str = "Workspace selection (mutually exclusive options)";
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct WorkspaceSelector {
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, requires = "output", help = "Requires output selection.")]
    pub active: bool,
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Workspaces are ordered by wayland protocol id. Global or on selected output.")]
    pub index: Option<usize>,
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Global or on selected output.")]
    pub name: Option<String>,
    #[clap(short, long, value_name = "ID", help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Wayland protocol id used in communication between server and client.")]
    pub protocol_id: Option<usize>,
    #[clap(short, long, value_delimiter = ',', num_args = 1.., value_name = "COORDS", help_heading = WORKSPACE_SELECTION_HELP_HEADING, requires = "output", help = "Coordinate space depends on compositor. Requires output selection.")]
    pub coordinates: Option<Vec<u8>>,
}

const OUTPUT_SELECTION_HELP_HEADING: &str = "Output selection (mutually exclusive options)";
#[derive(Args, Debug, Clone)]
#[group(id = "output", required = false, multiple = false)]
pub struct OutputSelector {
    #[clap(short = 'o', long, help_heading = OUTPUT_SELECTION_HELP_HEADING)]
    pub output_name: Option<String>,
    #[clap(short = 'u', long, value_name = "OUTPUT_ID", help_heading = OUTPUT_SELECTION_HELP_HEADING)]
    pub output_protocol_id: Option<usize>,
}

// same as OutputSelector, just needs a different name because assign command might require output selection twice
// TODO think of a better solution
const TARGET_OUTPUT_HELP_HEADING: &str = "Target output (mutually exclusive options)";
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct TargetOutput {
    #[clap(short = 't', long, help_heading = TARGET_OUTPUT_HELP_HEADING)]
    pub target_output_name: Option<String>,
    #[clap(short = 'r', long, value_name = "TARGET_ID", help_heading = TARGET_OUTPUT_HELP_HEADING)]
    pub target_output_protocol_id: Option<usize>,
}

impl TargetOutput {
    pub fn as_output_selection(&self) -> OutputSelector {
        OutputSelector{
            output_name: self.target_output_name.clone(),
            output_protocol_id: self.target_output_protocol_id,
        }
    }
}