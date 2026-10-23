#[derive(Default)]
pub struct QHead {
    pub id: u32,
    pub name: QString,
    pub description: QString,
    pub make: QString,
    pub model: QString,
    pub serial_number: QString,
    pub physical_size: QSize,

    pub enabled: bool,
    pub position: QPoint,
    pub geometry: QRect,
    pub transform: u32,
    pub scale: f64,
    pub adaptive_sync: bool,

    pub current_mode: u32,
    pub current_mode_index: i32,
    // FIXME: use a model for both of these
    pub modes: Vec<QMode>,
    pub modes_strings: QList<QString>,

    pub pending_config: PendingConfig,
}

impl QHead {
    pub fn set_mode(&mut self, v: &QVariant) -> bool {
        let Some(v) = v.value::<i32>() else {
            return false;
        };

        let Some(mode) = self.modes.get(v as usize) else {
            return false;
        };

        self.pending_config.mode = (v != self.current_mode_index).then_some(mode.id);
        true
    }

    pub fn set_enabled(&mut self, v: &QVariant) -> bool {
        let Some(v) = v.value::<bool>() else {
            return false;
        };

        self.pending_config.enabled = (v != self.enabled).then_some(v);
        true
    }

    pub fn set_position(&mut self, v: &QVariant) -> bool {
        // I could also check for QPointF
        let Some(v) = v.value::<QPoint>() else {
            return false;
        };

        self.pending_config.position = (v != self.position).then_some(v);
        true
    }

    pub fn set_transform(&mut self, v: &QVariant) -> bool {
        let Some(v @ 0..=7) = v.value::<u32>() else {
            return false;
        };

        self.pending_config.transform = (v != self.transform).then_some(v);
        true
    }

    pub fn set_scale(&mut self, v: &QVariant) -> bool {
        // just random numbers
        let Some(v @ 0.5..=2.0) = v.value::<f64>() else {
            return false;
        };

        self.pending_config.scale = (v != self.scale).then_some(v);
        true
    }

    pub fn set_adaptive_sync(&mut self, v: &QVariant) -> bool {
        let Some(v) = v.value::<bool>() else {
            return false;
        };

        self.pending_config.adaptive_sync = (v != self.adaptive_sync).then_some(v);
        true
    }

    #[inline]
    pub fn current_mode(&self) -> Option<(usize, &QMode)> {
        let idx = self.current_mode_index as usize;
        self.pending_config
            .mode
            .and_then(|id| self.modes.iter().enumerate().find(|m| m.1.id == id))
            .or_else(|| self.modes.get(idx).and_then(|m| Some((idx, m))))
    }

    #[inline]
    pub fn position(&self) -> QPoint {
        self.pending_config.position.as_ref().unwrap_or(&self.position).clone()
    }

    #[inline]
    pub fn enabled(&self) -> bool {
        self.pending_config.enabled.unwrap_or(self.enabled)
    }

    #[inline]
    pub fn scale(&self) -> f64 {
        self.pending_config.scale.unwrap_or(self.scale)
    }

    #[inline]
    pub fn transform(&self) -> u32 {
        self.pending_config.transform.unwrap_or(self.transform)
    }

    #[inline]
    pub fn adaptive_sync(&self) -> bool {
        self.pending_config.adaptive_sync.unwrap_or(self.adaptive_sync)
    }

    pub fn geometry(&self) -> QRect {
        if self.pending_config.is_empty() {
            return self.geometry.clone();
        }

        let Some((_, mode)) = self.current_mode() else {
            return self.geometry.clone();
        };

        let (w, h) = match self.transform() {
            t if t % 2 == 0 => (mode.width, mode.height),
            _ => (mode.height, mode.width),
        };

        let scale = self.scale();
        let pos = self.position();
        let h = ((h as f64) / scale).round() as i32;
        let w = ((w as f64) / scale).round() as i32;

        QRect::new(pos.x(), pos.y(), w, h)
    }
}

