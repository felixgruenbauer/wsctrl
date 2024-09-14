use log::debug;
use smithay_client_toolkit::globals::GlobalData;
use wayland_client::{Dispatch, Proxy};

use crate::{
    cli::WorkspaceArgs, ext::workspace::{
        ext_v0::client::{
            zext_workspace_group_handle_v1::{self, ZextWorkspaceGroupHandleV1},
            zext_workspace_handle_v1::{self, ZextWorkspaceHandleV1},
            zext_workspace_manager_v1::{self, ZextWorkspaceManagerV1},
        },
        ext_v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    }, workspace_state::{
        GroupHandle, WorkspaceDispatch, WorkspaceEvent, WorkspaceHandle,
        WorkspaceState, WorkspaceStates,
    }
};

#[derive(Clone)]
struct GroupHandleExtV0 {
    handle: ExtWorkspaceGroupHandleV1,
}

impl<D: WorkspaceDispatch> Dispatch<ZextWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ZextWorkspaceManagerV1,
        event: <ZextWorkspaceManagerV1 as wayland_client::Proxy>::Event,
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
            zext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::ExtV0(workspace_group))
            }
            zext_workspace_manager_v1::Event::Done {} => {
                state.workspace_state_mut().handle_events();
                return;
            }
            zext_workspace_manager_v1::Event::Finished {} => 
                WorkspaceEvent::ManagerFinished
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
        debug!("group: {:?}, event: {:?}", handle.id().protocol_id(), event);
        let event = match event {
            zext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                WorkspaceEvent::OutputEnter(GroupHandle::ExtV0(handle.clone()), output)
            }
            zext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                WorkspaceEvent::OutputLeave(GroupHandle::ExtV0(handle.clone()), output)
            }
            zext_workspace_group_handle_v1::Event::Remove => {
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::ExtV0(handle.clone()))
            }
            zext_workspace_group_handle_v1::Event::Workspace { workspace } => {
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
        debug!(
            "workspace: {:?}, event: {:?}",
            handle.id().protocol_id(),
            event
        );
        let event = match event {
            zext_workspace_handle_v1::Event::State { state } => {
                if state.len() != 4 {
                    return;
                };
                let bits = u32::from_ne_bytes(state.chunks(4).next().unwrap().try_into().unwrap());
                WorkspaceEvent::WorkspaceState(
                    WorkspaceHandle::ExtV0(handle.clone()),
                    WorkspaceStates(bits).symmetric_difference(WorkspaceStates(7)),
                )
            }
            zext_workspace_handle_v1::Event::Name { name } => {
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::ExtV0(handle.clone()), name)
            }
            zext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                WorkspaceEvent::WorkspaceCoord(WorkspaceHandle::ExtV0(handle.clone()), coordinates)
            }
            zext_workspace_handle_v1::Event::Remove => {
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::ExtV0(handle.clone()))
            }
        };
        state.workspace_state_mut().events.push(event);
    }
}

#[macro_export]
macro_rules! delegate_workspace_ext_v0 {
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
    };
}
