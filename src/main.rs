use cxx_qt_lib::{QQmlApplicationEngine, QUrl};
use cxx_qt_lib_extras::QApplication;
use std::process::ExitCode;

mod head;
mod mode;
mod qt;
mod state;
mod wayland;

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    // tbh it's better for the qml singleton to start wayland
    let wl_th = wayland::start_wayland()?;

    // while QGuiApplication is fine, it misses some small features that you'd expect from a
    // qt destkop app, maybe move to QWidgets instead
    let mut app = QApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    engine
        .as_mut()
        .expect("couldn't create QQmlApplicationEngine")
        .load(&QUrl::from("qrc:/qt/qml/qdisplays/qml/Main.qml"));

    let code = app.as_mut().expect("couldn't create QGuiApplication").exec();

    wayland::request(wayland::WlCommand::Exit);
    let _ = wl_th.join();

    Ok((code as u8).into())
}
