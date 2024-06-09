use log::warn;

use std::fmt::Display;
use std::{collections::HashSet, error::Error};

use crate::cli::{Cli, Commands, ListArgs, OutputSelector, WorkspaceSelector};
use crate::workspace_state::{State, Workspace, WorkspaceEvent, WorkspaceGroup, WorkspaceHandler};
use crate::{delegate_workspace, workspace_state::WorkspaceState};
use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{
    globals::registry_queue_init, protocol::wl_output, Connection, EventQueue, Proxy, QueueHandle,
};
impl WorkspaceManager {
    pub fn exec(args: &Cli) -> Result<(), Box<dyn Error>> {
        let (registry_state, workspace_state, output_state, mut events) =
            setup().expect("Failed to setup wayland socket connection!");

        let mut workspace_manager = WorkspaceManager {
            registry_state,
            workspace_state,
            output_state,
        };
        events.roundtrip(&mut workspace_manager)?;
        match &args.command {
            Commands::List(args) => {
                workspace_manager.print_data(&args)?;
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
                group.create_workspace(workspace_name.to_string())
            }
            Commands::Activate(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace.activate();
                workspace_manager.workspace_state().commit();
            }
            Commands::Deactivate(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace.deactivate();
                workspace_manager.workspace_state().commit();
            }
            Commands::Remove(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace.remove();
                workspace.destroy();
                workspace_manager.workspace_state().commit();
            }
            Commands::Assign {
                workspace_args,
                target,
            } => {
                let workspace = workspace_manager.workspace_from_selection(
                    &workspace_args.workspace,
                    workspace_args.output.as_ref(),
                )?;
                let group = workspace_manager.group_from_output(&target.as_output_selection())?;
                workspace.assign(&group.handle)?;
                workspace_manager.workspace_state.commit();
            }
        }
        events.roundtrip(&mut workspace_manager)?;
        Ok(())
    }
}

fn setup() -> Result<
    (
        RegistryState,
        WorkspaceState,
        OutputState,
        EventQueue<WorkspaceManager>,
    ),
    Box<dyn Error>,
> {
    let conn = Connection::connect_to_env()?;

    let (globals, events) = registry_queue_init(&conn)?;
    let qh: QueueHandle<WorkspaceManager> = events.handle();

    let registry_state = RegistryState::new(&globals);

    let output_state = OutputState::new(&globals, &qh);
    let workspace_state = WorkspaceState::new(&registry_state, &qh)?;

    Ok((registry_state, workspace_state, output_state, events))
}

pub struct WorkspaceManager {
    registry_state: RegistryState,
    workspace_state: WorkspaceState,
    output_state: OutputState,
}

