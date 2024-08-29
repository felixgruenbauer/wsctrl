use log::{info, warn};
use serde::{ser::{SerializeSeq, SerializeStruct}, Deserialize, Serialize, Serializer};
use smithay_client_toolkit::{
    output::{OutputData, OutputInfo},
    reexports::client::protocol::wl_output::WlOutput,
};
use std::collections::HashSet;
use wayland_backend::client::ObjectId;

use crate::ext::workspace::{
    ext_v0::client::{
        zext_workspace_group_handle_v1::{self, ZextWorkspaceGroupHandleV1},
        zext_workspace_handle_v1::{self, ZextWorkspaceHandleV1},
        zext_workspace_manager_v1::{self, ZextWorkspaceManagerV1}, //zext_workspace_group_handle_v1::ZextWorkspaceGroupCapabilitiesV1 as GroupCapabilities,
                                                                   //zext_workspace_handle_v1::ZextWorkspaceCapabilitiesV1 as WorkspaceCapabilities,
    },
    ext_v1::client::{
        ext_workspace_group_handle_v1::{
            self, ExtWorkspaceGroupCapabilitiesV1 as GroupCapabilities, ExtWorkspaceGroupHandleV1,
        },
        ext_workspace_handle_v1::{
            self, ExtWorkspaceCapabilitiesV1 as WorkspaceCapabilities, ExtWorkspaceHandleV1, State as StateV1
        },
        ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
    },
    cosmic_v1::client::{
        zcosmic_workspace_group_handle_v1::{self, ZcosmicWorkspaceGroupHandleV1},
        zcosmic_workspace_handle_v1::{self, ZcosmicWorkspaceHandleV1},
        zcosmic_workspace_manager_v1::{self, ZcosmicWorkspaceManagerV1},
    }
};

use smithay_client_toolkit::{
    globals::GlobalData,
    reexports::client::Dispatch,
    registry::{ProvidesRegistryState, RegistryHandler, RegistryState},
};
use wayland_client::{Connection, Proxy, QueueHandle, WEnum};

