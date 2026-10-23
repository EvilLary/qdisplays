use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("qdisplays").depend("org.kde.kirigami").version(1, 0).qml_files(&[
        "qml/Main.qml",
        "qml/OutputsView.qml",
        "qml/OptionsPanel.qml",
    ]))
    .qt_module("QuickControls2")
    .files(&["src/qt.rs"])
    .build();

    println!("cargo:rustc-link-arg=-fuse-ld=lld");
}
