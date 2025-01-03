use log::{debug, warn};
use smithay_client_toolkit::globals::GlobalData;
use wayland_client::{Dispatch, Proxy, WEnum};

use crate::{
    ext::workspace::cosmic_v1::client::{
        zcosmic_workspace_group_handle_v1::{self, ZcosmicWorkspaceGroupHandleV1},
        zcosmic_workspace_handle_v1::{self, ZcosmicWorkspaceHandleV1},
        zcosmic_workspace_manager_v1::{Event, ZcosmicWorkspaceManagerV1},
    },
    workspace_state::{
        GroupCapabilities, GroupHandle, WorkspaceState, WorkspaceCapabilities, WorkspaceDispatch,
        WorkspaceEvent, WorkspaceHandle, WorkspaceStates,
    },
};

impl<D: WorkspaceDispatch> Dispatch<ZcosmicWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZcosmicWorkspaceManagerV1,
        event: <ZcosmicWorkspaceManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        debug!(
            "manager: {:?}, event: {:?}",
            handle.id().protocol_id(),
            event
        );
        let event = match event {
            Event::WorkspaceGroup { workspace_group } => 
                WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::CosmicV1(workspace_group)),
            Event::Done {} => {
                state.workspace_state_mut().handle_events();
                return
            }
            Event::Finished {} => WorkspaceEvent::ManagerFinished,
        };
        state
            .workspace_state_mut()
            .events
            .push(event);
    }

    wayland_client::event_created_child!(D, ZcosmicWorkspaceManagerV1, [
        0 => (ZcosmicWorkspaceGroupHandleV1, ()),
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
        debug!("group: {:?}, event: {:?}", handle.id().protocol_id(), event);
        use zcosmic_workspace_group_handle_v1::Event;
        let event = match event {
            Event::OutputEnter { output } => {
                WorkspaceEvent::OutputEnter(GroupHandle::CosmicV1(handle.clone()), output)
            }
            Event::OutputLeave { output } => {
                WorkspaceEvent::OutputLeave(GroupHandle::CosmicV1(handle.clone()), output)
            }
            Event::Remove => {
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::CosmicV1(handle.clone()))
            }
            Event::Capabilities { capabilities } => {
                let mut caps = GroupCapabilities::empty();
                for bits in capabilities {
                    caps.insert(GroupCapabilities::from_bits_retain(bits as u32));
                }
                WorkspaceEvent::WorkspaceGroupCapabilities(
                    GroupHandle::CosmicV1(handle.clone()),
                    caps,
                )
            }
            Event::Workspace { workspace } => WorkspaceEvent::WorkspaceCreated(
                Some(GroupHandle::CosmicV1(handle.clone())),
                WorkspaceHandle::CosmicV1(workspace),
            ),
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
        debug!(
            "workspace: {:?}, event: {:?}",
            handle.id().protocol_id(),
            event
        );
        use zcosmic_workspace_handle_v1::Event;
        let event = match event {
            Event::State { state } => {
                if state.len() != 4 {
                    return;
                };
                let bits = u32::from_ne_bytes(state.chunks(4).next().unwrap().try_into().unwrap());
                WorkspaceEvent::WorkspaceState(
                    WorkspaceHandle::CosmicV1(handle.clone()),
                    //WorkspaceStates::from_bits_retain(bits).complement(),
                    WorkspaceStates::from_bits_retain(bits),
                )
            }
            Event::Name { name } => {
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::CosmicV1(handle.clone()), name)
            }
            Event::Coordinates { coordinates } => WorkspaceEvent::WorkspaceCoord(
                WorkspaceHandle::CosmicV1(handle.clone()),
                coordinates,
            ),
            Event::Remove => {
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::CosmicV1(handle.clone()))
            }
            Event::Capabilities { capabilities } => {
                let mut caps = WorkspaceCapabilities::empty();
                for bits in capabilities.iter() {
                    let mut bits = bits.clone();
                    if [8, 16].contains(&bits) {
                        bits = bits.rotate_left(1);
                    }
                    caps.insert(WorkspaceCapabilities::from_bits_retain(bits as u32));
                }
                WorkspaceEvent::WorkspaceCapabilities(
                    WorkspaceHandle::CosmicV1(handle.clone()),
                    caps,
                )
            }
            Event::TilingState { state } => match state {
                WEnum::Value(state) => WorkspaceEvent::WorkspaceTilingState(
                    WorkspaceHandle::CosmicV1(handle.clone()),
                    state,
                ),
                WEnum::Unknown(unknown) => {
                    warn!("tiling_state event with unkown value {:?}", unknown);
                    return;
                }
            },
        };
        state.workspace_state_mut().events.push(event);
    }
}

#[macro_export]
macro_rules! delegate_workspace_cosmic_v1 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
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