#[derive(Default, Clone)]
pub struct PendingConfig {
    pub enabled: Option<bool>,
    pub mode: Option<u32>,
    pub position: Option<QPoint>,
    pub scale: Option<f64>,
    pub transform: Option<u32>,
    pub adaptive_sync: Option<bool>,
}

impl PendingConfig {
    pub fn is_empty(&self) -> bool {
        self.mode.is_none() && self.position.is_none() && self.transform.is_none() && self.scale.is_none() && self.enabled.is_none() && self.adaptive_sync.is_none()
    }

    pub fn clear(&mut self) {
        self.enabled = None;
        self.mode = None;
        self.position = None;
        self.scale = None;
        self.transform = None;
        self.adaptive_sync = None;
    }
}

pub struct HeadUpdate {
    pub id: u32,
    pub enabled: bool,
    pub position: QPoint,
    pub geometry: QRect,
    pub transform: u32,
    pub scale: f64,
    pub adaptive_sync: bool,

    pub current_mode_index: i32,
    pub modes: Option<(Vec<QMode>, QList<QString>)>,
}

#[derive(Default, PartialEq, Eq)]
pub enum HeadState {
    #[default]
    NotReady,
    Ready,
    Updated,
    Removed,
}

pub struct WlrHead {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial_number: String,
    pub physical_size: QSize,

    pub enabled: bool,
    pub current_mode: u32,
    pub current_mode_index: i32,
    pub position: QPoint,
    pub transform: u32,
    pub scale: f64,
    pub adaptive_sync: bool,
    pub modes: Vec<WlrMode>,
    pub geometry: QRect,

    pub state: HeadState,
    pub recheck_mode_index: bool,
    pub recalc_geometry: bool,
    pub modes_changed: bool,

    pub wlr: ZwlrOutputHeadV1,
}

impl WlrHead {
    pub fn builds_qmodes(&self) -> (Vec<QMode>, QList<QString>) {
        use std::fmt::Write;
        let mut string_list = QList::default();
        string_list.reserve(self.modes.len() as isize);

        let mut buf = String::with_capacity(16);
        for m in &self.modes {
            buf.clear();
            let _ = write!(buf, "{}x{}@{:.2}Hz", m.width, m.height, (m.refresh as f32) / 1000.0);
            string_list.append(QString::from(&buf));
        }

        (self.modes.iter().map(Into::into).collect(), string_list)
    }

    pub fn on_done(&mut self) {
        if self.recalc_geometry {
            self.calc_geometry();
        }
        if self.recheck_mode_index {
            self.recheck_mode_index = false;
            self.current_mode_index = self.current_mode().map(|(i, _)| i as i32).unwrap_or(-1);
        }
    }

    pub fn into_update(&mut self) -> HeadUpdate {
        let modes = if self.modes_changed {
            self.modes_changed = false;
            Some(self.builds_qmodes())
        } else {
            None
        };

        HeadUpdate {
            id: self.wlr.id().protocol_id(),
            enabled: self.enabled,
            position: self.position.clone(),
            geometry: self.geometry.clone(),
            transform: self.transform,
            scale: self.scale,
            adaptive_sync: self.adaptive_sync,
            current_mode_index: self.current_mode_index,
            modes,
        }
    }

    pub fn new(wlr: ZwlrOutputHeadV1) -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            make: Default::default(),
            model: Default::default(),
            serial_number: Default::default(),
            physical_size: Default::default(),
            enabled: Default::default(),
            current_mode: Default::default(),
            current_mode_index: Default::default(),
            position: Default::default(),
            transform: Default::default(),
            scale: Default::default(),
            adaptive_sync: Default::default(),
            modes: Default::default(),
            geometry: Default::default(),
            state: Default::default(),
            recheck_mode_index: Default::default(),
            recalc_geometry: Default::default(),
            modes_changed: Default::default(),
            wlr,
        }
    }

    pub fn current_mode(&self) -> Option<(usize, &WlrMode)> {
        self.modes.iter().enumerate().find(|(_, m)| m.id() == self.current_mode)
    }

    pub fn calc_geometry(&mut self) {
        self.recalc_geometry = false;
        let Some((_, mode)) = self.current_mode() else {
            return;
        };

        // 1 3 5 7 transforms are flipped
        let (w, h) = match self.transform {
            t if t % 2 == 0 => (mode.width, mode.height),
            _ => (mode.height, mode.width),
        };

        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        let h = ((h as f64) / scale).round() as i32;
        let w = ((w as f64) / scale).round() as i32;

        self.geometry = QRect::new(self.position.x(), self.position.y(), w, h);
    }
}

