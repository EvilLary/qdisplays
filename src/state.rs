pub struct WlState {
    pub manager: ZwlrOutputManagerV1,
    pub heads: HashMap<u32, WlrHead>,
    pub modes_map: HashMap<u32, u32>,
    pub serial: u32,
}

impl WlState {
    pub fn new(manager: ZwlrOutputManagerV1) -> Self {
        Self {
            manager,
            heads: Default::default(),
            modes_map: Default::default(),
            serial: 0,
        }
    }

    pub fn apply_configuration(&mut self, qh: QueueHandle<Self>, mut confs: Vec<(u32, PendingConfig)>) {
        let wlr_config = self.manager.create_configuration(self.serial, &qh, ());

        // idk but the proto says no head to be omitted in configuration,
        // but no compositor sends an error if I do that
        for h in self.heads.values() {
            let c = confs.iter_mut().find(|(id, _)| h.wlr.id().protocol_id() == *id).map(|c| &mut c.1);

            if !c.as_ref().and_then(|c| c.enabled).unwrap_or(h.enabled) {
                wlr_config.disable_head(&h.wlr);
                continue;
            }

            let head_config = wlr_config.enable_head(&h.wlr, &qh, ());
            let Some(c) = c else {
                continue;
            };

            if let Some(v) = c.transform.take() {
                head_config.set_transform(wl_output::Transform::try_from(v).unwrap());
            }

            if let Some(v) = c.scale.take() {
                head_config.set_scale(v);
            }

            if let Some(v) = c.position.take() {
                head_config.set_position(v.x(), v.y());
            }

            if let Some(v) = c.adaptive_sync.take() {
                let v = match v {
                    true => zwlr_output_head_v1::AdaptiveSyncState::Enabled,
                    false => zwlr_output_head_v1::AdaptiveSyncState::Disabled,
                };

                head_config.set_adaptive_sync(v);
            }

            if let Some(v) = c.mode.take() {
                h.modes
                    .iter()
                    .find_map(|m| (m.wlr.id().protocol_id() == v).then(|| &m.wlr))
                    .map(|w| head_config.set_mode(w));
            }
        }

        wlr_config.apply();
    }
}

wayland_client::delegate_noop!(WlState: ZwlrOutputConfigurationHeadV1);

impl Dispatch<ZwlrOutputConfigurationV1, ()> for WlState {
    fn event(
        _this: &mut Self,
        wlr_config: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        wlr_config.destroy();

        match event {
            zwlr_output_configuration_v1::Event::Succeeded => qt::wayland_message(WlMsg::ConfigApplied),
            zwlr_output_configuration_v1::Event::Failed => qt::wayland_message(WlMsg::ConfigFailed),
            // FIXME: should retry
            zwlr_output_configuration_v1::Event::Cancelled => qt::wayland_message(WlMsg::ConfigCancelled),
            _ => {},
        }
    }
}
impl Dispatch<ZwlrOutputManagerV1, ()> for WlState {
    fn event(
        this: &mut Self,
        _wlr_mgr: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                this.heads.insert(head.id().protocol_id(), WlrHead::new(head));
            },

            zwlr_output_manager_v1::Event::Done { serial } => {
                this.serial = serial;

                this.heads.retain(|&id, h| {
                    h.on_done();

                    match h.state {
                        HeadState::NotReady => {
                            h.state = HeadState::Ready;
                            qt::add_head(h.into());
                            true
                        },
                        HeadState::Updated => {
                            h.state = HeadState::Ready;
                            qt::update_head(h.into_update());
                            true
                        },
                        HeadState::Removed => {
                            for m in &h.modes {
                                this.modes_map.remove(&m.id());
                                m.wlr.release();
                            }

                            qt::remove_head(id);
                            false
                        },
                        HeadState::Ready => true,
                    }
                });
            },

            zwlr_output_manager_v1::Event::Finished => {
                for h in this.heads.values() {
                    for m in &h.modes {
                        m.wlr.release();
                    }
                    h.wlr.release();
                }

                // tbh I don't know if this could be sent without manually calling 'stop'
                qt::wayland_message(WlMsg::Fatal(QString::from("zwlr_output_manager_v1 destroyed by the compositor")))
            },
            _ => {},
        }
    }

    wayland_client::event_created_child!(WlState, ZwlrOutputManagerV1, [
        zwlr_output_manager_v1::EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<WlRegistry, GlobalListContents> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

use std::collections::HashMap;

use cxx_qt_lib::QString;
use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_output;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::{self, ZwlrOutputHeadV1};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::{self, ZwlrOutputManagerV1};

use crate::head::{WlrHead, HeadState, PendingConfig};
use crate::qt;
use crate::wayland::WlMsg;
