use std::collections::{HashMap, HashSet};
use log::{debug, info};
use smithay_client_toolkit::{output::OutputData, reexports::client::protocol::wl_output::WlOutput};

use crate::ext::workspace::v1::client::{
    zext_workspace_group_handle_v1::{self, ZextWorkspaceGroupHandleV1}, zext_workspace_handle_v1::{self, ZextWorkspaceHandleV1}, zext_workspace_manager_v1::{self, ZextWorkspaceManagerV1}
    //zext_workspace_group_handle_v1::ZextWorkspaceGroupCapabilitiesV1 as GroupCapabilities,
    //zext_workspace_handle_v1::ZextWorkspaceCapabilitiesV1 as WorkspaceCapabilities,
};


use smithay_client_toolkit::{
    globals::GlobalData, reexports::client::
        Dispatch

    , registry::{ProvidesRegistryState, RegistryHandler, RegistryState}};
use wayland_client::{Connection, Proxy, QueueHandle};

#[derive(Default, Debug)]
pub struct WorkspaceGroupData {
    //pub capabilities: Option<Vec<GroupCapabilities>>,
    pub outputs: Vec<WlOutput>,
    pub workspaces: HashMap<ZextWorkspaceHandleV1, WorkspaceData>,
}


#[derive(Default, Debug)]
pub struct WorkspaceData {
    pub name: Option<String>,
    //pub capabilities: Option<Vec<WorkspaceCapabilities>>,
    pub coordinates: Option<Vec<u8>>,
    pub id: Option<usize>,
    pub states: HashSet<zext_workspace_handle_v1::State>,
}

pub struct WorkspaceState {
    workspace_groups: HashMap<ZextWorkspaceGroupHandleV1, WorkspaceGroupData>,    
    manager: ZextWorkspaceManagerV1,
    events: Vec<WorkspaceEvent>
}

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    WorkspaceGroupCreated(ZextWorkspaceGroupHandleV1),
    WorkspaceGroupRemoved(ZextWorkspaceGroupHandleV1),
    OutputEnter(ZextWorkspaceGroupHandleV1, WlOutput),
    OutputLeave(ZextWorkspaceGroupHandleV1, WlOutput),
    WorkspaceCreated(ZextWorkspaceGroupHandleV1, ZextWorkspaceHandleV1),
    WorkspaceRemoved(ZextWorkspaceHandleV1),
    WorkspaceState(ZextWorkspaceHandleV1, Vec<u8>),
    WorkspaceCoord(ZextWorkspaceHandleV1, Vec<u8>),
    WorkspaceName(ZextWorkspaceHandleV1, String),

}

pub trait WorkspaceHandler {
    fn workspace_state(&mut self) -> &mut WorkspaceState;
    fn handle_events(&mut self, events: Vec<WorkspaceEvent>);
}

impl WorkspaceState {
    pub fn new<D>
    (
        registry_state: &RegistryState,
        qh: &QueueHandle<D>,
    ) -> Self
    where
        D: Dispatch<ZextWorkspaceHandleV1, ()>
            + Dispatch<ZextWorkspaceGroupHandleV1, ()>
            + Dispatch<ZextWorkspaceManagerV1, GlobalData>
            + 'static
    {
        let manager = registry_state.bind_one(qh, 1..=1, GlobalData)
            .expect("Unable to bind ext_workspace_unstable_v1 global object! Does the compositor support the ext_workspace protocol?");
        WorkspaceState { workspace_groups: HashMap::new(), manager, events: vec![]}
    }
    
    pub fn workspace_groups(&mut self) -> &mut HashMap<ZextWorkspaceGroupHandleV1, WorkspaceGroupData> {
        &mut self.workspace_groups
    }

    pub fn activate_workspace(&mut self, workspace_id: usize) -> Result<(), String>{
        debug!("sending activate request for {}", workspace_id);
        let (handle, _) = self.find_workspace(workspace_id)?;
        handle.activate();  
        self.manager.commit();
        Ok(())
    }
    
