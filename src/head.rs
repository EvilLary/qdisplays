fn calc_geometry(size: QSize, pos: QPoint, scale: f64, transform: u32) -> QRect {
    let (w, h) = match transform {
        t if t % 2 == 0 => (size.width(), size.height()),
        _ => (size.height(), size.width()),
    };

    let h = ((h as f64) / scale).round() as i32;
    let w = ((w as f64) / scale).round() as i32;

    QRect::new(pos.x(), pos.y(), w, h)
}

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
    pub modes: UniquePtr<ModesModel>,
    pub pending_config: PendingConfig,
}

impl QHead {
    pub fn modes_qobject(&self) -> QObjectMutPtr {
        unsafe { QObjectMutPtr::from_raw(self.modes.as_mut_ptr().cast()) }
    }

    pub fn set_modes(&mut self, modes: Vec<QMode>) {
        self.modes.as_mut().unwrap().set_modes(modes)
    }

    pub fn set_mode(&mut self, v: &QVariant) -> bool {
        let Some(v) = v.value::<i32>() else {
            return false;
        };

        let Some(mode) = self.modes().get(v as usize) else {
            return false;
        };

        self.pending_config.mode = (mode.id != self.current_mode).then_some(mode.id);
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

    pub fn modes(&self) -> &[QMode] {
        &self.modes.as_ref().unwrap().modes
    }

    #[inline]
    pub fn current_mode(&self) -> Option<(usize, &QMode)> {
        let idx = self.current_mode_index as usize;
        let modes = self.modes();
        self.pending_config
            .mode
            .and_then(|id| modes.iter().enumerate().find(|m| m.1.id == id))
            .or_else(|| modes.get(idx).map(|m| (idx, m)))
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

        calc_geometry(mode.size.clone(), self.position(), self.scale(), self.transform())
    }
}

// SAFETY: idk
unsafe impl Send for QHead {}

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
    pub modes: Option<Vec<QMode>>,
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
    pub name: Box<CStr>,
    pub description: Box<CStr>,
    pub make: Box<CStr>,
    pub model: Box<CStr>,
    pub serial_number: Box<CStr>,
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
    pub mode_index_changed: bool,
    pub geometry_changed: bool,
    pub modes_changed: bool,

    pub wlr: ZwlrOutputHeadV1,
}

impl WlrHead {
    pub fn on_done(&mut self) {
        if self.geometry_changed {
            self.recalc_geometry();
        }

        if self.mode_index_changed {
            self.mode_index_changed = false;
            self.current_mode_index = self.current_mode().map(|(i, _)| i as i32).unwrap_or(-1);
        }
    }

    pub fn get_update(&mut self) -> HeadUpdate {
        let modes = self.modes_changed.then(|| {
            self.modes_changed = false;
            self.modes.iter().map(Into::into).collect()
        });

        HeadUpdate {
            id: self.wlr.id().as_u32(),
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
            current_mode_index: -1,
            position: Default::default(),
            transform: Default::default(),
            scale: Default::default(),
            adaptive_sync: Default::default(),
            modes: Default::default(),
            geometry: Default::default(),
            state: Default::default(),
            mode_index_changed: Default::default(),
            geometry_changed: Default::default(),
            modes_changed: Default::default(),
            wlr,
        }
    }

    pub fn current_mode(&self) -> Option<(usize, &WlrMode)> {
        self.modes.iter().enumerate().find(|(_, m)| m.id() == self.current_mode)
    }

    pub fn recalc_geometry(&mut self) {
        self.geometry_changed = false;
        let Some((_, mode)) = self.current_mode() else {
            return;
        };

        self.geometry = calc_geometry(mode.size.clone(), self.position.clone(), self.scale, self.transform);
    }
}

impl From<&mut WlrHead> for QHead {
    fn from(val: &mut WlrHead) -> Self {
        <&WlrHead as Into<QHead>>::into(val)
    }
}

