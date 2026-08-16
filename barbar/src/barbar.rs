use wayland_client::protocol::wl_shm;

use crate::io::{ctl::Ioctl, event::{IoEvent, IoInitRender, IoOutputEvent, IoRenderEvent, IoRenderRequest, IoRequest}};

pub struct Barbar {
     io: Ioctl,
     bar_height: i32
}

impl Barbar {
    pub fn new() -> anyhow::Result<Self> {
        let io = Ioctl::new()?;
        let bar_height  = 30;

        Ok(Barbar { io, bar_height })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let events = self.io.poll()?;
            // tracing::info!("{:#?}", events);
            let reqs = events
                .into_iter()
                .flat_map(|evt| self.logic(evt))
                .collect::<Vec<_>>();
            for req in reqs {
                self.io.send(req)?;
            }
        }
    }

    pub fn logic(&mut self, event: IoEvent) -> Vec<IoRequest> {
        match event {
            IoEvent::Render(event) => self.render_logic(event),
            IoEvent::ConfigureOutput(event) => self.configure_logic(event),
            IoEvent::NewOutput(event) => self.new_output_logic(event),
            IoEvent::DestroyOutput(event) => self.destroy_output_logic(event)
        }
    }

    pub fn render_logic(&mut self, event: IoRenderEvent) -> Vec<IoRequest> {
        let IoRenderEvent { mut slot, oi } = event;
        let (buffer, canvas) = match slot
            .create_buffer(
                oi.width,
                30,
                oi.width * 4,
                wl_shm::Format::Argb8888
            ) {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!("Failed to create buffer in rendering logic: {err}");
                    return vec![];
                }
            };

        // Do something with `canvas`
        //
        canvas.chunks_exact_mut(4).for_each(|buff|{
            buff[0] = 0;
            buff[1] = 0;
            buff[2] = 0;
            buff[3] = 120;
        }); 
        vec![IoRequest::Render( IoRenderRequest {
            slot,
            buffer,
            oi,
            bar_height: self.bar_height,
            render_next: false,
        })]
    }

    pub fn configure_logic(&mut self, event: IoOutputEvent) -> Vec<IoRequest> {
        let IoOutputEvent { oi } = event;
        let init = IoInitRender { oi, bar_height: self.bar_height };
        vec![IoRequest::InitRender(init)]
    }

    pub fn new_output_logic(&mut self, _: IoOutputEvent) -> Vec<IoRequest> {
        vec![]
    }

    pub fn destroy_output_logic(&mut self, _: IoOutputEvent) -> Vec<IoRequest> {
        vec![]
    }
}
