static THREAD_INVOKER: OnceLock<ThreadInvoker> = OnceLock::new();

pub enum WlCommand {
    GetHeads,
    ApplyConfiguration(Vec<(u32, PendingConfig)>),
    Exit,
}

pub enum WlMsg {
    Fatal(QString),
    ConfigApplied,
    ConfigCancelled,
    ConfigFailed,
}

struct ThreadInvoker {
    tx: Sender<WlCommand>,
    event_fd: OwnedFd,
}

pub fn request(r: WlCommand) {
    THREAD_INVOKER.get().inspect(|c| {
        let _ = c.tx.send(r);
        let _ = rustix::io::write(&c.event_fd, &1u64.to_ne_bytes());
    });
}

pub fn start_wayland() -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
    let mut conn = Connection::connect()?;
    conn.blocking_roundtrip()?;
    let manager: ZwlrOutputManagerV1 = conn.bind_singleton_with_cb(2..=ZwlrOutputManagerV1::INTERFACE.version, WlState::on_manager_event)?;

    let (tx, rx) = mpsc::channel();

    let event_fd = event::eventfd(0, EventfdFlags::NONBLOCK)?;
    let ev_fd = event_fd.try_clone()?;

    let _ = THREAD_INVOKER.set(ThreadInvoker { tx, event_fd });

    let state = WlState::new(manager);

    Ok(thread::spawn(move || {
        if let Err(e) = enter_loop(state, conn, ev_fd, rx) {
            qt::wayland_message(WlMsg::Fatal(QString::from(&e.to_string())));
        }
    }))
}

fn enter_loop(mut state: WlState, mut conn: Connection<WlState>, ev_fd: OwnedFd, rx: Receiver<WlCommand>) -> Result<(), Box<dyn std::error::Error>> {
    let event_fd = ev_fd.as_fd();
    let wl_fd = unsafe { BorrowedFd::borrow_raw(conn.as_raw_fd()) };

    loop {
        conn.flush(IoMode::Blocking)?;

        let mut pfds = [PollFd::new(&wl_fd, PollFlags::IN), PollFd::new(&event_fd, PollFlags::IN)];

        match event::poll(&mut pfds, None) {
            Ok(_) => {},
            Err(e) => match e.kind() {
                ErrorKind::Interrupted => continue,
                _ => return Err(e.into()),
            },
        }

        if pfds[0].revents().contains(PollFlags::IN) {
            conn.recv_events(IoMode::Blocking)?;
            conn.dispatch_events(&mut state);
        }

        if pfds[1].revents().contains(PollFlags::IN) {
            let mut buf = [0u8; 8];
            match rustix::io::read(event_fd, &mut buf) {
                Ok(_) => {},
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::Interrupted => continue,
                    _ => return Err(e.into()),
                },
            }

            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    WlCommand::GetHeads => qt::all_heads(state.heads.values().map(|h| h.into()).collect()),
                    WlCommand::ApplyConfiguration(confs) => state.apply_configuration(&mut conn, confs),
                    WlCommand::Exit => return Ok(()),
                }
            }
        }
    }
}

use cxx_qt_lib::QString;

use rustix::event::{self, EventfdFlags, PollFd, PollFlags};

use wayrs_client::object::Proxy;
use wayrs_client::{Connection, IoMode};

use wayrs_protocols::wlr_output_management_unstable_v1::ZwlrOutputManagerV1;

use std::io::ErrorKind;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::head::PendingConfig;
use crate::qt;
use crate::state::WlState;
