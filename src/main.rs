mod app;

use macroquad::prelude::*;

#[macroquad::main("MyGame")]
async fn main() {
    let mut app = app::App::new();

    loop {
        app.update().await;
        app.draw().await;
        next_frame().await
    }
}
