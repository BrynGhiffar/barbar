use std::os::fd::{AsFd, AsRawFd};

use mio::{Events, Interest, Poll, Token, unix::SourceFd};
use wayland_client::{EventQueue, backend::WaylandError};

use crate::io::{event::{IoEvent, IoRequest, OutputInfo}, layer::LayerIO};


pub struct Ioctl {
    layer_io: LayerIO,
    layer_evq: EventQueue<LayerIO>,
    poll: Poll,
    events: Events,
}

const LAYER_TOKEN: Token = Token(0);

impl Ioctl {
    pub fn new() -> anyhow::Result<Self> {
        let (mut layer_io, mut layer_evq) = LayerIO::new()?;
        let poll = Poll::new()?;
        let events = Events::with_capacity(32);
        let wl_fd = layer_io.conn.as_fd();
        let wl_fd = wl_fd.as_raw_fd();
        layer_evq.roundtrip(&mut layer_io)?;

        poll.registry()
            .register(&mut SourceFd(&wl_fd), LAYER_TOKEN, Interest::READABLE)?;

        Ok(Self { layer_io, layer_evq, poll, events })
    }

    pub fn pop_layer_io_evt(&mut self, res: &mut Vec<IoEvent>) {
        while let Some(evt) = self.layer_io.io_queue.pop_front() {
            res.push(evt);
        }
    }

    pub fn poll(&mut self) -> anyhow::Result<Vec<IoEvent>> {
        let mut res = vec![];

        self.layer_evq.flush()?;
        self.layer_evq.dispatch_pending(&mut self.layer_io)?;

        if let Some(guard) = self.layer_evq.prepare_read() {
            tracing::debug!("Polling...");
            loop {
                match self.poll.poll(&mut self.events, None) {
                    Ok(()) => break,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                        tracing::info!("System call interrupted by signal (EINTR). Retrying...");
                        continue;
                    },
                    Err(e) => return Err(e.into())
                }
            }
            tracing::debug!("Received some events...");

            if self.events.iter().any(|e| e.token() == LAYER_TOKEN) {
                match guard.read() {
                    Ok(_) => {
                        self.layer_evq.dispatch_pending(&mut self.layer_io)?;
                    }
                    Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        tracing::debug!("Spurious wakeup on wayland socket");
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        self.pop_layer_io_evt(&mut res);
        Ok(res)
    }

    pub fn send(&mut self, req: IoRequest) -> anyhow::Result<()> {
        match req {
            IoRequest::InitRender(oi) => self.layer_io.init_render(oi)?,
            IoRequest::Render(req) =>  self.layer_io.render(req)?,
        }
        Ok(())
    }

    pub fn get_oi_by_name(&self, name: &str) -> Option<OutputInfo> {
        self.layer_io.get_output_by_name(name).map(|out| out.to_oi())
    }
}
