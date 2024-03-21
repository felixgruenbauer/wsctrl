mod ext;
mod workspace_state;

use std::{collections::HashSet, error::Error};

use clap::{Parser, Subcommand};
use ext::workspace::v1::client::{zext_workspace_group_handle_v1::ZextWorkspaceGroupHandleV1, zext_workspace_handle_v1};

use smithay_client_toolkit::{
    delegate_registry,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{globals::registry_queue_init, Connection, EventQueue, Proxy, QueueHandle};
use workspace_state::{WorkspaceData, WorkspaceEvent, WorkspaceGroupData, WorkspaceHandler};
use crate::workspace_state::WorkspaceState;


fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();
    
    let (mut list_workspaces, mut events) = setup().expect("Unable to setup wayland socket connection!");

    match args.command {
        Some(Commands::List) => {
            events.roundtrip(&mut list_workspaces)?;
            for (handle, data) in list_workspaces.workspace_state.workspace_groups() {
                print_workspace_group(handle, data);
            }
        },
        Some(Commands::Listen) => {
            loop {
                events.blocking_dispatch(&mut list_workspaces)?;
            }
        },
        Some(Commands::Activate { workspace }) => {
            events.roundtrip(&mut list_workspaces)?;
            list_workspaces.workspace_state.activate_workspace(workspace)?;
            events.roundtrip(&mut list_workspaces)?;
        },
        Some(Commands::Deactivate { workspace }) => {
            events.roundtrip(&mut list_workspaces)?;
            list_workspaces.workspace_state.deactivate_workspace(workspace)?;
            events.roundtrip(&mut list_workspaces)?;
        },
        Some(Commands::CreateWorkspace { workspace_group , workspace_name}) => {
            events.roundtrip(&mut list_workspaces)?;
            list_workspaces.workspace_state.create_workspace(workspace_group, workspace_name)?;
            
        },
        Some(Commands::Destroy { workspace }) => {
            events.roundtrip(&mut list_workspaces)?;
            list_workspaces.workspace_state.destroy_workspace(workspace)?;
            events.roundtrip(&mut list_workspaces)?;
        },
        Some(Commands::Remove { workspace }) => {
            events.roundtrip(&mut list_workspaces)?;
            list_workspaces.workspace_state.remove_workspace(workspace)?;
            events.roundtrip(&mut list_workspaces)?;
        },
        None => {}
    }
    Ok(())
}
    
fn setup() -> Result<(WorkspaceManager, EventQueue<WorkspaceManager>), Box<dyn Error>> {
    // Try to connect to the Wayland server.
    let conn = Connection::connect_to_env()?;

    // Now create an event queue and a handle to the queue so we can create objects.
    let (globals, events) = registry_queue_init(&conn)?;
    let qh: QueueHandle<WorkspaceManager> = events.handle();

    // Initialize the registry handling so other parts of Smithay's client toolkit may bind
    // globals.
    let registry_state = RegistryState::new(&globals);

    // Initialize the delegate we will use for outputs.
    let workspace_delegate = WorkspaceState::new(&registry_state, &qh);

    // Set up application state.
    //
    // This is where you will store your delegates and any data you wish to access/mutate while the
    // application is running.
    let workspace_manager = WorkspaceManager { registry_state, workspace_state: workspace_delegate };
    Ok((workspace_manager, events))
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Activate { workspace: usize},
    Deactivate { workspace: usize},
    Destroy { workspace: usize}, // TODO destroy object: either workspace or group
    Remove { workspace: usize},
    CreateWorkspace { workspace_group: usize, workspace_name: Option<String>},
    List,
    Listen,
}
/// Application data.
///
/// This type is where the delegates for some parts of the protocol and any application specific data will
/// live.
struct WorkspaceManager {
    registry_state: RegistryState,
    workspace_state: WorkspaceState,
}