    fn find_workspace(&self, workspace_id: usize) -> Result<(&ZextWorkspaceHandleV1, &WorkspaceData), String> {
        self.workspace_groups.iter()
            .find_map(|(_, d)| d.workspaces.iter().find(|(h, _)| h.id().protocol_id() as usize == workspace_id))
            .map_or(Err(format!("Unable to find workspace with id {workspace_id}")), |w| Ok(w))
    }

    fn find_workspace_group(&self, workspace_group_id: usize) -> Result<(&ZextWorkspaceGroupHandleV1, &WorkspaceGroupData), String> {
        self.workspace_groups.iter()
            .find(|(handle, _)| handle.id().protocol_id() as usize == workspace_group_id)
            .map_or(Err(format!("Unable to find workspace group with id {workspace_group_id}")), |w| Ok(w))
    }
    
    pub fn deactivate_workspace(&mut self, workspace_id: usize) -> Result<(), String> {
        debug!("sending deactivate request for {}", workspace_id);
        let (handle, _) = self.find_workspace(workspace_id)?;
        handle.deactivate();  
        self.manager.commit();
        Ok(())
    }
    pub fn remove_workspace(&mut self, workspace_id: usize) -> Result<(), String> {
        debug!("sending remove request for {}", workspace_id);
        let (handle, _) = self.find_workspace(workspace_id)?;
        handle.remove();  
        self.manager.commit();
        Ok(())
    }
 
    pub fn destroy_workspace(&mut self, workspace_id: usize) -> Result<(), String> {
        debug!("sending destroy request for {}", workspace_id);
        let (handle, _) = self.find_workspace(workspace_id)?;
        handle.destroy();  
        self.manager.commit();
        Ok(())
    }
    
    pub fn create_workspace(&mut self, workspace_group: usize, workspace_name: Option<String>) -> Result<(), String> {
        debug!("sending request to create workspace in group {}", workspace_group);
        let (handle, _) = self.find_workspace_group(workspace_group)?;
        handle.create_workspace(workspace_name.unwrap_or(String::from("")));
        self.manager.commit();
        Ok(())
    }
}

impl<D> Dispatch<ZextWorkspaceManagerV1, GlobalData, D> for WorkspaceState 
where
    D: Dispatch<ZextWorkspaceGroupHandleV1, ()>
    + Dispatch<ZextWorkspaceManagerV1, GlobalData>
    + WorkspaceHandler
    + ProvidesRegistryState
    + 'static,
{
    fn event(
            state: &mut D,
            _proxy: &ZextWorkspaceManagerV1,
            event: <ZextWorkspaceManagerV1 as wayland_client::Proxy>::Event,
            _data: &GlobalData,
            _conn: &wayland_client::Connection,
            _qhandle: &wayland_client::QueueHandle<D>,
        ) {
        match event {
            zext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                info!("received workspace_group event with id {}", workspace_group.id().protocol_id());
                state.workspace_state().events.push(WorkspaceEvent::WorkspaceGroupCreated(workspace_group));
            },
            zext_workspace_manager_v1::Event::Done {  } => {
                info!("received done event");
                let events = state.workspace_state().events.drain(..).collect();
                state.handle_events(events);
            },
            zext_workspace_manager_v1::Event::Finished {} => {
                info!("received manager finished event");
            },
        }
    }
    
    wayland_client::event_created_child!(D, ZextWorkspaceManagerV1, [
        0 => (ZextWorkspaceGroupHandleV1, ()),
    ]);
}


