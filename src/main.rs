use winit::event_loop::*;
use winit::event::*;

mod app;
mod quad_renderer;
mod simulator;

use quad_renderer::QuadRenderer;
use app::App;
use simulator::Simulator;

fn main() {
    let event_loop = EventLoop::new();
    let app = App::new(&event_loop, "Quantum Echoes");
    let simulator = Simulator::new(&app);
    let mut renderer = QuadRenderer::new(&app, &simulator);
    simulator.compute(&app);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                renderer.window_resized = true;
            }
            Event::MainEventsCleared => {
                renderer.draw(&app);
            }
            _ => ()
        }
    }); 
}