enum ManagerHandle {
    ExtV0(ZextWorkspaceManagerV1),
    ExtV1(ExtWorkspaceManagerV1),
    CosmicV1(ZcosmicWorkspaceManagerV1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupHandle {
    ExtV0(ZextWorkspaceGroupHandleV1),
    ExtV1(ExtWorkspaceGroupHandleV1),
    CosmicV1(ZcosmicWorkspaceGroupHandleV1)
}

impl GroupHandle {
    pub fn id(&self) -> ObjectId {
        match &self {
            GroupHandle::ExtV1(handle) => handle.id(),
            GroupHandle::ExtV0(handle) => handle.id(),
            GroupHandle::CosmicV1(handle) => handle.id(),
        }
    }
    pub fn create_workspace(&self, name: String) {
        match &self {
            GroupHandle::ExtV0(handle) => handle.create_workspace(name),
            GroupHandle::ExtV1(handle) => handle.create_workspace(name),
            GroupHandle::CosmicV1(handle) => handle.create_workspace(name),
        }
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceHandle {
    ExtV0(ZextWorkspaceHandleV1),
    ExtV1(ExtWorkspaceHandleV1),
    CosmicV1(ZcosmicWorkspaceHandleV1)
}
impl WorkspaceHandle {
    pub fn id(&self) -> ObjectId {
        match &self {
            WorkspaceHandle::ExtV1(handle) => handle.id(),
            WorkspaceHandle::ExtV0(handle) => handle.id(),
            WorkspaceHandle::CosmicV1(handle) => handle.id(),
        }
    }
    pub fn activate(&self) {
        match &self {
            WorkspaceHandle::ExtV0(handle) => handle.activate(),
            WorkspaceHandle::ExtV1(handle) => handle.activate(),
            WorkspaceHandle::CosmicV1(handle) => handle.activate(),
        }
    }
    pub fn deactivate(&self) {
        match &self {
            WorkspaceHandle::ExtV0(handle) => handle.deactivate(),
            WorkspaceHandle::ExtV1(handle) => handle.deactivate(),
            WorkspaceHandle::CosmicV1(handle) => handle.deactivate(),
        }
    }
    pub fn destroy(&self) {
        match &self {
            WorkspaceHandle::ExtV0(handle) => handle.destroy(),
            WorkspaceHandle::ExtV1(handle) => handle.destroy(),
            WorkspaceHandle::CosmicV1(handle) => handle.destroy(),
        }
    }
    pub fn remove(&self) {
        match &self {
            WorkspaceHandle::ExtV0(handle) => handle.remove(),
            WorkspaceHandle::ExtV1(handle) => handle.remove(),
            WorkspaceHandle::CosmicV1(handle) => handle.remove(),
        }
    }
    pub fn assign(&self, group: &GroupHandle) -> Result<(), String> {
        match &self {
            WorkspaceHandle::ExtV0(_) => Err(format!("assign request not supported by unstable protocol version")),
            WorkspaceHandle::ExtV1(handle) => {
                match group {
                    GroupHandle::ExtV1(group_handle) => {
                        handle.assign(group_handle);
                        Ok(())
                    },
                    _ => Err(format!("assign request workspace and group handle version mismatch"))
                }
            },
            WorkspaceHandle::CosmicV1(_) => Err(format!("assign request not supported by unstable protocol version")),
        }
    }

}

pub struct WorkspaceState {
    pub groups: Vec<WorkspaceGroup>,
    pub workspaces: Vec<Workspace>,
    manager: ManagerHandle,
    events: Vec<WorkspaceEvent>,
    pub group_cap: Option<GroupCapabilities>,
    pub workspace_cap: Option<WorkspaceCapabilities>
}

impl WorkspaceState {
    pub fn commit(&self) {
        match &self.manager {
            ManagerHandle::ExtV0(manager) => manager.commit(),
            ManagerHandle::ExtV1(manager) => manager.commit(),
            ManagerHandle::CosmicV1(manager) => manager.commit(),
        }
    }
}

impl Serialize for WorkspaceState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer 
    {
        let mut state = serializer.serialize_seq(Some(self.groups.len()))?;

        #[derive(Serialize)]
        struct GroupSerialize {
            #[serde(serialize_with = "serialize_wloutput")] 
            output: Option<WlOutput>,
            #[serde(serialize_with = "serialize_group_handle")] 
            group_handle: Option<GroupHandle>,
            workspaces: Vec<Workspace>
        }
        for group in self.groups.iter() {
            let workspaces = self.workspaces.iter().filter(|ws| ws.group.clone().is_some_and(|g| g == group.handle.id())).cloned().collect::<Vec<_>>();
            if !workspaces.is_empty() {
                let group_s = GroupSerialize {
                    output: group.output.clone(),
                    group_handle: Some(group.handle.clone()),
                    workspaces: workspaces
                };
                state.serialize_element(&group_s)?;
            }
        }
            
        // unassigned workspaces
        let unassigned_workspaces = self.workspaces.iter().filter(|ws| ws.group.is_none()).cloned().collect::<Vec<_>>();
        if !unassigned_workspaces.is_empty() {
            state.serialize_element(&GroupSerialize {
                output: None,
                group_handle: None,
                workspaces: unassigned_workspaces,
            })?;
        }
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceGroup {
    pub output: Option<WlOutput>,
    pub handle: GroupHandle,
}

fn serialize_group_handle<S>(x: &Option<GroupHandle>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        None => s.serialize_none(), 
        Some(x) => s.serialize_some(&x.id().protocol_id())
    }
}


fn serialize_wloutput<S>(x: &Option<WlOutput>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(output) => {
            let info = &output.data::<OutputData>()
                    .and_then(|data| data.with_output_info(|info| Some(info.clone()))
            );

            let mut s = s.serialize_struct("Output", 5)?;
            s.serialize_field("protocolId", &output.id().protocol_id())?;
            s.serialize_field("name", &info.clone().and_then(|info| info.name))?;
            s.serialize_field("location", &info.clone().and_then(|info| Some(info.location)))?;
            s.serialize_field("description", &info.clone().and_then(|info| info.description))?;
            s.serialize_field("globalId", &info.clone().and_then(|info| Some(info.id)))?;
            s.end()
        },
        None => s.serialize_none()
    }
}
impl WorkspaceGroup {
    pub fn get_output_info(&self) -> Option<OutputInfo> {
        self.output.as_ref().and_then(|o| {
            o.data::<OutputData>()
                .and_then(|data| data.with_output_info(|info| Some(info.clone())))
        })
    }

    pub fn get_output_name(&self) -> Option<String> {
        self.output.as_ref().and_then(|o| {
            o.data::<OutputData>().and_then(|data| {
                data.with_output_info(|info| info.name.as_ref().and_then(|name| Some(name.clone())))
            })
        })
    }

    pub fn create_workspace(&self, name: String) {
        self.handle.create_workspace(name)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Workspace {
    #[serde(serialize_with = "serialize_workspace_handle")] 
    pub handle: WorkspaceHandle,
    pub name: Option<String>,
    pub coordinates: Option<Vec<u8>>,
    pub state: HashSet<State>,
    #[serde(skip_serializing)] 
    pub group: Option<ObjectId>,
}

fn serialize_workspace_handle<S>(x: &WorkspaceHandle, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u32(x.id().protocol_id())
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum State {
    Active,
    Urgent,
    Hidden
}

impl Workspace {
    pub fn activate(&self) {
        self.handle.activate()
    }
    pub fn deactivate(&self) {
        self.handle.deactivate()
    }
    pub fn destroy(&self) {
        self.handle.destroy()
    }
    pub fn remove(&self) {
        self.handle.remove()
    }
    pub fn assign(&self, group: &GroupHandle) -> Result<(), String> {
        self.handle.assign(group)
    }
}

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    WorkspaceGroupCreated(GroupHandle),
    WorkspaceGroupRemoved(GroupHandle),
    WorkspaceGroupCapabilities(GroupCapabilities),
    OutputEnter(GroupHandle, WlOutput),
    OutputLeave(GroupHandle, WlOutput),
    WorkspaceEnter(WorkspaceHandle, GroupHandle),
    WorkspaceLeave(WorkspaceHandle, GroupHandle),
    WorkspaceCreated(Option<GroupHandle>, WorkspaceHandle),
    WorkspaceRemoved(WorkspaceHandle),
    WorkspaceState(WorkspaceHandle, HashSet<State>),
    WorkspaceCapabilities(WorkspaceCapabilities),
    WorkspaceCoord(WorkspaceHandle, Vec<u8>),
    WorkspaceName(WorkspaceHandle, String),
}

pub trait WorkspaceHandler {
    fn workspace_state(&self) -> &WorkspaceState;
    fn workspace_state_mut(&mut self) -> &mut WorkspaceState;
    fn handle_events(&mut self, events: Vec<WorkspaceEvent>);
}

pub trait WorkspaceDispatch:
    Dispatch<ZextWorkspaceHandleV1, ()>
    + Dispatch<ZextWorkspaceGroupHandleV1, ()>
    + Dispatch<ZextWorkspaceManagerV1, GlobalData>
    + Dispatch<ExtWorkspaceHandleV1, ()>
    + Dispatch<ExtWorkspaceGroupHandleV1, ()>
    + Dispatch<ExtWorkspaceManagerV1, GlobalData>
    + Dispatch<ZcosmicWorkspaceHandleV1, ()>
    + Dispatch<ZcosmicWorkspaceGroupHandleV1, ()>
    + Dispatch<ZcosmicWorkspaceManagerV1, GlobalData>
    + WorkspaceHandler
    + 'static
{
}

impl<T> WorkspaceDispatch for T where
    T: Dispatch<ZextWorkspaceHandleV1, ()>
        + Dispatch<ZextWorkspaceGroupHandleV1, ()>
        + Dispatch<ZextWorkspaceManagerV1, GlobalData>
        + Dispatch<ExtWorkspaceHandleV1, ()>
        + Dispatch<ExtWorkspaceGroupHandleV1, ()>
        + Dispatch<ExtWorkspaceManagerV1, GlobalData>
        + Dispatch<ZcosmicWorkspaceHandleV1, ()>
        + Dispatch<ZcosmicWorkspaceGroupHandleV1, ()>
        + Dispatch<ZcosmicWorkspaceManagerV1, GlobalData>
        + WorkspaceHandler
        + 'static
{
}

impl WorkspaceState {
    pub fn new<D: WorkspaceDispatch>(
        registry_state: &RegistryState,
        qh: &QueueHandle<D>,
        protocol_version: &Option<String>
    ) -> Result<Self, String> {
        let manager: ManagerHandle = {
            if let Some(protocol) = protocol_version {
                match protocol.as_str() {
                    "ext_v0" => ManagerHandle::ExtV0(registry_state.bind_one(qh, 1..=1, GlobalData).expect("Failed to bind 'ext_workspace_manager_v0'!")),
                    "ext_v1" => ManagerHandle::ExtV1(registry_state.bind_one(qh, 1..=1, GlobalData).expect("Failed to bind 'ext_workspace_manager_v1'!")),
                    "cosmic_v1" => ManagerHandle::CosmicV1(registry_state.bind_one(qh, 1..=1, GlobalData).expect("Failed to bind 'zcosmic_workspace_manager_v1'!")),
                    _ => return Err(format!("only protocols zext_workspace_unstable_v1, ext_workspace_unstable_v1 and zcosmic_workspace_unstable_v1 supported!"))
                }
            } else {
                if let Ok(handle) = registry_state.bind_one(qh, 1..=1, GlobalData) {
                    ManagerHandle::ExtV0(handle)
                } else if let Ok(handle) = registry_state.bind_one(qh, 1..=1, GlobalData) {
                    ManagerHandle::ExtV1(handle)
                } else if let Ok(handle) = registry_state.bind_one(qh, 1..=1, GlobalData) {
                    ManagerHandle::CosmicV1(handle)
                } else {
                    return Err(format!("unable to bind any workspace management protocol version!"))
                }
            }
        };
        Ok(WorkspaceState {
            groups: Vec::new(),
            workspaces: Vec::new(),
            manager,
            events: vec![],
            group_cap: None,
            workspace_cap: None,
        })
    }
}

impl<D: WorkspaceDispatch> Dispatch<ZcosmicWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        _handle: &ZcosmicWorkspaceManagerV1,
        event: <ZcosmicWorkspaceManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        use zcosmic_workspace_manager_v1::Event;
        match event {
            Event::WorkspaceGroup { workspace_group } => {
                info!(
                    "received workspace_group event with id {}",
                    workspace_group.id().protocol_id()
                );
                state
                    .workspace_state_mut()
                    .events
                    .push(WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::CosmicV1(
                        workspace_group,
                    )));
            }
            Event::Done {} => {
                info!("received done event");
                let events = state.workspace_state_mut().events.drain(..).collect();
                state.handle_events(events);
            }
            Event::Finished {} => {
                // todo handle event
                info!("received manager finished event");
            }
        }
    }

    wayland_client::event_created_child!(D, ZcosmicWorkspaceManagerV1, [
        0 => (ZcosmicWorkspaceGroupHandleV1, ()),
        //1 => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl<D: WorkspaceDispatch> Dispatch<ZcosmicWorkspaceGroupHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZcosmicWorkspaceGroupHandleV1,
        event: <ZcosmicWorkspaceGroupHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        use zcosmic_workspace_group_handle_v1::Event;
        let event = match event {
            Event::OutputEnter { output } => {
                info!(
                    "recieved output_enter event (workspace_group id: {}, output: {})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                );
                WorkspaceEvent::OutputEnter(
                    GroupHandle::CosmicV1(handle.clone()),
                    output,
                )
            }
            Event::OutputLeave { output } => {
                info!(
                    "recieved output_leave event (workspace_group id: {}, output: {})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                );
                WorkspaceEvent::OutputLeave(
                    GroupHandle::CosmicV1(handle.clone()),
                    output,
                )
            }
            Event::Remove => {
                info!(
                    "received workspace_group_removed event for id {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::CosmicV1(
                    handle.clone(),
                ))
            }
            Event::Capabilities { capabilities } => {
                todo!()
                //match capabilities {
                //    WEnum::Value(caps) => {
                //        WorkspaceEvent::WorkspaceGroupCapabilities(caps)
                //    }
                //    WEnum::Unknown(unknown) => {
                //        warn!("received capabilities event with unknown value: {unknown}");
                //        return;
                //    }
                //}
            }
            Event::Workspace { workspace } => {
                WorkspaceEvent::WorkspaceCreated(
                    Some(GroupHandle::CosmicV1(handle.clone())),
                    WorkspaceHandle::CosmicV1(workspace),
                )
            }
        };
        state.workspace_state_mut().events.push(event);
    }

    wayland_client::event_created_child!(D, ZcosmicWorkspaceManagerV1, [
        //0 => (ZcosmicWorkspaceGroupHandleV1, ()),
        3 => (ZcosmicWorkspaceHandleV1, ()),
    ]);

}

impl<D: WorkspaceDispatch> Dispatch<ZcosmicWorkspaceHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZcosmicWorkspaceHandleV1,
        event: <ZcosmicWorkspaceHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        use zcosmic_workspace_handle_v1::Event;
        let event = match event {
            Event::State { state } => {
                info!(
                    "recv workspace_state event {:?} for workspace {}",
                    state,
                    handle.id().protocol_id()
                );
                if state.len() != 3 { return };
                let mut state_set = HashSet::new();
                if state[0] == 0 {state_set.insert(State::Active);};
                if state[1] == 1 {state_set.insert(State::Urgent);};
                if state[2] == 2 {state_set.insert(State::Hidden);};
                WorkspaceEvent::WorkspaceState(WorkspaceHandle::CosmicV1(handle.clone()), state_set)
            },
            Event::Name { name } => {
                info!(
                    "recv workspace_name event {:?} for workspace {}",
                    name,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::CosmicV1(handle.clone()), name)
            },
            Event::Coordinates { coordinates } => {
                info!(
                    "recv workspace_coordinates event {:?} for workspace {}",
                    coordinates,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCoord(WorkspaceHandle::CosmicV1(handle.clone()), coordinates)
            },
            Event::Remove => {
                info!(
                    "recv workspace_remove event for workspace {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::CosmicV1(handle.clone()))
            },
            Event::Capabilities { capabilities } => {
                return;
                //match capabilities {
                //    WEnum::Value(caps) => {
                //        WorkspaceEvent::WorkspaceCapabilities(caps)
                //    },
                //    WEnum::Unknown(unknown) => {
                //        warn!("received capabilities event with unknown value: {unknown}");
                //        return;
                //    },
                //}
            }
            Event::TilingState { state } => {
                return
            },
        };
        state.workspace_state_mut().events.push(event);
    }
}



impl<D: WorkspaceDispatch> Dispatch<ExtWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        _handle: &ExtWorkspaceManagerV1,
        event: <ExtWorkspaceManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                info!(
                    "received workspace_group event with id {}",
                    workspace_group.id().protocol_id()
                );
                state
                    .workspace_state_mut()
                    .events
                    .push(WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::ExtV1(
                        workspace_group,
                    )));
            }
            ext_workspace_manager_v1::Event::Done {} => {
                info!("received done event");
                let events = state.workspace_state_mut().events.drain(..).collect();
                state.handle_events(events);
            }
            ext_workspace_manager_v1::Event::Finished {} => {
                // todo handle event
                info!("received manager finished event");
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => state
                .workspace_state_mut()
                .events
                .push(WorkspaceEvent::WorkspaceCreated(
                    None,
                    WorkspaceHandle::ExtV1(workspace),
                )),
        }
    }

