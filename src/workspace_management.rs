use clap::{Args, Parser, Subcommand};
use log::{debug, warn};

use std::{collections::HashSet, error::Error};

use ext::workspace::v1::client::zext_workspace_handle_v1::{self};

use crate::workspace_state::{WorkspaceData, WorkspaceEvent, WorkspaceGroupData, WorkspaceHandler};
use crate::{delegate_workspace, ext, workspace_state::WorkspaceState};
use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{
    globals::registry_queue_init, protocol::wl_output, Connection, EventQueue, Proxy, QueueHandle,
};

#[derive(Parser, Debug)]
#[command(author = "fg", version = "0.1", about = "Manage workspaces via the wayland protocol extension 'ext-workspace-unstable-v1'.", long_about = None, arg_required_else_help = true)]
pub struct WorkspaceManagement {
    #[command(flatten)]
    global_opts: GlobalOpts,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct GlobalOpts {
    // todo
    //#[clap(long, global = true)]
    //json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
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
    List {
        #[command(flatten)]
        output: Option<OutputSelector>,
        #[clap(long, conflicts_with = "output")]
        outputs_only: bool,
    },
    #[clap(hide = true)]
    Listen,
}

#[derive(Args, Debug, Clone)]
struct WorkspaceArgs {
    #[command(flatten)]
    pub workspace: WorkspaceSelector,
    #[command(flatten)]
    pub output: Option<OutputSelector>,
}

const WORKSPACE_SELECTION_HELP_HEADING: &str = "Workspace selection (exclusive)";
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct WorkspaceSelector {
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, requires = "output", help = "Requires output selection.")]
    active: bool,
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Workspaces are ordered by wayland protocol id. Global or on selected output.")]
    index: Option<usize>,
    #[clap(short, long, help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Global or on selected output.")]
    name: Option<String>,
    #[clap(short, long, value_name = "ID", help_heading = WORKSPACE_SELECTION_HELP_HEADING, help = "Wayland protocol id used in communication between server and client.")]
    protocol_id: Option<usize>,
    //#[clap(short, long, value_name = "COORDS", help_heading = WORKSPACE_SELECTION_HELP_HEADING, requires = "output", help = "Coordinate space depends on compositor. Requires output selection.")]
    //coordinates: Option<String>,
}

const OUTPUT_SELECTION_HELP_HEADING: &str = "Output selection (exclusive)";
#[derive(Args, Debug, Clone)]
#[group(id = "output", required = false, multiple = false)]
struct OutputSelector {
    #[clap(short = 'o', long, help_heading = OUTPUT_SELECTION_HELP_HEADING)]
    output_name: Option<String>,
    #[clap(short = 'u', long, value_name = "OUTPUT_ID", help_heading = OUTPUT_SELECTION_HELP_HEADING)]
    output_protocol_id: Option<usize>,
}

impl WorkspaceManagement {
    pub fn exec(self) -> Result<(), Box<dyn Error>> {
        debug!("Cli arguments: {:?}", self);

        let (mut workspace_manager, mut events) =
            setup().expect("Unable to setup wayland socket connection!");

        events.roundtrip(&mut workspace_manager)?;
        match self.command {
            Commands::List {
                output,
                outputs_only,
            } => {
                let groups = if let Some(output) = output {
                    std::slice::from_ref(workspace_manager.group_from_output(&output)?)
                } else {
                    workspace_manager.workspace_state().workspace_groups()
                };
                print_data(groups, outputs_only);
                return Ok(());
            }
            Commands::Listen => loop {
                events.blocking_dispatch(&mut workspace_manager)?;
            },
            Commands::CreateWorkspace {
                workspace_name,
                output,
            } => {
                let group = workspace_manager.group_from_output(&output)?;
                workspace_manager
                    .workspace_state()
                    .create_workspace(&group.handle, Some(workspace_name));
            }
            Commands::Activate(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace_manager
                    .workspace_state()
                    .activate_workspace(&workspace.handle);
            }
            Commands::Deactivate(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace_manager
                    .workspace_state()
                    .deactivate_workspace(&workspace.handle);
            }
            Commands::Remove(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace_manager
                    .workspace_state()
                    .remove_workspace(&workspace.handle);
            }
        }
        events.roundtrip(&mut workspace_manager)?;
        Ok(())
    }
}

fn setup() -> Result<(WorkspaceManager, EventQueue<WorkspaceManager>), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;

    let (globals, events) = registry_queue_init(&conn)?;
    let qh: QueueHandle<WorkspaceManager> = events.handle();

    let registry_state = RegistryState::new(&globals);

    let output_state = OutputState::new(&globals, &qh);
    let workspace_state = WorkspaceState::new(&registry_state, &qh);

    let workspace_manager = WorkspaceManager {
        registry_state,
        workspace_state,
        output_state,
    };
    Ok((workspace_manager, events))
}