impl WorkspaceHandler for WorkspaceManager {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }
    
    fn handle_events(&mut self, events: Vec<WorkspaceEvent>) {
        for event in events.into_iter() {
            match event {
                WorkspaceEvent::WorkspaceGroupCreated(workspace_group) => {
                    self.workspace_state.workspace_groups().insert(workspace_group.clone(), WorkspaceGroupData::default());
                },
                WorkspaceEvent::WorkspaceGroupRemoved(workspace_group) => {
                    self.workspace_state.workspace_groups().remove(&workspace_group);
                },
                WorkspaceEvent::WorkspaceCreated(workspace_group, workspace) => {
                    self.workspace_state.workspace_groups()
                        .entry(workspace_group)
                        .and_modify(|e| {e.workspaces.insert(workspace, WorkspaceData::default());});
                },
                WorkspaceEvent::WorkspaceRemoved(workspace) => {
                    self.workspace_state.workspace_groups().iter_mut()
                        .for_each(|(_, data)| {data.workspaces.remove(&workspace);});
                },
                WorkspaceEvent::OutputEnter(workspace_group, output) => {
                    self.workspace_state.workspace_groups().entry(workspace_group).and_modify(|e| {e.outputs.push(output);});
                },
                WorkspaceEvent::OutputLeave(workspace_group, output) => {
                    self.workspace_state.workspace_groups().entry(workspace_group).and_modify(|e| {e.outputs.retain(|o| *o == output);});
                },
                WorkspaceEvent::WorkspaceState(workspace, state) => {
                    let mut states = HashSet::new(); 
                    if state.get(0).is_some_and(|s| *s == 0) {states.insert(zext_workspace_handle_v1::State::Active);};
                    if state.get(1).is_some_and(|s| *s == 1) {states.insert(zext_workspace_handle_v1::State::Urgent);};
                    if state.get(2).is_some_and(|s| *s == 2) {states.insert(zext_workspace_handle_v1::State::Hidden);};
                    if let Some(data) = self.workspace_state.workspace_groups().iter_mut().find_map(|(_, d)| d.workspaces.get_mut(&workspace)) {
                        data.states = states

                    }
                },
                WorkspaceEvent::WorkspaceName(workspace, name) => {
                    if let Some(data) = self.workspace_state.workspace_groups().iter_mut().find_map(|(_, d)| d.workspaces.get_mut(&workspace)) {
                        data.name = Some(name);                 
                    }
                },
                WorkspaceEvent::WorkspaceCoord(workspace, coordinates) => {
                    if let Some(data) = self.workspace_state.workspace_groups().iter_mut().find_map(|(_, d)| d.workspaces.get_mut(&workspace)) {
                        data.coordinates = Some(coordinates);                 
                    }
                },
            }
        }
    }
}

// Now we need to say we are delegating the responsibility of output related events for our application data
// type to the requisite delegate.
delegate_workspace!(WorkspaceManager);
// In order for our delegate to know of the existence of globals, we need to implement registry
// handling for the program. This trait will forward events to the RegistryHandler trait
// implementations.
delegate_registry!(WorkspaceManager);

// In order for delegate_registry to work, our application data type needs to provide a way for the
// implementation to access the registry state.
//
// We also need to indicate which delegates will get told about globals being created. We specify
// the types of the delegates inside the array.
impl ProvidesRegistryState for WorkspaceManager {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {
        // Here we specify that OutputState needs to receive events regarding the creation and destruction of
        // globals.
        WorkspaceState,
    }
}

fn print_workspace_group(handle: &ZextWorkspaceGroupHandleV1, data: &WorkspaceGroupData) {
    let id = handle.id();
    println!("Group ID {} (Protocol: {} Version: {})", id.protocol_id(), id.interface().name, id.interface().version);
    println!("  Outputs: {:?}", data.outputs);
    println!("  Workspaces: ");
    for (workspace_handle, workspace_data) in data.workspaces.iter() {
        println!("      {} {:?}", workspace_handle.id().protocol_id(), workspace_data);
    }
}