use winit::event_loop::*;
use winit::event::*;

mod app;
mod quad_renderer;

use quad_renderer::QuadRenderer;
use app::App;

fn main() {
    let event_loop = EventLoop::new();
    let app = App::new(&event_loop, "Quantum Echoes");
    let mut renderer = QuadRenderer::new(&app);

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

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute.glsl"
    }
}