struct WorkspaceManager {
    registry_state: RegistryState,
    workspace_state: WorkspaceState,
    output_state: OutputState,
}

impl WorkspaceManager {
    pub fn workspace_from_selection(
        &self,
        selector: &WorkspaceSelector,
        output: Option<&OutputSelector>,
    ) -> Result<&WorkspaceData, String> {
        let groups = if let Some(output) = output {
            std::slice::from_ref(self.group_from_output(output)?)
        } else {
            &self.workspace_state().workspace_groups()
        };
        if selector.active {
            return groups.get(0).map_or(
                Err(format!(
                    "In order to select an active workspace, an output has to be selected!"
                )),
                |group| {
                    group
                        .workspaces
                        .iter()
                        .find(|ws| ws.states.contains(&zext_workspace_handle_v1::State::Active))
                        // todo add  output name to error
                        .map_or(Err(format!("Unable to find active workspace!")), |ws| {
                            Ok(ws)
                        })
                },
            );
        } else if let Some(index) = selector.index {
            // groups and workspaces are ordered by their protocol id
            return groups
                .iter()
                .flat_map(|group| group.workspaces.iter())
                .nth(index)
                .map_or(
                    Err(format!("Unable to find workspace with index {}", index)),
                    |w| Ok(w),
                );
        } else if let Some(name) = &selector.name {
            return groups
                .iter()
                .flat_map(|group| group.workspaces.iter())
                .find(|workspace| workspace.name.as_ref().map_or(false, |n| n == name))
                .map_or(
                    Err(format!("Unable to find workspace with name {name}")),
                    |w| Ok(w),
                );
        } else if let Some(protocol_id) = selector.protocol_id {
            return groups
                .iter()
                .flat_map(|group| group.workspaces.iter())
                .find(|workspace| workspace.handle.id().protocol_id() == protocol_id as u32)
                .map_or(
                    Err(format!(
                        "Unable to find workspace with protocol id {protocol_id}"
                    )),
                    |w| Ok(w),
                );
        }
        return Err("No workspace handle for provided selector found!".to_string());
    }

    pub fn group_from_output(
        &self,
        output: &OutputSelector,
    ) -> Result<&WorkspaceGroupData, String> {
        if let Some(name) = &output.output_name {
            return self
                .workspace_state()
                .workspace_groups()
                .iter()
                .find(|group| group.get_output_name().map_or(false, |n| &n == name))
                .map_or(
                    Err(format!("Unable to find output with name {}!", name)),
                    |g| Ok(g),
                );
        } else if let Some(protocol_id) = output.output_protocol_id {
            return self
                .workspace_state()
                .workspace_groups()
                .iter()
                .find(|group| {
                    group.output.as_ref().map_or(false, |output| {
                        output.id().protocol_id() == protocol_id as u32
                    })
                })
                .map_or(
                    Err(format!(
                        "Unable to find output with protocol id {}!",
                        protocol_id
                    )),
                    |g| Ok(g),
                );
        } else {
            return Err(format!("No output/group found for provided selection!"));
        }
    }
}