impl Into<QHead> for &mut WlrHead {
    fn into(self) -> QHead {
        <&WlrHead as Into<QHead>>::into(self)
    }
}

impl Into<QHead> for &WlrHead {
    fn into(self) -> QHead {
        let (modes, modes_strings) = self.builds_qmodes();
        QHead {
            id: self.wlr.id().protocol_id(),
            name: QString::from(&self.name),
            description: QString::from(&self.description),
            make: QString::from(&self.make),
            model: QString::from(&self.model),
            serial_number: QString::from(&self.serial_number),
            physical_size: self.physical_size.clone(),
            enabled: self.enabled,
            position: self.position.clone(),
            geometry: self.geometry.clone(),
            transform: self.transform,
            scale: self.scale,
            adaptive_sync: self.adaptive_sync,
            current_mode: self.current_mode,
            current_mode_index: self.current_mode_index,
            modes,
            modes_strings,
            pending_config: Default::default(),
        }
    }
}

impl Dispatch<ZwlrOutputHeadV1, ()> for WlState {
    fn event(this: &mut Self, wlr_head: &ZwlrOutputHeadV1, event: <ZwlrOutputHeadV1 as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        let Some(head) = this.heads.get_mut(&wlr_head.id().protocol_id()) else {
            return;
        };

        if matches!(event, zwlr_output_head_v1::Event::Finished) {
            head.state = HeadState::Removed;
            wlr_head.release();
            return;
        }

        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                head.name = name;
            },
            zwlr_output_head_v1::Event::Description { description } => {
                head.description = description;
            },
            zwlr_output_head_v1::Event::PhysicalSize { width, height } => {
                head.physical_size = QSize::new(width, height);
            },
            zwlr_output_head_v1::Event::Make { make } => {
                head.make = make;
            },
            zwlr_output_head_v1::Event::Model { model } => {
                head.model = model;
            },
            zwlr_output_head_v1::Event::SerialNumber { serial_number } => {
                head.serial_number = serial_number;
            },
            zwlr_output_head_v1::Event::Mode { mode } => {
                this.modes_map.insert(mode.id().protocol_id(), wlr_head.id().protocol_id());
                head.modes_changed = true;
                head.recheck_mode_index = true;
                head.modes.push(WlrMode::new(mode));
            },
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                head.enabled = enabled == 1;
            },
            zwlr_output_head_v1::Event::CurrentMode { mode } => {
                head.recalc_geometry = true;
                head.recheck_mode_index = true;
                head.current_mode = mode.id().protocol_id();
            },
            zwlr_output_head_v1::Event::Position { x, y } => {
                head.recalc_geometry = true;
                head.position = QPoint::new(x, y);
            },
            zwlr_output_head_v1::Event::Transform { transform } => {
                let WEnum::Value(transform) = transform else {
                    return;
                };
                head.recalc_geometry = true;
                head.transform = transform as u32;
            },
            zwlr_output_head_v1::Event::Scale { scale } => {
                head.recalc_geometry = true;
                head.scale = scale;
            },
            zwlr_output_head_v1::Event::AdaptiveSync { state } => {
                head.adaptive_sync = state == WEnum::Value(zwlr_output_head_v1::AdaptiveSyncState::Enabled);
            },
            _ => {},
        }

        if head.state == HeadState::Ready {
            head.state = HeadState::Updated
        }
    }

    wayland_client::event_created_child!(WlState, ZwlrOutputHeadV1, [
        zwlr_output_head_v1::EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

use cxx_qt_lib::{QList, QPoint, QRect, QSize, QString, QVariant};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::{self, ZwlrOutputHeadV1};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use crate::mode::{QMode, WlrMode};
use crate::state::WlState;
