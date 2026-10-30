static THREAD: OnceLock<CxxQtThread<qobject::QDisplays>> = OnceLock::new();
// only these can changed, ideally I should build the list as the roles required only
static UPD_ROLES: LazyLock<QList<i32>> = LazyLock::new(|| {
    QList::from([
        Roles::Enabled as i32,
        Roles::Position as i32,
        Roles::Geometry as i32,
        Roles::Transform as i32,
        Roles::Scale as i32,
        Roles::AdaptiveSync as i32,
        Roles::CurrentModeIndex as i32,
    ])
});

#[cxx_qt::bridge]
mod qobject {
    extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        include!("cxx-qt-lib/qrect.h");
        type QRect = cxx_qt_lib::QRect;
    }

    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel;
    }

    #[qenum(QDisplays)]
    pub enum WaylandMessage {
        Fatal,
        Failed,
        Cancelled,
        Applied,
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[base = QAbstractListModel]
        #[qproperty(QRect, global_rect, READ, NOTIFY)]
        #[qproperty(bool, has_changes, READ = get_has_changes, NOTIFY)]
        type QDisplays = super::QDisplaysRs;

        fn get_has_changes(self: &QDisplays) -> bool;

        #[qinvokable]
        fn apply_configuration(self: Pin<&mut Self>);

        #[qinvokable]
        fn reset_configuration(self: Pin<&mut Self>);

        #[qsignal]
        fn wayland_message(self: Pin<&mut Self>, typ: i32, msg: QString);
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[cxx_override]
        fn data(self: &QDisplays, index: &QModelIndex, role: i32) -> QVariant;

        #[cxx_override]
        fn set_data(self: Pin<&mut QDisplays>, index: &QModelIndex, value: &QVariant, role: i32) -> bool;

        #[cxx_override]
        fn role_names(self: &QDisplays) -> QHash_i32_QByteArray;

        #[cxx_override]
        fn row_count(self: &QDisplays, parent: &QModelIndex) -> i32;

        #[inherit]
        fn index(self: &QDisplays, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[inherit]
        fn data_changed(self: Pin<&mut QDisplays>, top_left: &QModelIndex, bottom_right: &QModelIndex, roles: &QList_i32);

        #[inherit]
        fn begin_insert_rows(self: Pin<&mut QDisplays>, parent: &QModelIndex, first: i32, last: i32);

        #[inherit]
        fn end_insert_rows(self: Pin<&mut QDisplays>);

        #[inherit]
        fn begin_remove_rows(self: Pin<&mut QDisplays>, parent: &QModelIndex, first: i32, last: i32);

        #[inherit]
        fn end_remove_rows(self: Pin<&mut QDisplays>);

        #[inherit]
        fn begin_reset_model(self: Pin<&mut QDisplays>);

        #[inherit]
        fn end_reset_model(self: Pin<&mut QDisplays>);
    }

    impl cxx_qt::Threading for QDisplays {}
    impl cxx_qt::Constructor<()> for QDisplays {}
}

#[derive(Default)]
pub struct QDisplaysRs {
    global_rect: QRect,
    heads: Vec<QHead>,
}

impl qobject::QDisplays {
    fn clear_model(mut self: Pin<&mut Self>) {
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().heads.clear();
        self.as_mut().end_reset_model();
    }

    fn get_has_changes(&self) -> bool {
        self.heads.iter().any(|h| !h.pending_config.is_empty())
    }

    fn reset_configuration(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().heads.iter_mut().for_each(|h| h.pending_config.clear());
        let len = self.as_ref().rust().heads.len() as i32;
        let start = self.index(0, 0, &Default::default());
        let end = self.index(len - 1, 0, &Default::default());
        self.as_mut().data_changed(&start, &end, &UPD_ROLES);
        self.as_mut().calc_global_rect();
        self.as_mut().has_changes_changed();
    }

    fn apply_configuration(mut self: Pin<&mut Self>) {
        let confs = self
            .as_mut()
            .rust_mut()
            .heads
            .iter_mut()
            .filter_map(|h| {
                if h.pending_config.is_empty() {
                    None
                } else {
                    let c = h.pending_config.clone();
                    h.pending_config.clear();
                    Some((h.id, c))
                }
            })
            .collect();

        self.as_mut().has_changes_changed();
        wayland::request(WlCommand::ApplyConfiguration(confs));
    }

