use std::{cmp::Ordering, fmt::Display};

use log::{debug, info, warn};
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Serialize, Serializer,
};
use smithay_client_toolkit::{
    output::{OutputData, OutputInfo},
    reexports::client::protocol::wl_output::WlOutput,
};

use bitflags::bitflags;

use crate::ext::workspace::{
        cosmic_v1::client::{
            zcosmic_workspace_group_handle_v1::{self, ZcosmicWorkspaceGroupHandleV1},
            zcosmic_workspace_handle_v1::{self, TilingState, ZcosmicWorkspaceHandleV1},
            zcosmic_workspace_manager_v1::{self, ZcosmicWorkspaceManagerV1},
        },
        ext_v0::client::{
            zext_workspace_group_handle_v1::{self, ZextWorkspaceGroupHandleV1},
            zext_workspace_handle_v1::{self, ZextWorkspaceHandleV1},
            zext_workspace_manager_v1::{self, ZextWorkspaceManagerV1},
        },
        ext_v1::client::{
            ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
            ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1},
            ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
        },
};

use smithay_client_toolkit::{globals::GlobalData, reexports::client::Dispatch};
use wayland_client::Proxy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, clap::ValueEnum)]
pub enum Protocol {
    ExtV0,
    ExtV1,
    CosmicV1,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupCapabilities(u32);

bitflags! {
    impl GroupCapabilities: u32 {
        const CreateWorkspace = 0b00000001;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceCapabilities(u32);

bitflags! {
    impl WorkspaceCapabilities: u32 {
        const Activate = 0b00000001;
        const Deactivate = 0b00000010;
        const Remove = 0b00000100;
        const Assign = 0b00001000;
        const Rename = 0b00010000;
        const SetTilingState = 0b00100000;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceStates(u32);

bitflags! {
    impl WorkspaceStates: u32 {
        const Active = 0b00000001;
        const Hidden = 0b00000010;
        const Urgent = 0b00000100;
    }
}

pub enum ManagerHandle {
    ExtV0(ZextWorkspaceManagerV1),
    ExtV1(ExtWorkspaceManagerV1),
    CosmicV1(ZcosmicWorkspaceManagerV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupHandle {
    ExtV0(ZextWorkspaceGroupHandleV1),
    ExtV1(ExtWorkspaceGroupHandleV1),
    CosmicV1(ZcosmicWorkspaceGroupHandleV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceHandle {
    ExtV0(ZextWorkspaceHandleV1),
    ExtV1(ExtWorkspaceHandleV1),
    CosmicV1(ZcosmicWorkspaceHandleV1),
}

#[derive(Debug, Clone)]
pub struct WorkspaceGroup {
    pub output: Option<WlOutput>,
    pub handle: GroupHandle,
    pub capabilities: GroupCapabilities,
}

#[derive(Clone, Debug, Serialize)]
pub struct Workspace {
    #[serde(skip_serializing)]
    pub handle: WorkspaceHandle,
    pub name: Option<String>,
    pub id: Option<String>,
    pub coordinates: Vec<u8>,
    pub state: WorkspaceStates,
    #[serde(skip_serializing)]
    pub group: Option<GroupHandle>,
    pub tiling_state: Option<TilingState>,
    pub capabilities: WorkspaceCapabilities,
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

    pub fn id(&self) -> u32 {
        match &self.handle {
            GroupHandle::ExtV1(handle) => handle.id().protocol_id(),
            GroupHandle::ExtV0(handle) => handle.id().protocol_id(),
            GroupHandle::CosmicV1(handle) => handle.id().protocol_id(),
        }
    }
    pub fn create_workspace(&self, name: String) {
        match &self.handle {
            GroupHandle::ExtV0(handle) => handle.create_workspace(name),
            GroupHandle::ExtV1(handle) => handle.create_workspace(name),
            GroupHandle::CosmicV1(handle) => handle.create_workspace(name),
        }
    }
}
impl Workspace {
    pub fn id(&self) -> u32 {
        match &self.handle {
            WorkspaceHandle::ExtV1(handle) => handle.id().protocol_id(),
            WorkspaceHandle::ExtV0(handle) => handle.id().protocol_id(),
            WorkspaceHandle::CosmicV1(handle) => handle.id().protocol_id(),
        }
    }
    pub fn activate(&self) {
        match &self.handle {
            WorkspaceHandle::ExtV0(handle) => handle.activate(),
            WorkspaceHandle::ExtV1(handle) => handle.activate(),
            WorkspaceHandle::CosmicV1(handle) => handle.activate(),
        }
    }
    pub fn deactivate(&self) {
        match &self.handle {
            WorkspaceHandle::ExtV0(handle) => handle.deactivate(),
            WorkspaceHandle::ExtV1(handle) => handle.deactivate(),
            WorkspaceHandle::CosmicV1(handle) => handle.deactivate(),
        }
    }
    pub fn destroy(&self) {
        match &self.handle {
            WorkspaceHandle::ExtV0(handle) => handle.destroy(),
            WorkspaceHandle::ExtV1(handle) => handle.destroy(),
            WorkspaceHandle::CosmicV1(handle) => handle.destroy(),
        }
    }
    pub fn remove(&self) {
        match &self.handle {
            WorkspaceHandle::ExtV0(handle) => handle.remove(),
            WorkspaceHandle::ExtV1(handle) => handle.remove(),
            WorkspaceHandle::CosmicV1(handle) => handle.remove(),
        }
    }
    // todo change to group instead of handle
    pub fn assign(&self, group: &GroupHandle) -> Result<(), String> {
        match &self.handle {
            WorkspaceHandle::ExtV1(handle) => match group {
                GroupHandle::ExtV1(group_handle) => {
                    handle.assign(group_handle);
                    Ok(())
                }
                _ => Err(format!(
                    "assign request workspace and group handle version mismatch"
                )),
            },
            _ => Err(format!("assign request not supported by used protocol")),
        }
    }
}

pub struct WorkspaceState {
    pub groups: Vec<WorkspaceGroup>,
    pub workspaces: Vec<Workspace>,
    pub manager: ManagerHandle,
    pub events: Vec<WorkspaceEvent>,
    pub protocol: Protocol,
}

impl WorkspaceState {
    pub fn commit(&self) {
        match &self.manager {
            ManagerHandle::ExtV0(manager) => manager.commit(),
            ManagerHandle::ExtV1(manager) => manager.commit(),
            ManagerHandle::CosmicV1(manager) => manager.commit(),
        }
    }
    pub fn get_workspace_by_handle(&mut self, handle: &WorkspaceHandle) -> &mut Workspace {
        match self.workspaces.iter_mut().find(|ws| &ws.handle == handle) {
            Some(workspace) => workspace,
            None => panic!("no workspace found for handle {handle:?}"),
        }
    }
    pub fn get_group_by_handle(&mut self, handle: &GroupHandle) -> &mut WorkspaceGroup {
        match self.groups.iter_mut().find(|group| &group.handle == handle) {
            Some(group) => group,
            None => panic!("no group found for handle {handle:?}"),
        }
    }
    pub fn sort_workspaces_by_coords(&mut self) {
        self.workspaces.sort_unstable_by(|a, b| {
            (0..a.coordinates.len()).find_map(|i| {
                if a.coordinates[i] > b.coordinates[i] { Some(Ordering::Greater) }
                else if a.coordinates[i] < b.coordinates[i] { Some(Ordering::Less) }
                else { None }
            }).map_or(Ordering::Equal, |o| o)
        });
    }

    pub fn sort_workspaces_by_id(&mut self) {
        self.workspaces.sort_unstable_by(|a, b| a.id().cmp(&b.id()));
    }

    pub fn sort_groups_by_id(&mut self) {
        self.groups.sort_unstable_by(|a, b| a.id().cmp(&b.id()));
    }

    pub fn handle_events(&mut self) {
        for event in self.events.clone().into_iter() {
            match event {
                WorkspaceEvent::WorkspaceGroupCreated(group_handle) => {
                    self.groups.push(WorkspaceGroup {
                        handle: group_handle,
                        output: None,
                        capabilities: GroupCapabilities::empty(),
                    });
                }
                WorkspaceEvent::WorkspaceGroupRemoved(group_handle) => {
                    self.groups.retain(|group| group.handle != group_handle);
                }
                WorkspaceEvent::WorkspaceCreated(group_handle, workspace_handle) => {
                    self.workspaces.push(Workspace {
                        handle: workspace_handle,
                        id: None,
                        name: None,
                        coordinates: Vec::new(),
                        state: WorkspaceStates::empty(),
                        group: group_handle,
                        tiling_state: None,
                        capabilities: WorkspaceCapabilities::empty(),
                    })
                }
                WorkspaceEvent::WorkspaceRemoved(workspace_handle) => {
                    self.workspaces
                        .retain(|workspace| workspace.handle != workspace_handle);
                }
                WorkspaceEvent::OutputEnter(group_handle, output) => {
                    self.get_group_by_handle(&group_handle).output = Some(output);
                }
                WorkspaceEvent::OutputLeave(group_handle, output) => {
                    self.get_group_by_handle(&group_handle).output = None
                }
                WorkspaceEvent::WorkspaceState(workspace_handle, state) => {
                    self.get_workspace_by_handle(&workspace_handle).state = state;
                }
                WorkspaceEvent::WorkspaceId(workspace_handle, id) => {
                    self.get_workspace_by_handle(&workspace_handle).id = Some(id);
                }
                WorkspaceEvent::WorkspaceName(workspace_handle, name) => {
                    self.get_workspace_by_handle(&workspace_handle).name = Some(name);
                }
                WorkspaceEvent::WorkspaceCoord(workspace_handle, coordinates) => {
                    self.get_workspace_by_handle(&workspace_handle).coordinates = coordinates;
                }
                WorkspaceEvent::WorkspaceGroupCapabilities(group_handle, caps) => {
                    self.get_group_by_handle(&group_handle).capabilities = caps;
                }
                WorkspaceEvent::WorkspaceEnter(workspace_handle, group_handle) => {
                    self.get_workspace_by_handle(&workspace_handle).group = Some(group_handle);
                }
                WorkspaceEvent::WorkspaceLeave(workspace_handle, group_handle) => {
                    let workspace = self.get_workspace_by_handle(&workspace_handle);
                    if workspace.group.as_ref().is_some_and(|g| g == &group_handle) {
                        workspace.group = None;
                    } else {
                        warn!("workspace_leave event with wrong group handle");
                    }
                }
                WorkspaceEvent::WorkspaceCapabilities(workspace_handle, caps) => {
                    self.get_workspace_by_handle(&workspace_handle).capabilities = caps;
                }
                WorkspaceEvent::WorkspaceTilingState(workspace_handle, tiling_state) => {
                    self.get_workspace_by_handle(&workspace_handle).tiling_state =
                        Some(tiling_state);
                }
                WorkspaceEvent::ManagerFinished => todo!(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    WorkspaceGroupCreated(GroupHandle),
    WorkspaceGroupRemoved(GroupHandle),
    WorkspaceGroupCapabilities(GroupHandle, GroupCapabilities),
    OutputEnter(GroupHandle, WlOutput),
    OutputLeave(GroupHandle, WlOutput),
    WorkspaceEnter(WorkspaceHandle, GroupHandle),
    WorkspaceLeave(WorkspaceHandle, GroupHandle),
    WorkspaceCreated(Option<GroupHandle>, WorkspaceHandle),
    WorkspaceRemoved(WorkspaceHandle),
    WorkspaceState(WorkspaceHandle, WorkspaceStates),
    WorkspaceCapabilities(WorkspaceHandle, WorkspaceCapabilities),
    WorkspaceCoord(WorkspaceHandle, Vec<u8>),
    WorkspaceName(WorkspaceHandle, String),
    WorkspaceId(WorkspaceHandle, String),
    WorkspaceTilingState(WorkspaceHandle, TilingState),
    ManagerFinished,
}

pub trait WorkspaceHandler {
    fn workspace_state(&self) -> &WorkspaceState;
    fn workspace_state_mut(&mut self) -> &mut WorkspaceState;
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

impl Serialize for WorkspaceState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.groups.len()))?;

        #[derive(Serialize)]
        struct GroupSerialize {
            #[serde(serialize_with = "serialize_wloutput")]
            output: Option<WlOutput>,
            workspaces: Vec<Workspace>,
        }
        for group in self.groups.iter() {
            let workspaces = self
                .workspaces
                .iter()
                .filter(|ws| ws.group.clone().is_some_and(|g| g == group.handle))
                .cloned()
                .collect::<Vec<_>>();
            if !workspaces.is_empty() {
                let group_s = GroupSerialize {
                    output: group.output.clone(),
                    workspaces: workspaces,
                };
                state.serialize_element(&group_s)?;
            }
        }

        // unassigned workspaces
        let unassigned_workspaces = self
            .workspaces
            .iter()
            .filter(|ws| ws.group.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if !unassigned_workspaces.is_empty() {
            state.serialize_element(&GroupSerialize {
                output: None,
                workspaces: unassigned_workspaces,
            })?;
        }
        state.end()
    }
}

fn serialize_wloutput<S>(x: &Option<WlOutput>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(output) => {
            let info = &output
                .data::<OutputData>()
                .and_then(|data| data.with_output_info(|info| Some(info.clone())));

            let mut s = s.serialize_struct("Output", 5)?;
            s.serialize_field("protocolId", &output.id().protocol_id())?;
            s.serialize_field("name", &info.clone().and_then(|info| info.name))?;
            s.serialize_field(
                "location",
                &info.clone().and_then(|info| Some(info.location)),
            )?;
            s.serialize_field(
                "description",
                &info.clone().and_then(|info| info.description),
            )?;
            s.serialize_field("globalId", &info.clone().and_then(|info| Some(info.id)))?;
            s.end()
        }
        None => s.serialize_none(),
    }
}

impl Serialize for TilingState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TilingState::FloatingOnly => serializer.serialize_str("FloatingOnly"),
            TilingState::TilingEnabled => serializer.serialize_str("TilingEnabled"),
        }
    }
}

impl Display for WorkspaceStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer_strict(self, f)
    }
}
impl Display for WorkspaceCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer_strict(self, f)
    }
}
impl Display for GroupCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer_strict(self, f)
    }
}
impl Display for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: \"{}\", id: {}, coordinates: {:?}, state: [{}], capabilities: [{}]{}",
            self.name.clone().unwrap_or("".to_string()),
            self.id.clone().unwrap_or("".to_string()),
            self.coordinates,
            self.state,
            self.capabilities,
            self.tiling_state
                .map_or("".to_string(), |t| format!(", tiling_state: {:?}", t))
        )
    }
}