    wayland_client::event_created_child!(D, ExtWorkspaceManagerV1, [
        0 => (ExtWorkspaceGroupHandleV1, ()),
        1 => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl<D: WorkspaceDispatch> Dispatch<ExtWorkspaceGroupHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ExtWorkspaceGroupHandleV1,
        event: <ExtWorkspaceGroupHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        let event = match event {
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                info!(
                    "recieved output_enter event (workspace_group id: {}, output: {})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                );
                WorkspaceEvent::OutputEnter(
                    GroupHandle::ExtV1(handle.clone()),
                    output,
                )
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                info!(
                    "recieved output_leave event (workspace_group id: {}, output: {})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                );
                WorkspaceEvent::OutputLeave(
                    GroupHandle::ExtV1(handle.clone()),
                    output,
                )
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                info!(
                    "received workspace_group_removed event for id {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::ExtV1(
                    handle.clone(),
                ))
            }
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                match capabilities {
                    WEnum::Value(caps) => {
                        WorkspaceEvent::WorkspaceGroupCapabilities(caps)
                    }
                    WEnum::Unknown(unknown) => {
                        warn!("received capabilities event with unknown value: {unknown}");
                        return;
                    }
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                WorkspaceEvent::WorkspaceEnter(
                    WorkspaceHandle::ExtV1(workspace),
                    GroupHandle::ExtV1(handle.clone()),
                )
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                WorkspaceEvent::WorkspaceLeave(
                    WorkspaceHandle::ExtV1(workspace),
                    GroupHandle::ExtV1(handle.clone()),
                )
            }
        };
        state.workspace_state_mut().events.push(event);
    }
}

impl<D: WorkspaceDispatch> Dispatch<ExtWorkspaceHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ExtWorkspaceHandleV1,
        event: <ExtWorkspaceHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        let event = match event {
            ext_workspace_handle_v1::Event::State { state } => {
                info!(
                    "recv workspace_state event {:?} for workspace {}",
                    state,
                    handle.id().protocol_id()
                );
                match state {
                    WEnum::Value(s) => {
                        let mut state_set = HashSet::new();
                        if s.intersects(StateV1::Active) {
                            state_set.insert(State::Active);
                        }
                        if s.intersects(StateV1::Urgent) {
                            state_set.insert(State::Urgent);
                        }
                        if s.intersects(StateV1::Hidden) {
                            state_set.insert(State::Hidden);
                        };
                        WorkspaceEvent::WorkspaceState(WorkspaceHandle::ExtV1(handle.clone()), state_set)
                    }
                    WEnum::Unknown(unknown) => {
                        warn!("received workspace state event with unknown value: {unknown}");
                        return;
                    }
                }
            },
            ext_workspace_handle_v1::Event::Name { name } => {
                info!(
                    "recv workspace_name event {:?} for workspace {}",
                    name,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::ExtV1(handle.clone()), name)
            },
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                info!(
                    "recv workspace_coordinates event {:?} for workspace {}",
                    coordinates,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCoord(WorkspaceHandle::ExtV1(handle.clone()), coordinates)
            },
            ext_workspace_handle_v1::Event::Removed => {
                info!(
                    "recv workspace_remove event for workspace {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::ExtV1(handle.clone()))
            },
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                match capabilities {
                    WEnum::Value(caps) => {
                        WorkspaceEvent::WorkspaceCapabilities(caps)
                    },
                    WEnum::Unknown(unknown) => {
                        warn!("received capabilities event with unknown value: {unknown}");
                        return;
                    },
                }
            }
        };
        state.workspace_state_mut().events.push(event);
    }
}

