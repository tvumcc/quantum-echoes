use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, allocator::*,
};
use vulkano::descriptor_set::allocator::*;
use vulkano::memory::allocator::*;
use vulkano::sync::{self, GpuFuture};

use winit::application::ApplicationHandler;
use winit::event::{MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::*;

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};

use std::sync::Arc;
use std::time::Instant;

use crate::quad_renderer::QuadRenderer;
use crate::simulator::Simulator;
use crate::ui_state::UIState;

pub struct VulkanManager {
    pub context: VulkanoContext,
    pub windows: VulkanoWindows,

    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl VulkanManager {
    pub fn new() -> Self {
        let context = VulkanoContext::new(VulkanoConfig::default());
        let windows = VulkanoWindows::default();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
            context.device().clone(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            context.device().clone(),
            StandardDescriptorSetAllocatorCreateInfo::default(),
        ));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            context.device().clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));

        VulkanManager {
            context,
            windows,

            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
        }
    }

    pub fn get_compute_cmdbuffer_builder(
        &self,
    ) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.context.compute_queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
    }

    pub fn execute_compute_cmdbuffer_from_builder(
        &self,
        builder: AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        let command_buffer = builder.build().unwrap();

        let future = sync::now(self.context.device().clone())
            .then_execute(self.context.compute_queue().clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();
    }
}

pub struct App {
    pub mgr: VulkanManager,
    pub renderer: Option<QuadRenderer>,
    pub simulator: Option<Simulator>,
    pub ui_state: Option<UIState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.mgr.windows.create_window(
            event_loop,
            &self.mgr.context,
            &WindowDescriptor {
                title: String::from("Quantum Echoes"),
                ..Default::default()
            },
            |_| {},
        );

        self.simulator = Some(Simulator::new(&self.mgr));
        self.ui_state = Some(UIState::new(&event_loop, &mut self.mgr));
        self.renderer = Some(QuadRenderer::new(
            &self.mgr,
            self.simulator.as_ref().unwrap(),
            self.ui_state.as_ref().unwrap(),
        ));

        self.simulator
            .as_ref()
            .unwrap()
            .compute(&self.mgr, self.ui_state.as_ref().unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let ui_state = self.ui_state.as_mut().unwrap();
        let simulator = self.simulator.as_mut().unwrap();
        ui_state.handle_event(&event);
        let quad_renderer = self.renderer.as_mut().unwrap();

        let resolution = 5;

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                quad_renderer.window_resized = true;
                quad_renderer.last_resize_event = Instant::now();
                let window = self.mgr.windows.get_primary_window().unwrap();
                let gui_width = ui_state.gui_width;
                simulator.resize(
                    &self.mgr,
                    (window.inner_size().width as f32 - gui_width).max(gui_width) as u32 / resolution,
                    window.inner_size().height as u32 / resolution,
                );
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if state.is_pressed() && button == MouseButton::Left {
                    ui_state.brush_enabled = 1;
                } else if !state.is_pressed() && button == MouseButton::Left {
                    ui_state.brush_enabled = 0;
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    ui_state.theta += 0.25 * y as f32;
                }
                MouseScrollDelta::PixelDelta(u) => {
                    ui_state.theta += 0.1 * u.y as f32;
                }
            },
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                ui_state.brush_x =
                    ((position.x - ui_state.gui_width as f64) / resolution as f64) as i32;
                ui_state.brush_y = (position.y / resolution as f64) as i32;
                ui_state.mouse_x = position.x as f32;
                ui_state.mouse_y = position.y as f32;
            }
            WindowEvent::RedrawRequested => {
                ui_state.setup_gui(&self.mgr, simulator);

                simulator.compute(&self.mgr, ui_state);
                quad_renderer.draw(&mut self.mgr, simulator, ui_state);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.mgr
            .windows
            .get_primary_window()
            .as_mut()
            .unwrap()
            .request_redraw();
    }
}

impl App {
    pub fn new(_event_loop: &EventLoop<()>) -> Arc<Self> {
        let mgr = VulkanManager::new();

        Arc::new(App {
            mgr,
            renderer: None,
            simulator: None,
            ui_state: None,
        })
    }
}