impl From<&WlrHead> for QHead {
    fn from(val: &WlrHead) -> Self {
        QHead {
            id: val.wlr.id().as_u32(),
            name: QString::from(val.name.to_string_lossy().as_ref()),
            description: QString::from(val.description.to_string_lossy().as_ref()),
            make: QString::from(val.make.to_string_lossy().as_ref()),
            model: QString::from(val.model.to_string_lossy().as_ref()),
            serial_number: QString::from(val.serial_number.to_string_lossy().as_ref()),
            physical_size: val.physical_size.clone(),
            enabled: val.enabled,
            position: val.position.clone(),
            geometry: val.geometry.clone(),
            transform: val.transform,
            scale: val.scale,
            adaptive_sync: val.adaptive_sync,
            current_mode: val.current_mode,
            current_mode_index: val.current_mode_index,
            // this shouldn't be created here
            modes: ModesModel::new(val.modes.iter().map(Into::into).collect()),
            pending_config: Default::default(),
        }
    }
}

impl WlState {
    pub fn on_head_event(ctx: EventCtx<Self, ZwlrOutputHeadV1>) {
        let EventCtx { conn, state, proxy, event, .. } = ctx;

        let Some(head) = state.heads.get_mut(&proxy.id().as_u32()) else {
            return;
        };

        if matches!(event, zwlr_output_head_v1::Event::Finished) {
            head.state = HeadState::Removed;
            proxy.release(conn);
            return;
        }

        match event {
            zwlr_output_head_v1::Event::Name(name) => {
                head.name = name.into_boxed_c_str();
            },
            zwlr_output_head_v1::Event::Description(description) => {
                head.description = description.into_boxed_c_str();
            },
            zwlr_output_head_v1::Event::PhysicalSize(s) => {
                head.physical_size = QSize::new(s.width, s.height);
            },
            zwlr_output_head_v1::Event::Make(make) => {
                head.make = make.into_boxed_c_str();
            },
            zwlr_output_head_v1::Event::Model(model) => {
                head.model = model.into_boxed_c_str();
            },
            zwlr_output_head_v1::Event::SerialNumber(serial_number) => {
                head.serial_number = serial_number.into_boxed_c_str();
            },
            zwlr_output_head_v1::Event::Mode(mode) => {
                conn.set_callback_for(mode, WlState::on_mode_event);
                state.modes_map.insert(mode.id().as_u32(), head.wlr.id().as_u32());
                head.modes_changed = true;
                head.mode_index_changed = true;
                head.modes.push(WlrMode::new(mode));
            },
            zwlr_output_head_v1::Event::Enabled(enabled) => {
                head.enabled = enabled == 1;
            },
            zwlr_output_head_v1::Event::CurrentMode(mode) => {
                head.geometry_changed = true;
                head.mode_index_changed = true;
                head.current_mode = mode.as_u32();
            },
            zwlr_output_head_v1::Event::Position(p) => {
                head.geometry_changed = true;
                head.position = QPoint::new(p.x, p.y);
            },
            zwlr_output_head_v1::Event::Transform(transform) => {
                head.geometry_changed = true;
                head.transform = transform as u32;
            },
            zwlr_output_head_v1::Event::Scale(scale) => {
                head.geometry_changed = true;
                head.scale = scale.as_f64();
            },
            zwlr_output_head_v1::Event::AdaptiveSync(state) => {
                head.adaptive_sync = state == zwlr_output_head_v1::AdaptiveSyncState::Enabled;
            },
            _ => {},
        }

        if head.state == HeadState::Ready {
            head.state = HeadState::Updated
        }
    }
}
use std::ffi::CStr;

use cxx::UniquePtr;
use cxx_qt_lib::{QObjectMutPtr, QPoint, QRect, QSize, QString, QVariant};

use wayrs_client::EventCtx;
use wayrs_client::object::Proxy;

use wayrs_protocols::wlr_output_management_unstable_v1::zwlr_output_head_v1::{self, ZwlrOutputHeadV1};

use crate::mode::{ModesModel, QMode, WlrMode};
use crate::state::WlState;