impl<D: WorkspaceDispatch> Dispatch<ZextWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        _proxy: &ZextWorkspaceManagerV1,
        event: <ZextWorkspaceManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        let event = match event {
            zext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                info!(
                    "received workspace_group event with id {}",
                    workspace_group.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupCreated(
                        GroupHandle::ExtV0(workspace_group),
                    )
            }
            zext_workspace_manager_v1::Event::Done {} => {
                info!("received done event");
                let events = state.workspace_state_mut().events.drain(..).collect();
                state.handle_events(events);
                return
            }
            zext_workspace_manager_v1::Event::Finished {} => {
                // todo handle event
                info!("received manager finished event");
                return
            }
        };
        state.workspace_state_mut().events.push(event);
    }

    wayland_client::event_created_child!(D, ZextWorkspaceManagerV1, [
        0 => (ZextWorkspaceGroupHandleV1, ()),
    ]);
}

impl<D: WorkspaceDispatch> Dispatch<ZextWorkspaceGroupHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZextWorkspaceGroupHandleV1,
        event: <ZextWorkspaceGroupHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        let event = match event {
            zext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                info!(
                    "recieved output_enter event (workspace_group id: {}, output: {} {:?})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                    output.data::<OutputData>().unwrap()
                );
                WorkspaceEvent::OutputEnter(
                        GroupHandle::ExtV0(handle.clone()),
                        output,
                    )
            }
            zext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                info!(
                    "recieved output_leave event (workspace_group id: {}, output: {} {:?})",
                    handle.id().protocol_id(),
                    output.id().protocol_id(),
                    output.data::<OutputData>().unwrap()
                );
                WorkspaceEvent::OutputLeave(
                        GroupHandle::ExtV0(handle.clone()),
                        output,
                    )
            }
            zext_workspace_group_handle_v1::Event::Remove => {
                info!(
                    "received workspace_group_remove event for id {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupRemoved(
                        GroupHandle::ExtV0(handle.clone()),
                    )
            }
            zext_workspace_group_handle_v1::Event::Workspace { workspace } => {
                info!(
                    "received workspace event with id {}",
                    workspace.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCreated(
                        Some(GroupHandle::ExtV0(handle.clone())),
                        WorkspaceHandle::ExtV0(workspace),
                    )
            }
        };
        state.workspace_state_mut().events.push(event);
    }
    wayland_client::event_created_child!(D, ZextWorkspaceGroupHandleV1, [
        2 => (ZextWorkspaceHandleV1, ()),
    ]);
}

