#[derive(Default)]
pub struct QMode {
    pub id: u32,
    pub width: i32,
    pub height: i32,
}

pub struct WlrMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
    pub wlr: ZwlrOutputModeV1,
}

// fucking retarded rust, can't auto impl for T, &T & &mut T
impl Into<QMode> for &WlrMode {
    fn into(self) -> QMode {
        QMode {
            id: self.id(),
            width: self.width,
            height: self.height,
        }
    }
}

impl WlrMode {
    pub fn id(&self) -> u32 {
        self.wlr.id().protocol_id()
    }

    pub fn new(wlr: ZwlrOutputModeV1) -> Self {
        Self {
            wlr,
            width: -1,
            height: -1,
            refresh: Default::default(),
            preferred: Default::default(),
        }
    }
}

impl Dispatch<ZwlrOutputModeV1, ()> for WlState {
    fn event(this: &mut Self, wlr_mode: &ZwlrOutputModeV1, event: <ZwlrOutputModeV1 as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        let mode_id = wlr_mode.id().protocol_id();
        let Some(head) = this.modes_map.get(&mode_id).and_then(|i| this.heads.get_mut(i)) else {
            return;
        };

        let Some((idx, mode)) = head.modes.iter_mut().enumerate().find(|(_, m)| m.id() == mode_id) else {
            return;
        };

        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                mode.width = width;
                mode.height = height;
                head.modes_changed = true;
                // I could check for mode_id against current_mode,
                // but what if this event was emitted before Head::CurrentMode
                head.recalc_geometry = true;
            },
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                mode.refresh = refresh;
                head.modes_changed = true;
            },
            zwlr_output_mode_v1::Event::Preferred => {
                mode.preferred = true;
            },
            zwlr_output_mode_v1::Event::Finished => {
                if head.current_mode == mode_id {
                    head.recheck_mode_index = true;
                    head.recalc_geometry = true;
                }

                this.modes_map.remove(&mode_id);
                head.modes.swap_remove(idx);
                head.modes_changed = true;
                wlr_mode.release();
            },
            _ => {},
        }

        if head.state == HeadState::Ready {
            head.state = HeadState::Updated;
        }
    }
}

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::{self, ZwlrOutputModeV1};

use crate::head::HeadState;
use crate::state::WlState;
