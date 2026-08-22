use tracing_subscriber::EnvFilter;

use crate::barbar::Barbar;

pub mod io;
pub mod barbar;
pub mod surface;
pub mod ext;
pub mod stat;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        ).init();
    let mut app = Barbar::new()?;
    app.run()?;
    Ok(())
}
