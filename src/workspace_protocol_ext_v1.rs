use log::{debug, warn};
use smithay_client_toolkit::globals::GlobalData;
use wayland_client::{Dispatch, Proxy, WEnum};

use crate::{
    ext::workspace::ext_v1::client::{
        ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
        ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1},
        ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
    },
    workspace_state::{
        GroupCapabilities, GroupHandle, WorkspaceCapabilities, WorkspaceDispatch, WorkspaceEvent, WorkspaceHandle, WorkspaceState, WorkspaceStates
    },
};

impl<D: WorkspaceDispatch> Dispatch<ExtWorkspaceManagerV1, GlobalData, D> for WorkspaceState {
    fn event(
        state: &mut D,
        handle: &ExtWorkspaceManagerV1,
        event: <ExtWorkspaceManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<D>,
    ) {
        debug!(
            "manager: {:?}, event: {:?}",
            handle.id().protocol_id(),
            event
        );
        use ext_workspace_manager_v1::Event;
        match event {
            Event::WorkspaceGroup { workspace_group } => {
                state
                    .workspace_state_mut()
                    .events
                    .push(WorkspaceEvent::WorkspaceGroupCreated(GroupHandle::ExtV1(
                        workspace_group,
                    )));
            }
            Event::Done {} => {
                state.workspace_state_mut().handle_events();
                return
            }
            Event::Finished {} => {
                // todo handle event
            }
            Event::Workspace { workspace } => {
                state
                    .workspace_state_mut()
                    .events
                    .push(WorkspaceEvent::WorkspaceCreated(
                        None,
                        WorkspaceHandle::ExtV1(workspace),
                    ))
            }
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
        debug!("group: {:?}, event: {:?}", handle.id().protocol_id(), event);
        let event = match event {
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                WorkspaceEvent::OutputEnter(GroupHandle::ExtV1(handle.clone()), output)
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                WorkspaceEvent::OutputLeave(GroupHandle::ExtV1(handle.clone()), output)
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                WorkspaceEvent::WorkspaceGroupRemoved(GroupHandle::ExtV1(handle.clone()))
            }
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                match capabilities {
                    WEnum::Value(ext_caps) => {
                        if let Some(caps) = GroupCapabilities::from_bits(ext_caps.bits()) {
                            WorkspaceEvent::WorkspaceGroupCapabilities(
                                GroupHandle::ExtV1(handle.clone()),
                                caps,
                            )
                        } else {
                            warn!("group_capabilities event with unexpected value: {ext_caps:?}");
                            return;
                        }
                    }
                    WEnum::Unknown(unknown) => {
                        warn!("group_capabilities event with unknown value: {unknown}");
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
        debug!(
            "workspace: {:?}, event: {:?}",
            handle.id().protocol_id(),
            event
        );
        let event = match event {
            ext_workspace_handle_v1::Event::Id { id } => {
                WorkspaceEvent::WorkspaceId(WorkspaceHandle::ExtV1(handle.clone()), id)
            },
            ext_workspace_handle_v1::Event::State { state } => match state {
                WEnum::Value(s) => WorkspaceEvent::WorkspaceState(
                    WorkspaceHandle::ExtV1(handle.clone()),
                    WorkspaceStates::from_bits_retain(s.bits()),
                ),
                WEnum::Unknown(unknown) => {
                    warn!("workspace_state event with unknown value: {unknown}");
                    return;
                }
            },
            ext_workspace_handle_v1::Event::Name { name } => {
                WorkspaceEvent::WorkspaceName(WorkspaceHandle::ExtV1(handle.clone()), name)
            }
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                WorkspaceEvent::WorkspaceCoord(WorkspaceHandle::ExtV1(handle.clone()), coordinates)
            }
            ext_workspace_handle_v1::Event::Removed => {
                WorkspaceEvent::WorkspaceRemoved(WorkspaceHandle::ExtV1(handle.clone()))
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => match capabilities {
                WEnum::Value(ext_caps) => {
                    if let Some(caps) = WorkspaceCapabilities::from_bits(ext_caps.bits()) {
                        WorkspaceEvent::WorkspaceCapabilities(
                            WorkspaceHandle::ExtV1(handle.clone()),
                            caps,
                        )
                    } else {
                        warn!("workspace_capabilities event with unknown bits: {ext_caps:?}");
                        return;
                    }
                }
                WEnum::Unknown(unknown) => {
                    warn!("workspace_capabilities event with unknown value: {unknown}");
                    return;
                }
            },
        };
        state.workspace_state_mut().events.push(event);
    }
}

#[macro_export]
macro_rules! delegate_workspace_ext_v1 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ext::workspace::ext_v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: ()
        ] => $crate::workspace_state::WorkspaceState);
    };
}
