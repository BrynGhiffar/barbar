use std::collections::VecDeque;

use anyhow::Context;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState}, delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_shm, output::{OutputHandler, OutputState}, registry::{ProvidesRegistryState, RegistryState}, registry_handlers, seat::{Capability, SeatHandler, SeatState, keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers}, pointer::{PointerEvent, PointerHandler}}, shell::{WaylandSurface, wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure}}, shm::{Shm, ShmHandler, slot::SlotPool}
};

use wayland_client::{
    Connection, EventQueue, QueueHandle, globals::registry_queue_init, protocol::{wl_keyboard::WlKeyboard, wl_output::{Transform, WlOutput}, wl_pointer::WlPointer, wl_seat::WlSeat, wl_shm, wl_surface::WlSurface}
};
use crate::io::event::{OutputInfo, IoEvent, IoOutputEvent, IoRenderEvent, IoRenderRequest};


#[derive(Debug)]
pub struct Output {
    pub name: String,
    pub layer: LayerSurface,
    pub width: i32,
    pub height: i32,
    pub configured: bool,
}

impl Output {
    pub fn to_oi(&self) -> OutputInfo {
        OutputInfo { name: self.name.clone(), width: self.width, height: self.height }
    }
}

pub struct LayerIO {
    pub conn: Connection,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub layer_shell: LayerShell,
    pub compositor_state: CompositorState,
    pub shm: Shm,
    pub qh: QueueHandle<Self>,
    pub outputs: Vec<Output>,
    pub io_queue: VecDeque<IoEvent>,
    pub bar_height: i32
}

impl LayerIO {
    pub fn new() -> anyhow::Result<(Self, EventQueue<Self>)> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<Self>(&conn)?;
        let qh = event_queue.handle();

        let compositor_state = CompositorState::bind(&globals, &qh)?;

        let shm = Shm::bind(&globals, &qh)?;

        let layer_shell = LayerShell::bind(&globals, &qh)?;

        let bar_height = 30;

        Ok((Self {
            conn,
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            layer_shell,
            compositor_state,
            qh,
            shm,
            outputs: vec![],
            io_queue: VecDeque::new(),
            bar_height
        }, event_queue))
    }

    pub fn init_render(&self, oi: OutputInfo) -> anyhow::Result<()> {
        let output = self.get_output_by_name(&oi.name).context("Output not found")?;
        let mut slot = SlotPool::new(1, &self.shm)?;

        let (buffer, _) = slot
            .create_buffer(oi.width, self.bar_height, oi.width * 4, wl_shm::Format::Argb8888)?;

        buffer.attach_to(output.layer.wl_surface())?;
        output
            .layer
            .wl_surface()
            .damage_buffer(0, 0, oi.width, self.bar_height);

        output.layer.wl_surface().frame(&self.qh, output.layer.wl_surface().clone());
        output.layer.commit();
        Ok(())
    }

    pub fn render(&mut self, IoRenderRequest {
        slot: _slot,
        buffer,
        oi,
        render_next
    }: IoRenderRequest) -> anyhow::Result<()> {
        let output = self.get_output_by_name(&oi.name).context("Output not found")?;
        if !output.configured {
            tracing::error!("output needs to be configured before rendered");
            return Ok(())
        }
        buffer.attach_to(output.layer.wl_surface())?;
        output.layer.wl_surface().damage_buffer(0, 0, output.width, self.bar_height);
        if render_next {
            output.layer.wl_surface().frame(&self.qh, output.layer.wl_surface().clone());
        }
        output.layer.commit();
        Ok(())
    }


    pub fn get_output_by_name(&self, name: &str) -> Option<&Output> {
        self.outputs
            .iter()
            .find(|out| out.name == name)
    }

    fn get_output_by_surface(&self, surface: &WlSurface) -> Option<&Output> {
        self.outputs
            .iter()
            .find(|out| out.layer.wl_surface() == surface)
    }

    fn get_output_by_surface_mut(&mut self, surface: &WlSurface) -> Option<&mut Output> {
        self.outputs
            .iter_mut()
            .find(|out| out.layer.wl_surface() == surface)
    }

    fn extract_output_info(&self, output: &WlOutput) -> anyhow::Result<OutputInfo> {
        let output_info = self
            .output_state
            .info(output)
            .context("Failed to get output info")?;
        let monitor_name = output_info
            .name
            .context("Failed to get monitor_name")?
            .clone();
        let (width, height) = output_info
            .logical_size
            .context("Failed to get monitor width and height")?;

        Ok(OutputInfo {
            name: monitor_name,
            width,
            height,
        })
    }
}

impl CompositorHandler for LayerIO {
    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        let output = self.get_output_by_surface(surface);
        let Some(output) = output else {
            tracing::error!("LOST_FRAME: Cannot render frame on uninitialized output");
            return;
        };
        let slot = match SlotPool::new(1, &self.shm) {
            Ok(pool) => pool,
            Err(err) => {
                tracing::error!("LOST_FRAME: Cannot create slot pool: {err}");
                return;
            },
        };

        self.io_queue
            .push_back(IoEvent::Render(IoRenderEvent { oi: output.to_oi(), slot }));
    }

    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: Transform,
    ) {
        // Not needed for this example.
    }
    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
        // Not needed for this example.
    }
}

impl OutputHandler for LayerIO {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: WlOutput,
    ) {
        let output_info = self.extract_output_info(&output);
        let Ok(output_info) = output_info else {
            tracing::error!("Failed to extract output info");
            return;
        };
        tracing::info!("new output event: {:?}", &output_info);
        let surface = self.compositor_state.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Bottom,
            Some("barbar"),
            Some(&output),
        );
        layer.set_margin(0, 0, self.bar_height, 0);
        layer.set_exclusive_zone(self.bar_height + 5);
        layer.set_anchor(Anchor::TOP);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(output_info.width as u32, self.bar_height as u32);
        layer.commit();

        let output = Output {
            name: output_info.name.clone(),
            width: output_info.width,
            height: output_info.height,
            layer,
            configured: false,
        };

        let oi = output.to_oi();
        self.outputs.push(output);
        self.io_queue.push_back(IoEvent::NewOutput(IoOutputEvent { oi }))
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: WlOutput,
    ) {
        tracing::info!("UPDATE OUTPUT EVENT");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput,
    ) {
        let output_info = self.extract_output_info(&output);
        let Ok(oi) = output_info else {
            tracing::error!("Failed to extract output info");
            return;
        };
        self.outputs.retain(|output | output.name != oi.name);
        tracing::info!("Destroyed Output Event: {:?}", oi);
        self.io_queue.push_back(IoEvent::DestroyOutput(IoOutputEvent { oi }));
    }
}

impl LayerShellHandler for LayerIO {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {

        let output = self.get_output_by_surface_mut(layer.wl_surface());
        let Some(output) = output else {
            return;
        };
        if output.configured {
            return;
        }
        output.configured = true;
        let oi = output.to_oi();
        self.io_queue.push_back(IoEvent::ConfigureOutput(IoOutputEvent { oi }))
    }
}

impl SeatHandler for LayerIO {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        _: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        _: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl KeyboardHandler for LayerIO {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
    }
}

impl PointerHandler for LayerIO {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        _events: &[PointerEvent],
    ) {
    }
}

impl ShmHandler for LayerIO {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for LayerIO {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(LayerIO);
delegate_output!(LayerIO);
delegate_seat!(LayerIO);
delegate_keyboard!(LayerIO);
delegate_pointer!(LayerIO);
delegate_shm!(LayerIO);
delegate_layer!(LayerIO);
delegate_registry!(LayerIO);
