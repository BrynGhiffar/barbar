use crate::barbar::Barbar;

pub mod io;
pub mod barbar;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let mut app = Barbar::new()?;
    app.run()?;
    Ok(())
}
