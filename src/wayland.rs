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
    let conn = Connection::connect_to_env()?;
    let (globals, event_q) = registry_queue_init::<WlState>(&conn)?;
    let manager = globals.bind::<ZwlrOutputManagerV1, _, _>(&event_q.handle(), 2..=ZwlrOutputManagerV1::interface().version, ())?;

    let (tx, rx) = mpsc::channel();

    let event_fd = event::eventfd(0, EventfdFlags::NONBLOCK)?;
    let ev_fd = event_fd.try_clone()?;

    let _ = THREAD_INVOKER.set(ThreadInvoker { tx, event_fd });

    let state = WlState::new(manager);

    Ok(thread::spawn(move || {
        if let Err(e) = enter_loop(state, conn, event_q, ev_fd, rx) {
            qt::wayland_message(WlMsg::Fatal(QString::from(&e.to_string())));
        }
    }))
}

fn enter_loop(mut state: WlState, conn: Connection, mut q: EventQueue<WlState>, ev_fd: OwnedFd, rx: Receiver<WlCommand>) -> Result<(), Box<dyn std::error::Error>> {
    let event_fd = ev_fd.as_fd();
    let wl_fd = conn.as_fd();

    loop {
        let mut pfds = [PollFd::new(&wl_fd, PollFlags::IN), PollFd::new(&event_fd, PollFlags::IN)];

        let guard = loop {
            if let Some(g) = q.prepare_read() {
                break g;
            }
            q.dispatch_pending(&mut state)?;
        };

        // just to get wayland error msg
        let flush_err = conn.flush().err();

        if flush_err.is_none() {
            match event::poll(&mut pfds, None) {
                Ok(_) => {},
                Err(e) => match e.kind() {
                    ErrorKind::Interrupted => continue,
                    _ => return Err(e.into()),
                },
            }
        }

        let revents = pfds[0].revents();
        let wl_readable = flush_err.is_some() || revents.intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR);

        if wl_readable {
            match guard.read() {
                Ok(_) => {
                    q.dispatch_pending(&mut state)?;
                },
                Err(e) => {
                    return Err(e.into());
                },
            }
        } else {
            drop(guard);
        }

        if let Some(e) = flush_err {
            return Err(e.into());
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
                    WlCommand::ApplyConfiguration(confs) => state.apply_configuration(q.handle(), confs),
                    WlCommand::Exit => return Ok(()),
                }
            }
        }

        conn.flush()?;
    }
}

use cxx_qt_lib::QString;

use rustix::event::{self, EventfdFlags, PollFd, PollFlags};

use wayland_client::globals::registry_queue_init;
use wayland_client::{Connection, EventQueue, Proxy};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

use std::io::ErrorKind;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::head::PendingConfig;
use crate::qt;
use crate::state::WlState;
