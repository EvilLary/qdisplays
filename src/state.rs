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

    pub fn apply_configuration(&mut self, conn: &mut Connection<Self>, mut confs: Vec<(u32, PendingConfig)>) {
        let wlr_config = self.manager.create_configuration_with_cb(conn, self.serial, Self::on_config_event);

        // idk but the proto says no head to be omitted in configuration,
        // but no compositor sends an error if I do that
        for h in self.heads.values() {
            let c = confs.iter_mut().find(|(id, _)| h.wlr.id().as_u32() == *id).map(|c| &mut c.1);

            if !c.as_ref().and_then(|c| c.enabled).unwrap_or(h.enabled) {
                wlr_config.disable_head(conn, h.wlr);
                continue;
            }

            let head_config = wlr_config.enable_head(conn, h.wlr);
            let Some(c) = c else {
                continue;
            };

            if let Some(v) = c.transform.take() {
                head_config.set_transform(conn, Transform::try_from(v).unwrap());
            }

            if let Some(v) = c.scale.take() {
                head_config.set_scale(conn, v.into());
            }

            if let Some(v) = c.position.take() {
                head_config.set_position(conn, v.x(), v.y());
            }

            if let Some(v) = c.adaptive_sync.take() {
                let v = match v {
                    true => zwlr_output_head_v1::AdaptiveSyncState::Enabled,
                    false => zwlr_output_head_v1::AdaptiveSyncState::Disabled,
                };

                head_config.set_adaptive_sync(conn, v);
            }

            if let Some(w) = h.modes.iter().find(|m| Some(m.wlr.id().as_u32()) == c.mode).map(|m| &m.wlr) {
                head_config.set_mode(conn, *w)
            }
        }

        wlr_config.apply(conn);
    }
}

impl WlState {
    pub fn on_config_event(ctx: EventCtx<Self, ZwlrOutputConfigurationV1>) {
        let EventCtx { conn, proxy, event, .. } = ctx;

        proxy.destroy(conn);
        match event {
            zwlr_output_configuration_v1::Event::Succeeded => qt::wayland_message(WlMsg::ConfigApplied),
            zwlr_output_configuration_v1::Event::Failed => qt::wayland_message(WlMsg::ConfigFailed),
            // FIXME: should retry
            zwlr_output_configuration_v1::Event::Cancelled => qt::wayland_message(WlMsg::ConfigCancelled),
            _ => {},
        }
    }

    pub fn on_manager_event(ctx: EventCtx<Self, ZwlrOutputManagerV1>) {
        let EventCtx { conn, state, event, .. } = ctx;

        match event {
            zwlr_output_manager_v1::Event::Head(head) => {
                conn.set_callback_for(head, WlState::on_head_event);
                state.heads.insert(head.id().as_u32(), WlrHead::new(head));
            },

            zwlr_output_manager_v1::Event::Done(serial) => {
                state.serial = serial;

                state.heads.retain(|&id, h| {
                    h.on_done();

                    match h.state {
                        HeadState::NotReady => {
                            h.state = HeadState::Ready;
                            qt::add_head(h.into());
                            true
                        },
                        HeadState::Updated => {
                            h.state = HeadState::Ready;
                            qt::update_head(h.get_update());
                            true
                        },
                        HeadState::Removed => {
                            for m in &h.modes {
                                state.modes_map.remove(&m.id());
                                m.wlr.release(conn);
                            }

                            qt::remove_head(id);
                            false
                        },
                        HeadState::Ready => true,
                    }
                });
            },

            zwlr_output_manager_v1::Event::Finished => {
                for h in state.heads.values() {
                    for m in &h.modes {
                        m.wlr.release(conn);
                    }
                    h.wlr.release(conn);
                }

                // tbh I don't know if this could be sent without manually calling 'stop'
                qt::wayland_message(WlMsg::Fatal(QString::from("zwlr_output_manager_v1 destroyed by the compositor")))
            },
            _ => {},
        }
    }
}

use std::collections::HashMap;

use cxx_qt_lib::QString;
use wayrs_client::object::Proxy;
use wayrs_client::protocol::wl_output::Transform;
use wayrs_client::{Connection, EventCtx};

use wayrs_protocols::wlr_output_management_unstable_v1::zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1};
use wayrs_protocols::wlr_output_management_unstable_v1::zwlr_output_head_v1;
use wayrs_protocols::wlr_output_management_unstable_v1::zwlr_output_manager_v1::{self, ZwlrOutputManagerV1};

use crate::head::{HeadState, PendingConfig, WlrHead};
use crate::qt;
use crate::wayland::WlMsg;
