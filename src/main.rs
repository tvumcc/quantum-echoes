use winit::event_loop::*;
use winit::event::*;

use std::sync::Arc;

mod app;
mod quad_renderer;
mod simulator;

use quad_renderer::QuadRenderer;
use app::App;
use simulator::Simulator;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);

    event_loop.run_app(Arc::get_mut(&mut app).unwrap());
}