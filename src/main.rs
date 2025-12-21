use winit::event_loop::*;

use std::sync::Arc;

mod app;
mod quad_renderer;
mod simulator;

use app::App;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);
    let _ = event_loop.run_app(Arc::get_mut(&mut app).unwrap());
}