    fn update_head(mut self: Pin<&mut Self>, update: HeadUpdate) {
        let heads = &mut self.as_mut().rust_mut().heads;
        let Some((row, head)) = heads.iter_mut().enumerate().find(|(_, h)| h.id == update.id) else {
            return;
        };

        head.pending_config.clear();
        head.enabled = update.enabled;
        head.position = update.position;
        head.geometry = update.geometry;
        head.transform = update.transform;
        head.scale = update.scale;
        head.adaptive_sync = update.adaptive_sync;
        head.current_mode_index = update.current_mode_index;
        if let Some(m) = update.modes {
            head.set_modes(m);
        }

        let idx = self.index(row as i32, 0, &Default::default());
        self.as_mut().data_changed(&idx, &idx, &UPD_ROLES);

        self.as_mut().calc_global_rect();
    }

    fn remove_head(mut self: Pin<&mut Self>, id: u32) {
        let Some(row) = self.heads.iter().position(|h| h.id == id) else {
            return;
        };

        self.as_mut().begin_remove_rows(&Default::default(), row as i32, row as i32);
        self.as_mut().rust_mut().heads.remove(row);
        self.as_mut().end_remove_rows();

        self.calc_global_rect();
    }

    fn add_head(mut self: Pin<&mut Self>, h: QHead) {
        let row = self.heads.len() as i32;

        self.as_mut().begin_insert_rows(&Default::default(), row, row);
        self.as_mut().rust_mut().heads.push(h);
        self.as_mut().end_insert_rows();

        self.calc_global_rect();
    }

    fn calc_global_rect(mut self: Pin<&mut Self>) {
        let rect = self
            .heads
            .iter()
            .filter(|h| h.enabled())
            .fold(None, |acc: Option<QRect>, h| {
                let geom = h.geometry();
                match acc {
                    Some(r) => Some(r.united(&geom)),
                    None => Some(geom),
                }
            })
            .unwrap_or_default();

        if rect != self.global_rect {
            self.as_mut().rust_mut().global_rect = rect;
            self.as_mut().global_rect_changed();
        }
    }

    fn attach_heads(mut self: Pin<&mut Self>, heads: Vec<QHead>) {
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().heads = heads;
        self.as_mut().end_reset_model();
        self.calc_global_rect();
    }

    fn set_data(mut self: Pin<&mut Self>, index: &QModelIndex, value: &QVariant, role: i32) -> bool {
        let row = index.row();
        let mut this_rs = self.as_mut().rust_mut();
        let Some(head) = this_rs.heads.get_mut(row as usize) else {
            return false;
        };
        let Ok(role) = Roles::try_from(role) else {
            return false;
        };

        let mut recalc = false;
        let mut changed_roles = QList::from([role as i32]);

        match role {
            Roles::Enabled => {
                recalc = head.set_enabled(value);
            },
            Roles::CurrentModeIndex => {
                recalc = head.set_mode(value);
            },
            Roles::Position => {
                recalc = head.set_position(value);
            },
            Roles::Transform => {
                recalc = head.set_transform(value);
            },
            Roles::Scale => {
                recalc = head.set_scale(value);
            },
            Roles::AdaptiveSync => {
                head.set_adaptive_sync(value);
            },
            _ => return false,
        }

        if recalc {
            self.as_mut().calc_global_rect();
            changed_roles.append(Roles::Geometry as i32);
        }

        self.as_mut().has_changes_changed();

        let idx = self.as_ref().index(row, 0, &Default::default());
        self.as_mut().data_changed(&idx, &idx, &changed_roles);

        true
    }

    #[rustfmt::skip]
    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(h) = self.heads.get(index.row() as usize) else {
            return QVariant::default();
        };

        let Ok(role) = Roles::try_from(role) else {
            return QVariant::default();
        };

        match role {
            Roles::Name             => QVariant::from(&h.name),
            Roles::Description      => QVariant::from(&h.description),
            Roles::Make             => QVariant::from(&h.make),
            Roles::ModelName        => QVariant::from(&h.model),
            Roles::SerialNumber     => QVariant::from(&h.serial_number),
            Roles::PhysicalSize     => QVariant::from(&h.physical_size),
            Roles::Enabled          => QVariant::from(&h.enabled()),
            Roles::CurrentModeIndex => QVariant::from(&h.current_mode().map_or(-1, |m| m.0 as i32)),
            Roles::Geometry         => QVariant::from(&h.geometry()),
            Roles::Transform        => QVariant::from(&h.transform()),
            Roles::Position         => QVariant::from(&h.position()),
            Roles::Scale            => QVariant::from(&h.scale()),
            Roles::AdaptiveSync     => QVariant::from(&h.adaptive_sync()),
            Roles::Modes            => QVariant::from(&h.modes_qobject()),
        }
    }

    #[rustfmt::skip]
    fn role_names(&self) -> qobject::QHash_i32_QByteArray {
        let mut roles = qobject::QHash_i32_QByteArray::default();
        roles.insert(Roles::Name             as i32, "name".into());
        roles.insert(Roles::Description      as i32, "description".into());
        roles.insert(Roles::Make             as i32, "make".into());
        roles.insert(Roles::ModelName        as i32, "modelName".into());
        roles.insert(Roles::SerialNumber     as i32, "serialNumber".into());
        roles.insert(Roles::PhysicalSize     as i32, "physicalSize".into());
        roles.insert(Roles::Enabled          as i32, "enabled".into());
        roles.insert(Roles::CurrentModeIndex as i32, "currentModeIndex".into());
        roles.insert(Roles::Geometry         as i32, "geometry".into());
        roles.insert(Roles::Transform        as i32, "transform".into());
        roles.insert(Roles::Scale            as i32, "scale".into());
        roles.insert(Roles::Position         as i32, "position".into());
        roles.insert(Roles::AdaptiveSync     as i32, "adaptiveSync".into());
        roles.insert(Roles::Modes            as i32, "modes".into());
        roles
    }

    pub fn row_count(&self, parent: &QModelIndex) -> i32 {
        if !parent.is_valid() { self.heads.len() as i32 } else { 0 }
    }
}

