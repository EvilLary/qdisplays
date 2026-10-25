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

    unsafe extern "RustQt" {
        #[cxx_override]
        fn data(self: &ModesModel, index: &QModelIndex, role: i32) -> QVariant;

        #[cxx_override]
        #[rust_name = "row_count"]
        fn rowCount(self: &ModesModel, parent: &QModelIndex) -> i32;

        #[inherit]
        #[rust_name = "begin_reset_model"]
        fn beginResetModel(self: Pin<&mut ModesModel>);

        #[inherit]
        #[rust_name = "end_reset_model"]
        fn endResetModel(self: Pin<&mut ModesModel>);
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
        let Some(m) = self.modes.get(index.row() as usize) else {
            return QVariant::default();
        };

        use std::cell::Cell;
        use std::fmt::Write;
        thread_local! {
            static BUF: Cell<String> = Cell::new(String::with_capacity(14));
        }

        if role == cxx_qt_lib::ItemDataRole::DisplayRole.repr {
            let mut buf = BUF.take();
            buf.clear();
            let _ = write!(buf, "{}x{}@{:.2}Hz", m.size.width(), m.size.height(), (m.refresh as f32) / 1000.0);
            let v = QVariant::from(&QString::from(&buf));
            BUF.set(buf);
            v
        } else {
            QVariant::default()
        }
    }

    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() { 0 } else { self.modes.len() as i32 }
    }
}

// at least it's 16B not 64B :)
#[derive(Default)]
pub struct QMode {
    pub id: u32,
    pub size: QSize,
    pub refresh: i32,
}

pub struct WlrMode {
    pub size: QSize,
    pub refresh: i32,
    // pub preferred: bool,
    pub wlr: ZwlrOutputModeV1,
}

// fucking retarded rust, can't auto impl for T, &T & &mut T
impl From<&WlrMode> for QMode {
    fn from(val: &WlrMode) -> Self {
        QMode {
            id: val.id(),
            size: val.size.clone(),
            refresh: val.refresh,
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
            size: Default::default(),
            refresh: Default::default(),
            // preferred: Default::default(),
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
                mode.size = QSize::new(width, height);
                head.modes_changed = true;
                // I could check for mode_id against current_mode,
                // but what if this event was emitted before Head::CurrentMode
                head.geometry_changed = true;
            },
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                mode.refresh = refresh;
                head.modes_changed = true;
            },
            zwlr_output_mode_v1::Event::Preferred => {
                // mode.preferred = true;
            },
            zwlr_output_mode_v1::Event::Finished => {
                if head.current_mode == mode_id {
                    head.mode_index_changed = true;
                    head.geometry_changed = true;
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

use std::pin::Pin;

use cxx::UniquePtr;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QModelIndex, QSize, QString, QVariant};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::{self, ZwlrOutputModeV1};

use crate::head::HeadState;
use crate::state::WlState;
