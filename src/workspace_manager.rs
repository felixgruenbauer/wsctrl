use log::warn;
use smithay_client_toolkit::globals::GlobalData;
use wayland_client::WEnum;

use std::error::Error;
use std::fmt::Display;
use std::fmt::Write;

use crate::cli::{Cli, Commands, ListArgs, OutputSelector, WorkspaceSelector};
use crate::ext::workspace;
use crate::workspace_state::{
    GroupCapabilities, Workspace, WorkspaceCapabilities, WorkspaceEvent, WorkspaceGroup,
    WorkspaceHandler, WorkspaceStates,
};
use crate::workspace_state::{ManagerHandle, Protocol, WorkspaceState};
use crate::{delegate_workspace_cosmic_v1, delegate_workspace_ext_v0, delegate_workspace_ext_v1};
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
            setup(args).expect("Failed to setup wayland socket connection!");

        let mut workspace_manager = WorkspaceManager {
            registry_state,
            workspace_state,
            output_state,
        };
        events.roundtrip(&mut workspace_manager)?;
        match &args.command {
            Commands::List(args) => {
                workspace_manager.list_data(&args)?;
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
                workspace_manager.workspace_state.commit();
            }
            Commands::Deactivate(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace.deactivate();
                workspace_manager.workspace_state.commit();
            }
            Commands::Remove(args) => {
                let workspace = workspace_manager
                    .workspace_from_selection(&args.workspace, args.output.as_ref())?;
                workspace.remove();
                workspace.destroy();
                workspace_manager.workspace_state.commit();
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

fn setup(
    args: &Cli,
) -> Result<
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

    let (protocol, manager) = {
        if let Some(protocol) = &args.global_opts.protocol {
            match protocol {
                Protocol::ExtV0 => (
                    protocol,
                    ManagerHandle::ExtV0(
                        registry_state
                            .bind_one(&qh, 1..=1, GlobalData)
                            .expect("failed to bind 'ext_workspace_manager_v0'"),
                    ),
                ),
                Protocol::ExtV1 => (
                    protocol,
                    ManagerHandle::ExtV1(
                        registry_state
                            .bind_one(&qh, 1..=1, GlobalData)
                            .expect("failed to bind 'ext_workspace_manager_v1'"),
                    ),
                ),
                Protocol::CosmicV1 => (
                    protocol,
                    ManagerHandle::CosmicV1(
                        registry_state
                            .bind_one(&qh, 1..=1, GlobalData)
                            .expect("failed to bind 'zcosmic_workspace_manager_v1'"),
                    ),
                ),
            }
        } else {
            if let Ok(handle) = registry_state.bind_one(&qh, 1..=1, GlobalData) {
                (&Protocol::ExtV0, ManagerHandle::ExtV0(handle))
            } else if let Ok(handle) = registry_state.bind_one(&qh, 1..=1, GlobalData) {
                (&Protocol::ExtV1, ManagerHandle::ExtV1(handle))
            } else if let Ok(handle) = registry_state.bind_one(&qh, 1..=1, GlobalData) {
                (&Protocol::CosmicV1, ManagerHandle::CosmicV1(handle))
            } else {
                return Err(
                    format!("unable to bind any workspace management protocol version").into(),
                );
            }
        }
    };
    let workspace_state = WorkspaceState {
        groups: Vec::new(),
        workspaces: Vec::new(),
        manager,
        events: vec![],
        protocol: *protocol,
    };
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
                .filter(move |ws| ws.group.as_ref().is_some_and(|g| group.handle == *g))
                .collect::<Vec<_>>()
        } else {
            self.workspace_state.workspaces.iter().collect::<Vec<_>>()
        };
        if workspaces.len() == 0 {
            return Err(format!("No workspaces (on selected output)"));
        };
        if selector.active {
            return workspaces
                .iter()
                .find(|ws| ws.state.contains(WorkspaceStates::Active))
                .map_or(Err(format!("Unable to find active workspace!")), |ws| {
                    Ok(ws)
                });
        } else if let Some(index) = selector.index {
            workspaces.sort_unstable_by(|a, b| a.id().cmp(&b.id()));
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
                .find(|workspace| workspace.id() == protocol_id as u32)
                .map_or(
                    Err(format!(
                        "Unable to find workspace with protocol id {protocol_id}"
                    )),
                    |w| Ok(w),
                );
        } else if let Some(coordinates) = &selector.coordinates {
            let coords_len = workspaces.first().unwrap().coordinates.len();
            if coords_len != coordinates.len() {
                return Err(format!(
                    "Wrong coordinate length/number of axis. Expected {coords_len}, got {}",
                    coordinates.len()
                ));
            };
            return workspaces
                .iter()
                .find(|workspace| workspace.coordinates == *coordinates)
                .map_or(
                    Err(format!(
                        "Unable to find workspace with coordinates {coordinates:?}"
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

delegate_workspace_ext_v1!(WorkspaceManager);
delegate_workspace_ext_v0!(WorkspaceManager);
delegate_workspace_cosmic_v1!(WorkspaceManager);

impl WorkspaceHandler for WorkspaceManager {
    fn workspace_state(&self) -> &WorkspaceState {
        &self.workspace_state
    }
    fn workspace_state_mut(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }
}
delegate_registry!(WorkspaceManager);

impl ProvidesRegistryState for WorkspaceManager {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {
        OutputState
    }
}

impl WorkspaceManager {
    fn list_data(&mut self, args: &ListArgs) -> Result<(), String> {
        self.workspace_state.sort_workspaces_by_id();
        self.workspace_state.sort_workspaces_by_coords();
        self.workspace_state.sort_groups_by_id();

        if let Some(output) = &args.output {
            let group_filter = self.group_from_output(&output)?.handle.clone();
            self.workspace_state
                .workspaces
                .retain(|ws| ws.group.as_ref().is_some_and(|g| g == &group_filter));
            self.workspace_state
                .groups
                .retain(|g| g.handle == group_filter);
        };

        if args.json {
            match serde_json::to_string(&self.workspace_state) {
                Ok(json) => println!("{json}"),
                Err(e) => println!("{e}"),
            };
        } else {
            print!("{}", self.workspace_state);
        }
        Ok(())
    }
}
