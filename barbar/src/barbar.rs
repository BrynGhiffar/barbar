use fontdue::{Font, FontSettings};
use wayland_client::protocol::wl_shm;

use crate::{io::{ctl::Ioctl, event::{IoEvent, IoInitRender, IoOutputEvent, IoRenderEvent, IoRenderRequest, IoRequest}}, surface::BarSurface};

pub struct Barbar {
     io: Ioctl,
     bar_height: i32,
     font: Font
}

impl Barbar {
    pub fn new() -> anyhow::Result<Self> {
        let io = Ioctl::new()?;
        let bar_height  = 20;

        // let font = include_bytes!("/usr/share/fonts/TTF/FiraCodeNerdFontMono-SemiBold.ttf") as &[u8];
        let font = include_bytes!("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-SemiBold.ttf") as &[u8];
        // let font = include_bytes!("/usr/share/fonts/TTF/SymbolsNerdFontMono-Regular.ttf") as &[u8];
        let font = Font::from_bytes(font, FontSettings::default()).unwrap();
        // let fonts = &[font];
        // let mut layout = Layout::new(CoordinateSystem::PositiveYUp);
        // layout.append(fonts, &TextStyle::new("Hello ", 35.0, 0));
        // println!("{:?}", layout.glyphs());
        // layout.ra

        Ok(Barbar { io, font, bar_height })
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
        std::thread::sleep(std::time::Duration::from_millis(1_000));
        let IoRenderEvent { mut slot, oi } = event;

        let (buffer, canvas) = match slot
            .create_buffer(
                oi.width,
                self.bar_height,
                oi.width * 4,
                wl_shm::Format::Argb8888
            ) {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!("Failed to create buffer in rendering logic: {err}");
                    return vec![];
                }
            };

        let mut surf = BarSurface::from_raw(canvas, oi.width as usize, self.bar_height as usize);
        surf.draw(&self.font);

        vec![IoRequest::Render(IoRenderRequest {
            slot,
            buffer,
            oi,
            bar_height: self.bar_height,
            render_next: true,
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