impl<D> Dispatch<ZextWorkspaceGroupHandleV1, (), D> for WorkspaceState 
where
    D: Dispatch<ZextWorkspaceGroupHandleV1, ()>
        + Dispatch<ZextWorkspaceHandleV1, ()>
        + WorkspaceHandler
        + 'static,
{
    fn event(
            state: &mut D,
            proxy: &ZextWorkspaceGroupHandleV1,
            event: <ZextWorkspaceGroupHandleV1 as wayland_client::Proxy>::Event,
            _data: &(),
            _conn: &wayland_client::Connection,
            _qhandle: &wayland_client::QueueHandle<D>,
    ) 
    {
        match event {
            zext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                info!("recieved output_enter event (workspace_group id: {}, output: {:?}", proxy.id().protocol_id(), output.data::<OutputData>().unwrap());
                state.workspace_state().events.push(WorkspaceEvent::OutputEnter(proxy.clone(), output));
            },
            zext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                info!("recieved output_leave event (workspace_group id: {}, output: {:?}", proxy.id().protocol_id(), output.data::<OutputData>().unwrap());
                state.workspace_state().events.push(WorkspaceEvent::OutputLeave(proxy.clone(), output));
            },
            zext_workspace_group_handle_v1::Event::Remove => {
                info!("received workspace_group_remove event for id {}", proxy.id().protocol_id());
                state.workspace_state().events.push(WorkspaceEvent::WorkspaceGroupRemoved(proxy.clone()));
            },
            zext_workspace_group_handle_v1::Event::Workspace { workspace } => {
                info!("received workspace event with id {}", workspace.id().protocol_id());
                state.workspace_state().events.push(WorkspaceEvent::WorkspaceCreated(proxy.clone(), workspace));
            },
        };
    }
    wayland_client::event_created_child!(D, ZextWorkspaceGroupHandleV1, [
        2 => (ZextWorkspaceHandleV1, ()),
    ]);

}

impl<D> Dispatch<ZextWorkspaceHandleV1, (), D> for WorkspaceState 
where
    D: Dispatch<ZextWorkspaceHandleV1, ()>
        + ProvidesRegistryState
        + WorkspaceHandler
        + 'static,
{
    fn event(
            state: &mut D,
            proxy: &ZextWorkspaceHandleV1,
            event: <ZextWorkspaceHandleV1 as wayland_client::Proxy>::Event,
            _data: &(),
            _conn: &wayland_client::Connection,
            _qhandle: &wayland_client::QueueHandle<D>,
        ) {
        let workspace_state = state.workspace_state();
        match event {
            zext_workspace_handle_v1::Event::State { state } => {
                info!("recv workspace_state event {:?} for workspace {}", state, proxy.id().protocol_id());
                workspace_state.events.push(WorkspaceEvent::WorkspaceState(proxy.clone(), state));
            },
            zext_workspace_handle_v1::Event::Name { name } => {
                info!("recv workspace_name event {:?} for workspace {}", name, proxy.id().protocol_id());
                workspace_state.events.push(WorkspaceEvent::WorkspaceName(proxy.clone(), name));
            },
            zext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                info!("recv workspace_coordinates event {:?} for workspace {}", coordinates, proxy.id().protocol_id());
                workspace_state.events.push(WorkspaceEvent::WorkspaceCoord(proxy.clone(), coordinates));
            },
            zext_workspace_handle_v1::Event::Remove => {
                info!("recv workspace_remove event for workspace {}", proxy.id().protocol_id());
                workspace_state.events.push(WorkspaceEvent::WorkspaceRemoved(proxy.clone()));
            },
       }
    }
}


impl<D> RegistryHandler<D> for WorkspaceState
where
    D: Dispatch<ZextWorkspaceHandleV1, ()>
        + Dispatch<ZextWorkspaceGroupHandleV1, ()>
        + Dispatch<ZextWorkspaceManagerV1, GlobalData>
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        data: &mut D,
        _: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _version: u32,
    ) {
        if interface == "ext_workspace_unstable_v1" {
            let _manager = data
                .registry()
                .bind_specific(qh, name, 1..=4, GlobalData)
                .expect("Failed to bind global ext_workspace_unstable_v1 object!");
        }
    }

    fn remove_global(
        _data: &mut D,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
        _name: u32,
        interface: &str,
    ) {
        if interface == "ext_workspace_unstable_v1" {
            //TODO
        }
    }
}


#[macro_export]
macro_rules! delegate_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::zext_workspace_manager_v1::ZextWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData 
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::zext_workspace_group_handle_v1::ZextWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::zext_workspace_handle_v1::ZextWorkspaceHandleV1: () 
        ] => $crate::workspace_state::WorkspaceState);
    };
}