impl OutputHandler for WorkspaceManager {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

delegate_output!(WorkspaceManager);

impl WorkspaceHandler for WorkspaceManager {
    fn workspace_state(&self) -> &WorkspaceState {
        &self.workspace_state
    }
    fn workspace_state_mut(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }
    fn handle_events(&mut self, events: Vec<WorkspaceEvent>) {
        for event in events.into_iter() {
            match event {
                WorkspaceEvent::WorkspaceGroupCreated(group_handle) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .push(WorkspaceGroupData {
                            handle: group_handle,
                            output: None,
                            workspaces: Vec::new(),
                        });
                }
                WorkspaceEvent::WorkspaceGroupRemoved(group_handle) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .retain(|group| group.handle != group_handle);
                }
                WorkspaceEvent::WorkspaceCreated(group_handle, workspace_handle) => self
                    .workspace_state_mut()
                    .workspace_groups_mut()
                    .iter_mut()
                    .find(|group| group.handle == group_handle)
                    .map_or(warn!("Workspace created in non-existent group!"), |group| {
                        let workspace = WorkspaceData {
                            handle: workspace_handle,
                            name: None,
                            coordinates: None,
                            id: None,
                            states: HashSet::new(),
                        };
                        group.workspaces.push(workspace);
                    }),
                WorkspaceEvent::WorkspaceRemoved(workspace_handle) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .for_each(|group| {
                            group
                                .workspaces
                                .retain(|workspace| workspace.handle != workspace_handle)
                        });
                }
                WorkspaceEvent::OutputEnter(group_handle, output) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .find(|group| group.handle == group_handle)
                        .map_or_else(
                            || warn!("output_enter event for unknown workspace group handle"),
                            |group| group.output = Some(output),
                        );
                }
                WorkspaceEvent::OutputLeave(group_handle, output) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .find(|group| group.handle == group_handle)
                        .map_or_else(
                            || warn!("output_leave event for unknown workspace group handle"),
                            |group| {
                                if group.output.as_ref().is_some_and(|o| o == &output) {
                                    group.output = None
                                }
                            },
                        );
                }
                WorkspaceEvent::WorkspaceState(workspace_handle, state) => {
                    let mut states = HashSet::new();
                    if state.get(0).is_some_and(|s| *s == 0) {
                        states.insert(zext_workspace_handle_v1::State::Active);
                    };
                    if state.get(1).is_some_and(|s| *s == 1) {
                        states.insert(zext_workspace_handle_v1::State::Urgent);
                    };
                    if state.get(2).is_some_and(|s| *s == 2) {
                        states.insert(zext_workspace_handle_v1::State::Hidden);
                    };
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .find_map(|group| {
                            group
                                .workspaces
                                .iter_mut()
                                .find(|workspace| workspace.handle == workspace_handle)
                        })
                        .map_or_else(
                            || warn!("State event for unknown workspace handle!"),
                            |workspace| workspace.states = states,
                        )
                }
                WorkspaceEvent::WorkspaceName(workspace_handle, name) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .find_map(|group| {
                            group
                                .workspaces
                                .iter_mut()
                                .find(|workspace| workspace.handle == workspace_handle)
                        })
                        .map_or_else(
                            || warn!("name event for unknown workspace handle!"),
                            |workspace| workspace.name = Some(name),
                        );
                }
                WorkspaceEvent::WorkspaceCoord(workspace_handle, coordinates) => {
                    self.workspace_state_mut()
                        .workspace_groups_mut()
                        .iter_mut()
                        .find_map(|group| {
                            group
                                .workspaces
                                .iter_mut()
                                .find(|workspace| workspace.handle == workspace_handle)
                        })
                        .map_or_else(
                            || warn!("coordinates event for unknown workspace handle!"),
                            |workspace| workspace.coordinates = Some(coordinates),
                        );
                }
            }
        }
    }
}

delegate_workspace!(WorkspaceManager);
delegate_registry!(WorkspaceManager);

impl ProvidesRegistryState for WorkspaceManager {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {
        WorkspaceState,
        OutputState
    }
}

fn print_data(groups: &[WorkspaceGroupData], outputs_only: bool) {
    if groups.is_empty() {
        return;
    }
    let mut workspace_indent = "";
    if groups.len() != 1 {
        //    println!("// output: name description location globalId protocolId groupId");
        workspace_indent = "    ";
    }
    //if !outputs_only {
    //    println!("//{workspace_indent} workspace: # name states coords protocolId");
    //}

    for (idx, group) in groups.iter().enumerate() {
        if idx == 0 && groups.len() != 1 {
            println!("# Name Description Location GlobalId ProtocolId GroupId");
        }
        if groups.len() != 1 {
            println!("{idx} {}", format_group_data(group));
        }
        if idx == 0 {
            println!("{workspace_indent}# Name States Coords ProtocolId");
        }
        if !outputs_only {
            for (idx, workspace) in group.workspaces.iter().enumerate() {
                println!("{workspace_indent}{idx} {}", format_workspace(workspace))
            }
        }
    }
}

fn format_group_data(group: &WorkspaceGroupData) -> String {
    let output_out = format!(
        "{} {:?} {} {} {} {}",
        group.get_output_name().unwrap_or("MissingName".to_string()),
        group
            .get_output_info()
            .map_or("MissingInfo".to_string(), |info| info
                .description
                .unwrap_or("MissingDescription".to_string())),
        group
            .get_output_info()
            .map_or("MissingInfo".to_string(), |info| format!(
                "({}, {})",
                info.location.0, info.location.1
            )),
        group
            .get_output_info()
            .map_or("MissingInfo".to_string(), |info| format!(
                "{:?}",
                // corresponds to the global `name` of the wl_output
                info.id
            )),
        group
            .output
            .as_ref()
            .map_or("MissingOutputId".to_string(), |output| format!(
                "{}",
                output.id().protocol_id()
            )),
        group.handle.id().protocol_id()
    );
    output_out
}

fn format_workspace(workspace: &WorkspaceData) -> String {
    let workspace_out = format!(
        "{:?} {:?} {:?} {}",
        workspace.name,
        workspace.states,
        workspace.coordinates,
        workspace.handle.id().protocol_id()
    );
    workspace_out
}
