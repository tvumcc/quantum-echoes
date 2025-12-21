use vulkano::image::view::ImageView;
use vulkano::render_pass::Framebuffer;
use vulkano::render_pass::FramebufferCreateInfo;
use vulkano::*;
use vulkano::swapchain::*;
use vulkano::instance::*;
use vulkano::device::*;
use vulkano::device::physical::*;
use vulkano::memory::allocator::*;
use vulkano::command_buffer::allocator::*;
use vulkano::descriptor_set::allocator::*;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::*;
use winit::event_loop::{EventLoop, ActiveEventLoop};

use egui_winit_vulkano::egui;
use egui_winit_vulkano::Gui;
use egui_winit_vulkano::GuiConfig;
use egui::{ScrollArea, TextEdit, TextStyle};

use vulkano::sync::{self, GpuFuture};

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};

use std::sync::Arc;

use crate::quad_renderer::QuadRenderer;
use crate::simulator::Simulator;


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

    gui: Option<Gui>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.mgr.windows.create_window(event_loop, &self.mgr.context, &WindowDescriptor::default(), |create_info| {
            // create_info.image_format = format::Format::R8G8B8A8_UNORM;
            create_info.min_image_count = create_info.min_image_count.max(2);
        });

        self.simulator = Some(Simulator::new(&self.mgr));
        self.renderer = Some(QuadRenderer::new(&self.mgr, self.simulator.as_ref().unwrap()));

        let gui_config = GuiConfig {
            allow_srgb_render_target: true,
            is_overlay: true,
            ..Default::default()
        };

        self.gui = Some({
            let renderer = self.mgr.windows.get_primary_renderer_mut().unwrap();
            Gui::new(
                event_loop,
                renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                gui_config,
            )
        });

        self.simulator.as_ref().unwrap().compute(&self.mgr);
    } 

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.gui.as_mut().unwrap().update(&event);
        let renderer = self.mgr.windows.get_renderer_mut(id).unwrap();
        let quad_renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(_) => {
                quad_renderer.window_resized = true;
            },
            WindowEvent::ScaleFactorChanged { .. } => {
                quad_renderer.window_resized = true;
            }
            WindowEvent::RedrawRequested => {
                self.gui.as_mut().unwrap().immediate_ui(|gui| {
                    let ctx = gui.context();
                    // egui::Window::new("Hello World")
                    //     .show(&ctx, |ui| {
                    //         ui.label("sup");
                    //     });
                        
                    egui::SidePanel::right("Hello").show(&ctx, |ui| {
                        ui.heading("Hello");
                        ui.vertical_centered(|ui| {
                            ui.add(egui::widgets::Label::new("Hi there!"));
                        });
                        ui.separator();
                    });
                });

                quad_renderer.draw(&self.mgr, self.gui.as_mut().unwrap());
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.mgr.windows.get_primary_window().as_mut().unwrap().request_redraw();
    }
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Arc<Self> {
        let mgr = VulkanManager::new();

        Arc::new(App {
            mgr,
            renderer: None,
            simulator: None,
            gui: None,
        })
    }
}