impl cxx_qt::Initialize for qobject::QDisplays {
    fn initialize(self: Pin<&mut Self>) {
        let _ = THREAD.set(self.qt_thread());
        wayland::request(WlCommand::GetHeads);
    }
}

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Roles {
    Name = cxx_qt_lib::ItemDataRole::UserRole.repr + 1,
    Description,
    Make,
    ModelName,
    SerialNumber,
    PhysicalSize,
    Enabled,
    CurrentModeIndex,
    Geometry,
    Transform,
    Position,
    Scale,
    AdaptiveSync,
    Modes,
}

impl TryFrom<i32> for Roles {
    type Error = ();

    #[rustfmt::skip]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            v if v == Roles::Name              as i32 => Ok(Roles::Name),
            v if v == Roles::Description       as i32 => Ok(Roles::Description),
            v if v == Roles::Make              as i32 => Ok(Roles::Make),
            v if v == Roles::ModelName         as i32 => Ok(Roles::ModelName),
            v if v == Roles::SerialNumber      as i32 => Ok(Roles::SerialNumber),
            v if v == Roles::PhysicalSize      as i32 => Ok(Roles::PhysicalSize),
            v if v == Roles::Enabled           as i32 => Ok(Roles::Enabled),
            v if v == Roles::CurrentModeIndex  as i32 => Ok(Roles::CurrentModeIndex),
            v if v == Roles::Geometry          as i32 => Ok(Roles::Geometry),
            v if v == Roles::Transform         as i32 => Ok(Roles::Transform),
            v if v == Roles::Position          as i32 => Ok(Roles::Position),
            v if v == Roles::Scale             as i32 => Ok(Roles::Scale),
            v if v == Roles::AdaptiveSync      as i32 => Ok(Roles::AdaptiveSync),
            v if v == Roles::Modes             as i32 => Ok(Roles::Modes),
            _ => Err(()),
        }
    }
}

macro_rules! invoke {
    (|$m:ident| $bdy:expr) => {
        THREAD.get().inspect(|qt_thread| {
            #[allow(unused_mut)]
            let _ = qt_thread.queue(move |mut $m| $bdy);
        });
    };
}

pub fn all_heads(heads: Vec<QHead>) {
    invoke!(|q| q.attach_heads(heads));
}

pub fn add_head(head: QHead) {
    invoke!(|q| q.add_head(head));
}

pub fn remove_head(id: u32) {
    invoke!(|q| q.remove_head(id));
}

pub fn update_head(update: HeadUpdate) {
    invoke!(|q| q.update_head(update));
}

pub fn wayland_message(err: WlMsg) {
    use qobject::WaylandMessage;

    let (typ, msg) = match err {
        WlMsg::Fatal(qstring) => {
            invoke!(|q| q.clear_model());
            (WaylandMessage::Fatal, qstring)
        },
        WlMsg::ConfigCancelled => {
            // just so stale options get cleared
            invoke!(|q| q.reset_configuration());
            (WaylandMessage::Cancelled, QString::default())
        },
        WlMsg::ConfigFailed => {
            invoke!(|q| q.reset_configuration());
            (WaylandMessage::Failed, QString::default())
        },
        WlMsg::ConfigApplied => (WaylandMessage::Applied, QString::default()),
    };

    invoke!(|q| q.wayland_message(typ.repr, msg));
}

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{QList, QModelIndex, QRect, QString, QVariant};

use std::pin::Pin;
use std::sync::{LazyLock, OnceLock};

use crate::head::{HeadUpdate, QHead};
use crate::wayland::{self, WlCommand, WlMsg};
