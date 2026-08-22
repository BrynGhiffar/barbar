use std::os::{fd::{AsFd, AsRawFd}, raw::c_void};
use serde::{Deserialize, Serialize};

use mio::net::UnixStream;

use nom::{
    IResult, Parser, bytes::complete::{tag, take_until}, combinator::rest, sequence::separated_pair
};

#[derive(Debug, PartialEq)]
pub struct RawHyprEvent<'a> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
}

/// Parses any Hyprland IPC event string into a generic RawHyprEvent
pub fn parse_raw_event(input: &str) -> IResult<&str, RawHyprEvent<'_>> {
    let input = input.trim_end_matches(['\r', '\n']);
    let mut f = separated_pair(
        take_until(">>"),
        tag(">>"),
        rest,
    );

    let (remaining, (name, raw_args)) = f.parse(input)?;

    let args = if raw_args.is_empty() {
        Vec::new()
    } else {
        raw_args.split(',').collect()
    };

    Ok((remaining, RawHyprEvent { name, args }))
}

impl<'a> From<RawHyprEvent<'a>> for HyprEvent {
    fn from(raw: RawHyprEvent<'a>) -> Self {
        match (raw.name, raw.args.as_slice()) {
            ("workspacev2", [id, name, ..]) => HyprEvent::Workspace(HyprEventWorkspace {
                workspace_id: id.to_string(),
                workspace_name: name.to_string(),
            }),
            ("focusedmonv2", [mon, ws, ..]) => HyprEvent::FocusedMon(HyprEventFocusedMon {
                monitor_name: mon.to_string(),
                workspace_id: ws.to_string(),
            }),

            ("activewindowv2", [addr, ..]) => HyprEvent::ActiveWindow(HyprEventActiveWindow {
                window_address: addr.to_string(),
            }),
            ("monitorremovedv2", [id, name]) => HyprEvent::MonitorRemoved(HyprEventMonitorRemoved {
                monitor_id: id.to_string(),
                monitor_name: name.to_string(),
                monitor_description: String::new(),
            }),

            ("monitoraddedv2", [id, name, desc, ..]) => HyprEvent::MonitorAdded(HyprEventMonitorAdded {
                monitor_id: id.to_string(),
                monitor_name: name.to_string(),
                monitor_description: desc.to_string(),
            }),
            ("createworkspacev2", [id, name, ..]) => HyprEvent::CreateWorkspace(HyprEventCreateWorkspace {
                workspace_id: id.to_string(),
                workspace_name: name.to_string(),
            }),
            ("movewindowv2", [addr, ws_id, ws_name, ..]) => HyprEvent::MoveWindow(HyprEventMoveWindow {
                window_address: addr.to_string(),
                workspace_id: ws_id.to_string(),
                workspace_name: ws_name.to_string(),
            }),
            ("openwindow", [addr, ws_name, class, ..]) => HyprEvent::OpenWindow(HyprEventOpenWindow {
                window_address: addr.to_string(),
                workspace_name: ws_name.to_string(),
                window_class: class.to_string(),
            }),

            ("closewindow", [addr, ..]) => HyprEvent::CloseWindow(HyprEventCloseWindow {
                window_address: addr.to_string(),
            }),
            (event, args) => HyprEvent::Raw(HyprEventRaw {
                event: event.to_string(),
                args: args.iter()
                    .map(|s | s.to_string())
                    .collect()
            }),
        }
    }
}

// Single convenience function
pub fn parse_hypr_event(input: &str) -> Vec<HyprEvent> {
    let mut res = vec![];
    let its = input.trim().split('\n');
    for it in its {
        if let Ok((_, raw_event)) = parse_raw_event(it) {
            res.push(raw_event.into());
        }
    }
    res
}


#[derive(Debug, PartialEq)]
pub enum HyprEvent {
    // emitted on workspace change.
    // Is emitted ONLY when a user requests a workspace change,
    // and is not emitted on mouse movements (see focusedmon)
    Workspace(HyprEventWorkspace),
    FocusedMon(HyprEventFocusedMon),
    ActiveWindow(HyprEventActiveWindow),
    MonitorRemoved(HyprEventMonitorRemoved),
    MonitorAdded(HyprEventMonitorAdded),
    CreateWorkspace(HyprEventCreateWorkspace),
    MoveWindow(HyprEventMoveWindow),
    OpenWindow(HyprEventOpenWindow),
    CloseWindow(HyprEventCloseWindow),
    Raw(HyprEventRaw)
}

