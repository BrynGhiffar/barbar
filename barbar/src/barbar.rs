use fontdue::{Font, FontSettings};
use wayland_client::protocol::wl_shm;
use systemstat::{CPULoad, Platform};

use crate::{io::{ctl::Ioctl, event::{IoEvent, IoInitRender, IoOutputEvent, IoRenderEvent, IoRenderRequest, IoRequest}}, surface::BarSurface};


pub struct Barbar {
     io: Ioctl,
     bar_height: i32,
     font: Font,
     stat: SystemStat
}

pub struct SystemStat {
    sys: systemstat::System,
    cpu_stat: Option<systemstat::DelayedMeasurement<CPULoad>>,
}

pub struct SystemStatSnapshot {
    pub cpu_user: f32,
    pub cpu_system: f32,
    pub mem_free: u64,
    pub mem_total: u64,
    pub total_disk: u64,
    pub free_disk: u64
}

impl Default for SystemStat {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemStat {
    pub fn new() -> Self {
        let sys = systemstat::System::new();
        let cpu_stat = sys.cpu_load_aggregate().ok();
        SystemStat {
            sys,
            cpu_stat,
        }
    }

    fn mem_stat(sys: &systemstat::System) -> (u64, u64) {
        let mem = sys.memory().ok();
        if let Some(mem) = mem {
            (mem.free.as_u64(), mem.total.as_u64())
        } else {
            (0, 0)
        }
    }

    fn disk_stat(sys: &systemstat::System) -> (u64, u64) {
        let disk = sys.mount_at("/").ok();
        if let Some(disk) = disk {
            (disk.avail.as_u64(), disk.total.as_u64())
        } else {
            (0, 0)
        }
    }

    pub fn snapshot(&mut self) -> SystemStatSnapshot {
        let mut cpu_user = 0.0;
        let mut cpu_system = 0.0;
        if let Some(cpu_stat) = self.cpu_stat.take() 
            && let Some(cpu_stat) = cpu_stat.done().ok() {
            cpu_user = cpu_stat.user;
            cpu_system = cpu_stat.system;
            self.cpu_stat = self.sys.cpu_load_aggregate().ok();
        }
        let (mem_free, mem_total) = Self::mem_stat(&self.sys);
        let (free_disk, total_disk) = Self::disk_stat(&self.sys);
        SystemStatSnapshot {
            cpu_user,
            cpu_system,
            mem_free,
            mem_total,
            total_disk,
            free_disk
        }
    }
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

        let stat = SystemStat::new();

        Ok(Barbar { io, font, bar_height, stat })
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
            IoEvent::DestroyOutput(event) => self.destroy_output_logic(event),
            IoEvent::RenderTimer => self.render_timer()
        }
    }

    pub fn render_logic(&mut self, event: IoRenderEvent) -> Vec<IoRequest> {
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
        surf.draw(&self.font, self.stat.snapshot());

        vec![IoRequest::Render(IoRenderRequest {
            slot,
            buffer,
            oi,
            bar_height: self.bar_height,
            render_next: false,
        })]
    }

    pub fn render_timer(&self) -> Vec<IoRequest> {
        let ois = self.io.get_all_oi();
        ois.into_iter()
            .map(|oi| IoRequest::InitRender(IoInitRender {
                oi,
                bar_height: self.bar_height,
                trigger_only: true
            }))
            .collect()
    }

    pub fn configure_logic(&mut self, event: IoOutputEvent) -> Vec<IoRequest> {
        let IoOutputEvent { oi } = event;
        let init = IoInitRender { oi, bar_height: self.bar_height, trigger_only: false };
        vec![IoRequest::InitRender(init)]
    }

    pub fn new_output_logic(&mut self, _: IoOutputEvent) -> Vec<IoRequest> {
        vec![]
    }

    pub fn destroy_output_logic(&mut self, _: IoOutputEvent) -> Vec<IoRequest> {
        vec![]
    }
}
