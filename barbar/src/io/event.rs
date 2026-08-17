use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
// use wpdm_common::CliRequest;

#[derive(Debug)]
pub enum IoEvent {
    Render(IoRenderEvent),
    ConfigureOutput(IoOutputEvent),
    NewOutput(IoOutputEvent),
    DestroyOutput(IoOutputEvent),
    RenderTimer
    // CliRequest(CliRequest)
}

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub struct IoOutputEvent {
    pub oi: OutputInfo
}

#[derive(Debug)]
pub struct IoRenderEvent {
    pub slot: SlotPool,
    pub oi: OutputInfo
}

#[derive(Debug)]
pub enum IoRequest {
    InitRender(IoInitRender),
    Render(IoRenderRequest)
}

#[derive(Debug)]
pub struct IoInitRender {
    pub oi: OutputInfo,
    pub bar_height: i32,
    pub trigger_only: bool
}

#[derive(Debug)]
pub struct IoRenderRequest {
    pub slot: SlotPool,
    pub buffer: Buffer,
    pub oi: OutputInfo,
    pub bar_height: i32,
    pub render_next: bool
}
