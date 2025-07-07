mod app;
mod image;
mod map;

use macroquad::prelude::*;
use anyhow::{Context, Result};

#[macroquad::main("Thermal Image Viewer")]
async fn main() -> Result<()> {
    let mut app = app::App::new();

    loop {
        app.update().await.context("Failed to update app")?;
        app.draw().await.context("Failed to draw app")?;
        next_frame().await
    }
}