impl<D: WorkspaceDispatch> Dispatch<ZextWorkspaceHandleV1, (), D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZextWorkspaceHandleV1,
        event: <ZextWorkspaceHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        let event = match event {
            zext_workspace_handle_v1::Event::State { state } => {
                info!(
                    "recv workspace_state event {:?} for workspace {}",
                    state,
                    handle.id().protocol_id()
                );
                let mut state_set = HashSet::new();
                if state.get(0).is_some_and(|s| *s == 0) {
                    state_set.insert(State::Active);
                };
                if state.get(1).is_some_and(|s| *s == 1) {
                    state_set.insert(State::Urgent);
                };
                if state.get(2).is_some_and(|s| *s == 2) {
                    state_set.insert(State::Hidden);
                }
                WorkspaceEvent::WorkspaceState(WorkspaceHandle::ExtV0(handle.clone()), state_set)
            }
            zext_workspace_handle_v1::Event::Name { name } => {
                info!(
                    "recv workspace_name event {:?} for workspace {}",
                    name,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceName(
                    WorkspaceHandle::ExtV0(handle.clone()),
                    name,
                )
            }
            zext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                info!(
                    "recv workspace_coordinates event {:?} for workspace {}",
                    coordinates,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCoord(
                    WorkspaceHandle::ExtV0(handle.clone()),
                    coordinates,
                )
            }
            zext_workspace_handle_v1::Event::Remove => {
                info!(
                    "recv workspace_remove event for workspace {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::ExtV0(
                        handle.clone(),
                    ))
            }
        };
        state.workspace_state_mut().events.push(event);

    }
}

impl<D> RegistryHandler<D> for WorkspaceState
where
    D: WorkspaceDispatch + ProvidesRegistryState + 'static,
{
    fn new_global(
        _data: &mut D,
        _: &Connection,
        _qh: &QueueHandle<D>,
        _name: u32,
        _interface: &str,
        _version: u32,
    ) {
    }

    fn remove_global(
        _data: &mut D,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
        _name: u32,
        _interface: &str,
    ) {
    }
}

#[macro_export]
macro_rules! delegate_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v0::client::zext_workspace_manager_v1::ZextWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v0::client::zext_workspace_group_handle_v1::ZextWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v0::client::zext_workspace_handle_v1::ZextWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::cosmic_v1::client::zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::cosmic_v1::client::zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::cosmic_v1::client::zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);


    };
}