impl WorkspaceManager {
    pub fn workspace_from_selection(
        &self,
        selector: &WorkspaceSelector,
        output: Option<&OutputSelector>,
    ) -> Result<&Workspace, String> {
        let mut workspaces = if let Some(output) = output {
            let group = self.group_from_output(output)?;
            self.workspace_state
                .workspaces
                .iter()
                .filter(move |ws| ws.group.as_ref().is_some_and(|g| group.handle.id() == *g))
                .collect::<Vec<_>>()
        } else {
            self.workspace_state.workspaces.iter().collect::<Vec<_>>()
        };
        if selector.active {
            return workspaces
                .iter()
                .find(|ws| ws.state.contains(&State::Active))
                .map_or(Err(format!("Unable to find active workspace!")), |ws| {
                    Ok(ws)
                });
        } else if let Some(index) = selector.index {
            workspaces.sort_unstable_by(|a, b| {
                a.handle
                    .id()
                    .protocol_id()
                    .cmp(&b.handle.id().protocol_id())
            });
            return workspaces.get(index).map_or(
                Err(format!("Unable to find workspace with index {}", index)),
                |w| Ok(w),
            );
        } else if let Some(name) = &selector.name {
            return workspaces
                .iter()
                .find(|workspace| workspace.name.as_ref().is_some_and(|n| n == name))
                .map_or(
                    Err(format!("Unable to find workspace with name {name}")),
                    |w| Ok(w),
                );
        } else if let Some(protocol_id) = selector.protocol_id {
            return workspaces
                .iter()
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

    pub fn group_from_output(&self, output: &OutputSelector) -> Result<&WorkspaceGroup, String> {
        let groups = &self.workspace_state.groups;
        if let Some(name) = &output.output_name {
            return groups
                .iter()
                .find(|group| group.get_output_name().map_or(false, |n| &n == name))
                .map_or(
                    Err(format!("Unable to find output with name {}!", name)),
                    |g| Ok(g),
                );
        } else if let Some(protocol_id) = output.output_protocol_id {
            return groups
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
                    self.workspace_state.groups.push(WorkspaceGroup {
                        handle: group_handle,
                        output: None,
                    });
                }
                WorkspaceEvent::WorkspaceGroupRemoved(group_handle) => {
                    self.workspace_state
                        .groups
                        .retain(|group| group.handle != group_handle);
                }
                WorkspaceEvent::WorkspaceCreated(group_handle, workspace_handle) => {
                    self.workspace_state.workspaces.push(Workspace {
                        handle: workspace_handle,
                        name: None,
                        coordinates: None,
                        state: HashSet::new(),
                        group: group_handle.map_or(None, |g| Some(g.id())),
                    })
                }
                WorkspaceEvent::WorkspaceRemoved(workspace_handle) => self
                    .workspace_state
                    .workspaces
                    .retain(|workspace| workspace.handle != workspace_handle),
                WorkspaceEvent::OutputEnter(group_handle, output) => {
                    self.workspace_state
                        .groups
                        .iter_mut()
                        .find(|group| group.handle == group_handle)
                        .map_or_else(
                            || warn!("output_enter event for unknown workspace group handle"),
                            |group| group.output = Some(output),
                        );
                }
                WorkspaceEvent::OutputLeave(group_handle, output) => {
                    self.workspace_state
                        .groups
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
                WorkspaceEvent::WorkspaceState(workspace_handle, state) => self
                    .workspace_state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.handle == workspace_handle)
                    .map_or_else(
                        || warn!("State event for unknown workspace handle!"),
                        |workspace| workspace.state = state,
                    ),
                WorkspaceEvent::WorkspaceName(workspace_handle, name) => {
                    self.workspace_state
                        .workspaces
                        .iter_mut()
                        .find(|workspace| workspace.handle == workspace_handle)
                        .map_or_else(
                            || warn!("name event for unknown workspace handle!"),
                            |workspace| workspace.name = Some(name),
                        );
                }
                WorkspaceEvent::WorkspaceCoord(workspace_handle, coordinates) => {
                    self.workspace_state
                        .workspaces
                        .iter_mut()
                        .find(|workspace| workspace.handle == workspace_handle)
                        .map_or_else(
                            || warn!("coordinates event for unknown workspace handle!"),
                            |workspace| workspace.coordinates = Some(coordinates),
                        );
                }
                WorkspaceEvent::WorkspaceGroupCapabilities(caps) => {
                    self.workspace_state.group_cap = Some(caps);
                }
                WorkspaceEvent::WorkspaceEnter(workspace, group) => {
                    self.workspace_state
                        .workspaces
                        .iter_mut()
                        .find(|ws| ws.handle == workspace)
                        .map_or_else(
                            || warn!("workspace_enter event for unknown workspace"),
                            |ws| ws.group = Some(group.id()),
                        );
                }
                WorkspaceEvent::WorkspaceLeave(workspace, group) => {
                    self.workspace_state
                        .workspaces
                        .iter_mut()
                        .find(|ws| ws.handle == workspace)
                        .map_or_else(
                            || warn!("workspace_leave event for unknown workspace"),
                            |ws| {
                                if ws.group.as_ref().is_some_and(|g| g == &group.id()) {
                                    ws.group = None;
                                } else {
                                    warn!("workspace_leave event for unassigned group");
                                }
                            },
                        );
                }
                WorkspaceEvent::WorkspaceCapabilities(caps) => {
                    self.workspace_state.workspace_cap = Some(caps)
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

impl WorkspaceManager {
    fn sort_workspaces_by_id(&mut self) {
        self.workspace_state.workspaces.sort_unstable_by(|a, b| {
            a.handle
                .id()
                .protocol_id()
                .cmp(&b.handle.id().protocol_id())
        });
    }

    fn sort_groups_by_id(&mut self) {
        self.workspace_state.groups.sort_unstable_by(|a, b| {
            a.handle
                .id()
                .protocol_id()
                .cmp(&b.handle.id().protocol_id())
        });
    }

    fn print_data(&mut self, args: &ListArgs) -> Result<(), String> {

        self.sort_workspaces_by_id();
        self.sort_groups_by_id();

        if let Some(output) = &args.output {
            let group_filter = self.group_from_output(&output)?.handle.id();
            self.workspace_state.workspaces.retain(|ws| ws.group.as_ref().is_some_and(|g| g == &group_filter));
            self.workspace_state.groups.retain(|g| g.handle.id() == group_filter);
        };

        if args.json {
            match serde_json::to_string(&self.workspace_state) {
                Ok(json) => println!("{json}"),
                Err(e) => println!("{e}"),
            }
            return Ok(())
        }
        let header = concat!(
            "// output: # groupId globalId name location protId description //\n",
            "// workspace: # name states coordinates protId                 //"
        );
        let workspace_indent = "    ";

        let mut workspace_idx = 0u32;



        let out = self
            .workspace_state
            .groups
            .iter()
            .enumerate()
            .fold("".to_string(), |acc, (group_idx, group)| {
                let workspaces = self.workspace_state
                    .workspaces
                    .iter()
                    .filter(|ws| ws.group.as_ref().is_some_and(|g| g == &group.handle.id()))
                    .fold("".to_string(), |acc, workspace| {
                        let out = format!(
                            "{acc}\n{workspace_indent}{workspace_idx} {}",
                            format_workspace(workspace)
                        );
                        workspace_idx += 1;
                        out
                    });
                format!("{acc}\n{group_idx} {group}{workspaces}")
            });
            
        let unassigned = self.workspace_state.workspaces.iter().filter(|ws| ws.group.is_none()).fold("".to_string(), |acc, workspace| {
            let out = format!("{}\n{workspace_indent}{workspace_idx} {}",
                if acc.is_empty() {"\n- unassigned workspaces"} else {""},
                format_workspace(workspace)
            );
            workspace_idx += 1;
            out
        });
        
        println!("{header}{out}{unassigned}");

        Ok(())
    }
}

impl Display for WorkspaceGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.output.is_none() {
            write!(
                f,
                "{}: no output assigned to group",
                self.handle.id().protocol_id()
            )
        } else {
            let info = &self.get_output_info();
            write!(
                f,
                "{} {} {} {} {} {}",
                self.handle.id().protocol_id(),
                // corresponds to the global `name` of the wl_output
                info.as_ref()
                    .map_or("--".to_string(), |info| info.id.to_string()),
                info.as_ref().map_or("--".to_string(), |info| info
                    .name
                    .clone()
                    .unwrap_or("--".to_string())),
                info.as_ref()
                    .map_or("--".to_string(), |info| format!("{:?}", info.location)),
                self.output.as_ref().map_or("--".to_string(), |o| o.id().protocol_id().to_string()),
                info.as_ref().map_or("--".to_string(), |info| info
                    .description
                    .clone()
                    .unwrap_or("--".to_string())),
            )
        }
    }
}

fn format_workspace(workspace: &Workspace) -> String {
    let workspace_out = format!(
        "{} {} {} {}",
        workspace.name.as_ref().unwrap_or(&"--".to_string()),
        if workspace.state.is_empty() {
            "--".to_string()
        } else {
            workspace
                .state
                .iter()
                .fold("".to_string(), |acc, s| acc + &format!("{s:?}"))
        },
        workspace
            .coordinates
            .as_ref()
            .map_or("--".to_string(), |coords| format!("{coords:?}")),
        workspace.handle.id().protocol_id()
    );
    workspace_out
}
