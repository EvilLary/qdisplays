pub use qobject::ModesModel;

#[cxx_qt::bridge]
mod qobject {
    extern "C++" {
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        type ModesModel = super::ModesModelRs;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[cxx_override]
        fn data(self: &ModesModel, index: &QModelIndex, role: i32) -> QVariant;

        #[cxx_override]
        fn row_count(self: &ModesModel, parent: &QModelIndex) -> i32;

        #[inherit]
        fn begin_reset_model(self: Pin<&mut ModesModel>);

        #[inherit]
        fn end_reset_model(self: Pin<&mut ModesModel>);
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "modesmodel_make_unique"]
        fn make_unique() -> UniquePtr<ModesModel>;
    }
}

#[derive(Default)]
pub struct ModesModelRs {
    pub modes: Vec<QMode>,
}

impl qobject::ModesModel {
    pub fn new(modes: Vec<QMode>) -> UniquePtr<ModesModel> {
        let mut me = qobject::modesmodel_make_unique();
        me.as_mut().unwrap().set_modes(modes);
        me
    }

    pub fn set_modes(mut self: Pin<&mut Self>, modes: Vec<QMode>) {
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().modes = modes;
        self.as_mut().end_reset_model();
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        if role != cxx_qt_lib::ItemDataRole::DisplayRole.repr {
            return QVariant::default();
        }

        match self.modes.get(index.row() as usize) {
            Some(m) => m.display.clone(),
            None => QVariant::default(),
        }
    }

    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() { 0 } else { self.modes.len() as i32 }
    }
}

#[derive(Default)]
pub struct QMode {
    pub id: u32,
    pub size: QSize,
    pub display: QVariant,
}

pub struct WlrMode {
    pub size: QSize,
    pub refresh: i32,
    pub wlr: ZwlrOutputModeV1,
}

impl From<&WlrMode> for QMode {
    fn from(val: &WlrMode) -> Self {
        let display = format!("{}x{}@{:.2}Hz", val.size.width(), val.size.height(), (val.refresh as f32) / 1000.0);
        let display = QVariant::from(&QString::from(&display));

        QMode {
            id: val.id(),
            size: val.size.clone(),
            display,
        }
    }
}

impl WlrMode {
    pub fn id(&self) -> u32 {
        self.wlr.id().as_u32()
    }

    pub fn new(wlr: ZwlrOutputModeV1) -> Self {
        Self {
            wlr,
            size: Default::default(),
            refresh: Default::default(),
            // preferred: Default::default(),
        }
    }
}

impl WlState {
    pub fn on_mode_event(ctx: EventCtx<Self, ZwlrOutputModeV1>) {
        let EventCtx { conn, state, proxy, event, .. } = ctx;

        let mode_id = proxy.id().as_u32();
        let Some(head) = state.modes_map.get(&mode_id).and_then(|i| state.heads.get_mut(i)) else {
            return;
        };

        let Some((idx, mode)) = head.modes.iter_mut().enumerate().find(|(_, m)| m.id() == mode_id) else {
            return;
        };

        match event {
            zwlr_output_mode_v1::Event::Size(s) => {
                mode.size = QSize::new(s.width, s.height);
                head.modes_changed = true;
                // I could check for mode_id against current_mode,
                // but what if this event was emitted before Head::CurrentMode
                head.geometry_changed = true;
            },
            zwlr_output_mode_v1::Event::Refresh(refresh) => {
                mode.refresh = refresh;
                head.modes_changed = true;
            },
            zwlr_output_mode_v1::Event::Finished => {
                if head.current_mode == mode_id {
                    head.mode_index_changed = true;
                    head.geometry_changed = true;
                }

                mode.wlr.release(conn);
                state.modes_map.remove(&mode_id);
                head.modes.swap_remove(idx);
                head.modes_changed = true;
            },
            _ => {},
        }

        if head.state == HeadState::Ready {
            head.state = HeadState::Updated;
        }
    }
}

use std::pin::Pin;

use cxx::UniquePtr;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QModelIndex, QSize, QString, QVariant};

use wayrs_client::object::Proxy;
use wayrs_client::EventCtx;

use wayrs_protocols::wlr_output_management_unstable_v1::zwlr_output_mode_v1::{self, ZwlrOutputModeV1};

use crate::head::HeadState;
use crate::state::WlState;