#[derive(Debug, PartialEq)]
pub struct HyprEventWorkspace {
    pub workspace_id: String,
    pub workspace_name: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventFocusedMon {
    pub monitor_name: String,
    pub workspace_id: String,
}

#[derive(Debug, PartialEq)]
pub struct HyprEventActiveWindow {
    pub window_address: String,
}

#[derive(Debug, PartialEq)]
pub struct HyprEventMonitorRemoved {
    pub monitor_id: String,
    pub monitor_name: String,
    pub monitor_description: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventMonitorAdded {
    pub monitor_id: String,
    pub monitor_name: String,
    pub monitor_description: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventCreateWorkspace {
    pub workspace_id: String,
    pub workspace_name: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventMoveWindow {
    pub window_address: String,
    pub workspace_id: String,
    pub workspace_name: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventOpenWindow {
    pub window_address: String,
    pub workspace_name: String,
    pub window_class: String,
}

#[derive(Debug, PartialEq)]
pub struct HyprEventCloseWindow {
    pub window_address: String
}

#[derive(Debug, PartialEq)]
pub struct HyprEventRaw {
    pub event: String,
    pub args: Vec<String>
}

pub struct HyprIPC {
    stream: UnixStream
}

impl AsFd for HyprIPC {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.stream.as_fd()
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyprWorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    #[serde(rename = "monitorID")]
    pub monitor_id: i32,
    pub windows: u32,
    pub hasfullscreen: bool,
    pub lastwindow: String,
    pub lastwindowtitle: String,
    pub ispersistent: bool,
    pub tiled_layout: String,
}

impl HyprIPC {
    pub fn new() -> anyhow::Result<Self> {
        let hypdir = std::env::var("XDG_RUNTIME_DIR")?;
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
        let socket = format!("{}/hypr/{}/.socket2.sock", hypdir, sig);
        let stream = UnixStream::connect(socket)?;
        Ok(Self { stream })
    }

    fn ctl_socket() -> anyhow::Result<UnixStream> {
        let hypdir = std::env::var("XDG_RUNTIME_DIR")?;
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
        let socket = format!("{}/hypr/{}/.socket.sock", hypdir, sig);
        let stream = UnixStream::connect(socket)?;
        Ok(stream)
    }

    fn read_stream(stream: &mut UnixStream) -> anyhow::Result<String> {
        let mut buf = [0; 2048];
        let n = stream.try_io(|| {
            let buf_ptr = &mut buf as *mut _ as *mut _;
            let res = unsafe { libc::recv(stream.as_raw_fd(), buf_ptr, buf.len(), 0) };
            if res != -1 {
                Ok(res as usize)
            } else {
                Err(std::io::Error::last_os_error())
            }
        })?;
        let string = str::from_utf8(&buf[..n])?;
        Ok(string.to_string())
    }

    fn write_stream(stream: &mut UnixStream, buf: &[u8]) -> anyhow::Result<usize> {
        let n = stream.try_io(|| {
            let buf_ptr = buf.as_ptr() as *const c_void;
            let res = unsafe { libc::send(stream.as_raw_fd(), buf_ptr, buf.len(), 0) };
            if res != -1 {
                Ok(res as usize)
            } else {
                // If EAGAIN or EWOULDBLOCK is set by libc::send, the closure
                // should return `WouldBlock` error.
                Err(std::io::Error::last_os_error())
            }
        })?;
        Ok(n)
    }

    fn wait_readable(stream: &mut UnixStream, timeout_ms: i32) -> anyhow::Result<()> {
        let mut pfd = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };

        if ret < 0 {
            return Err(std::io::Error::last_os_error().into());
        } else if ret == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "poll timed out").into());
        }

        if (pfd.revents & libc::POLLIN) != 0 {
            Ok(())
        } else {
            Err(std::io::Error::other("poll error or hangup").into())
        }
    }

    pub fn recv(&mut self) -> anyhow::Result<Vec<HyprEvent>> {
        let res = Self::read_stream(&mut self.stream)?;
        let evts = parse_hypr_event(&res);
        Ok(evts)
    }

    pub fn get_workspaces(&self) -> anyhow::Result<Vec<HyprWorkspaceInfo>> {
        let mut sock = Self::ctl_socket()?;
        let buf = "j/workspaces";
        let _n = Self::write_stream(&mut sock, buf.as_bytes())?;
        Self::wait_readable(&mut sock, 1)?;
        let res = Self::read_stream(&mut sock)?;
        let workspaces = serde_json::from_str(&res)?;
        Ok(workspaces)
    }
}