impl Display for WorkspaceState {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for group in self.groups.iter() {
            writeln!(out, "{}", group)?;
            for workspace in self
                .workspaces
                .iter()
                .filter(|ws| ws.group.as_ref().is_some_and(|g| g == &group.handle))
            {
                writeln!(out, "    {}", workspace)?;
            }
        }

        let unassigned_ws = self
            .workspaces
            .iter()
            .filter(|ws| ws.group.is_none())
            .collect::<Vec<_>>();
        if !unassigned_ws.is_empty() {
            writeln!(out, "workspaces without assigned workspace group")?;
            for workspace in unassigned_ws {
                writeln!(out, "    {}", workspace)?;
            }
        }
        Ok(())
    }
}

impl Display for WorkspaceGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.output.is_none() { return write!(f, "workspace group without assigned output")};
        let output_info = self.get_output_info();
        write!(
            f,
            "name: \"{}\", capabilities: [{}], location: {}, size: {}, description: {}",
            self.get_output_name().unwrap_or("".to_string()),
            self.capabilities,
            output_info.as_ref().map_or("(, )".to_string(), |info| format!("({}, {})", info.location.0, info.location.1)),
            output_info.as_ref().map_or("(, )".to_string(), |info| format!("({}, {})", info.physical_size.0, info.physical_size.1)),
            output_info.as_ref().map_or("".to_string(), |info| info.description.clone().unwrap_or("".to_string())),
        )
    }
}
