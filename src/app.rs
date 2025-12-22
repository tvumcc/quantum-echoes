use vulkano::memory::allocator::*;
use vulkano::command_buffer::allocator::*;
use vulkano::descriptor_set::allocator::*;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::*;
use winit::event_loop::{EventLoop, ActiveEventLoop};

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};

use std::sync::Arc;

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

        let memory_allocator         = Arc::new(StandardMemoryAllocator::new_default(context.device().clone()));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(context.device().clone(), StandardDescriptorSetAllocatorCreateInfo::default())); 
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(context.device().clone(), StandardCommandBufferAllocatorCreateInfo::default())); 

        VulkanManager {
            context,
            windows,

            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator 
        }
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
        self.mgr.windows.create_window(event_loop, &self.mgr.context, &WindowDescriptor {
            title: String::from("Quantum Echoes"),
            ..Default::default()
        }, |_| {});

        self.simulator = Some(Simulator::new(&self.mgr));
        self.renderer = Some(QuadRenderer::new(&self.mgr, self.simulator.as_ref().unwrap()));
        self.ui_state = Some(UIState::new(&event_loop, &mut self.mgr));

        self.simulator.as_ref().unwrap().compute(&self.mgr);
    } 

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.ui_state.as_mut().unwrap().handle_event(&event);
        let quad_renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                quad_renderer.window_resized = true;
            },
            WindowEvent::RedrawRequested => {
                self.ui_state.as_mut().unwrap().setup_gui();

                quad_renderer.draw(&self.mgr, self.ui_state.as_mut().unwrap());
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.mgr.windows.get_primary_window().as_mut().unwrap().request_redraw();
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
