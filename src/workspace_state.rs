use log::{info, warn};
use serde::{ser::{SerializeSeq, SerializeStruct}, Deserialize, Serialize, Serializer};
use smithay_client_toolkit::{
    output::{OutputData, OutputInfo},
    reexports::client::protocol::wl_output::WlOutput,
};
use std::collections::HashSet;
use wayland_backend::client::ObjectId;

use crate::ext::workspace::{
    unstable_v1::client::{
        zext_workspace_group_handle_v1::{self, ZextWorkspaceGroupHandleV1},
        zext_workspace_handle_v1::{self, ZextWorkspaceHandleV1},
        zext_workspace_manager_v1::{self, ZextWorkspaceManagerV1}, //zext_workspace_group_handle_v1::ZextWorkspaceGroupCapabilitiesV1 as GroupCapabilities,
                                                                   //zext_workspace_handle_v1::ZextWorkspaceCapabilitiesV1 as WorkspaceCapabilities,
    },
    v1::client::{
        ext_workspace_group_handle_v1::{
            self, ExtWorkspaceGroupCapabilitiesV1 as GroupCapabilities, ExtWorkspaceGroupHandleV1,
        },
        ext_workspace_handle_v1::{
            self, ExtWorkspaceCapabilitiesV1 as WorkspaceCapabilities, ExtWorkspaceHandleV1, State as StateV1
        },
        ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
    },
};

use smithay_client_toolkit::{
    globals::GlobalData,
    reexports::client::Dispatch,
    registry::{ProvidesRegistryState, RegistryHandler, RegistryState},
};
use wayland_client::{Connection, Proxy, QueueHandle, WEnum};

enum ManagerHandle {
    V1(ExtWorkspaceManagerV1),
    Unstable(ZextWorkspaceManagerV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupHandle {
    V1(ExtWorkspaceGroupHandleV1),
    Unstable(ZextWorkspaceGroupHandleV1),
}

impl GroupHandle {
    pub fn id(&self) -> ObjectId {
        match &self {
            GroupHandle::V1(handle) => handle.id(),
            GroupHandle::Unstable(handle) => handle.id(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceHandle {
    V1(ExtWorkspaceHandleV1),
    Unstable(ZextWorkspaceHandleV1),
}
impl WorkspaceHandle {
    pub fn id(&self) -> ObjectId {
        match &self {
            WorkspaceHandle::V1(handle) => handle.id(),
            WorkspaceHandle::Unstable(handle) => handle.id(),
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
            ManagerHandle::Unstable(manager) => manager.commit(),
            ManagerHandle::V1(manager) => manager.commit(),
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
        match &self.handle {
            GroupHandle::V1(handle) => handle.create_workspace(name),
            GroupHandle::Unstable(handle) => handle.create_workspace(name),
        }
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
        match &self.handle {
            WorkspaceHandle::V1(handle) => handle.activate(),
            WorkspaceHandle::Unstable(handle) => handle.activate(),
        }
    }
    pub fn deactivate(&self) {
        match &self.handle {
            WorkspaceHandle::V1(handle) => handle.deactivate(),
            WorkspaceHandle::Unstable(handle) => handle.deactivate(),
        }
    }
    pub fn destroy(&self) {
        match &self.handle {
            WorkspaceHandle::V1(handle) => handle.destroy(),
            WorkspaceHandle::Unstable(handle) => handle.destroy(),
        }
    }
    pub fn remove(&self) {
        match &self.handle {
            WorkspaceHandle::V1(handle) => handle.remove(),
            WorkspaceHandle::Unstable(handle) => handle.remove(),
        }
    }
    pub fn assign(&self, group: &GroupHandle) -> Result<(), String> {
        match &self.handle {
            WorkspaceHandle::V1(handle) => {
                match group {
                    GroupHandle::V1(group_handle) => {
                        handle.assign(group_handle);
                        Ok(())
                    },
                    GroupHandle::Unstable(_) => Err(format!("assign request workspace and group handle version mismatch (unstable vs v1)"))
                }
            },
            WorkspaceHandle::Unstable(_) => Err(format!("assign request not supported by unstable protocol version")),
        }
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
        + WorkspaceHandler
        + 'static
{
}

impl WorkspaceState {
    pub fn new<D: WorkspaceDispatch>(
        registry_state: &RegistryState,
        qh: &QueueHandle<D>,
    ) -> Result<Self, String> {
        let manager: ManagerHandle = {
            if let Ok(manager_v1) = registry_state.bind_one(qh, 1..=1, GlobalData).map_err(|e| warn!("Failed to bind 'ext_workspace_manager_v1' with error {e}. Trying 'zext_workspace_manager_v1'.")) {
                ManagerHandle::V1(manager_v1)
            } else {
                ManagerHandle::Unstable(registry_state.bind_one(qh, 1..=1, GlobalData).expect("Failed to bind 'ext_workspace_manager_v1' or 'zext_workspace_manager_v1' globals! Does the compositor support the ext_workspace protocol?"))
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
                    .push(WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::V1(
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
                    WorkspaceHandle::V1(workspace),
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
                    GroupHandle::V1(handle.clone()),
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
                    GroupHandle::V1(handle.clone()),
                    output,
                )
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                info!(
                    "received workspace_group_removed event for id {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::V1(
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
                    WorkspaceHandle::V1(workspace),
                    GroupHandle::V1(handle.clone()),
                )
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                WorkspaceEvent::WorkspaceLeave(
                    WorkspaceHandle::V1(workspace),
                    GroupHandle::V1(handle.clone()),
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
                        WorkspaceEvent::WorkspaceState(WorkspaceHandle::V1(handle.clone()), state_set)
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
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::V1(handle.clone()), name)
            },
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                info!(
                    "recv workspace_coordinates event {:?} for workspace {}",
                    coordinates,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCoord(WorkspaceHandle::V1(handle.clone()), coordinates)
            },
            ext_workspace_handle_v1::Event::Removed => {
                info!(
                    "recv workspace_remove event for workspace {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::V1(handle.clone()))
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
                        GroupHandle::Unstable(workspace_group),
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
                        GroupHandle::Unstable(handle.clone()),
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
                        GroupHandle::Unstable(handle.clone()),
                        output,
                    )
            }
            zext_workspace_group_handle_v1::Event::Remove => {
                info!(
                    "received workspace_group_remove event for id {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceGroupRemoved(
                        GroupHandle::Unstable(handle.clone()),
                    )
            }
            zext_workspace_group_handle_v1::Event::Workspace { workspace } => {
                info!(
                    "received workspace event with id {}",
                    workspace.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceCreated(
                        Some(GroupHandle::Unstable(handle.clone())),
                        WorkspaceHandle::Unstable(workspace),
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
                WorkspaceEvent::WorkspaceState(WorkspaceHandle::Unstable(handle.clone()), state_set)
            }
            zext_workspace_handle_v1::Event::Name { name } => {
                info!(
                    "recv workspace_name event {:?} for workspace {}",
                    name,
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceName(
                    WorkspaceHandle::Unstable(handle.clone()),
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
                    WorkspaceHandle::Unstable(handle.clone()),
                    coordinates,
                )
            }
            zext_workspace_handle_v1::Event::Remove => {
                info!(
                    "recv workspace_remove event for workspace {}",
                    handle.id().protocol_id()
                );
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::Unstable(
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
            $crate::ext::workspace::unstable_v1::client::zext_workspace_manager_v1::ZextWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::unstable_v1::client::zext_workspace_group_handle_v1::ZextWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::unstable_v1::client::zext_workspace_handle_v1::ZextWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);

    };
}
