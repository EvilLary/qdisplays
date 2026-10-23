use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
// TODO: use wayrs instead

mod head;
mod mode;
mod qt;
mod state;
mod wayland;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tbh it's better for the qml singleton to start wayland
    wayland::start_wayland()?;

    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    engine
        .as_mut()
        .expect("couldn't create QQmlApplicationEngine")
        .load(&QUrl::from("qrc:/qt/qml/qdisplays/qml/Main.qml"));

    std::process::exit(app.as_mut().expect("couldn't create QGuiApplication").exec());
}
