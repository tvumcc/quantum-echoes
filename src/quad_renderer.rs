use vulkano::buffer::*;
use vulkano::command_buffer::*;
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::image::view::*;
use vulkano::memory::allocator::*;
use vulkano::pipeline::graphics::vertex_input::*;
use vulkano::pipeline::graphics::*;
use vulkano::pipeline::layout::*;
use vulkano::pipeline::*;
use vulkano::render_pass::*;
use vulkano::shader::*;
use vulkano::sync::GpuFuture;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::app::VulkanManager;
use crate::simulator::Simulator;
use crate::ui_state::UIState;

#[derive(BufferContents, vertex_input::Vertex)]
#[repr(C)]
struct VertexContainer {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    uv: [f32; 2],
}

pub struct QuadRenderer {
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,

    viewport: viewport::Viewport,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    pipeline: Arc<GraphicsPipeline>,

    descriptor_set: Arc<DescriptorSet>,

    vertex_buffer: Subbuffer<[VertexContainer]>,

    pub window_resized: bool,
    pub last_resize_event: Instant,
}

impl QuadRenderer {
    pub fn new(mgr: &VulkanManager, simulator: &Simulator, ui_state: &UIState) -> Self {
        let window_renderer = mgr.windows.get_primary_renderer().unwrap();
        let window_size = window_renderer.window().inner_size();
        
        let vertex_buffer = Buffer::from_iter(
            mgr.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![
                VertexContainer {position: [-1.0, -1.0], uv: [0.0, 0.0]},
                VertexContainer {position: [-1.0, 1.0],  uv: [0.0, 1.0]},
                VertexContainer {position: [1.0, 1.0],   uv: [1.0, 1.0]},
                VertexContainer {position: [-1.0, -1.0], uv: [0.0, 0.0]},
                VertexContainer {position: [1.0, 1.0],   uv: [1.0, 1.0]},
                VertexContainer {position: [1.0, -1.0],  uv: [1.0, 0.0]},
            ],
        )
        .unwrap();

        let vs = vs::load(mgr.context.device().clone()).expect("failed to create vertex shader module");
        let fs = fs::load(mgr.context.device().clone()).expect("failed to create fragment shader module");

        let viewport = Self::get_viewport(window_size.width as f32, window_size.height as f32, ui_state.gui_width);
        let render_pass = Self::get_render_pass(mgr);
        let framebuffers = Self::get_framebuffers(window_renderer.swapchain_image_views(), &render_pass);
        let pipeline = Self::get_pipeline(mgr, &vs, &fs, &render_pass, viewport.clone());

        let descriptor_set = Self::get_descriptor_set(mgr, simulator, &pipeline);

        QuadRenderer {
            vertex_shader: vs,
            fragment_shader: fs,
            
            viewport,
            render_pass,
            framebuffers,
            pipeline,
            descriptor_set,

            vertex_buffer,
            window_resized: false,
            last_resize_event: Instant::now(),
        }
    }

    pub fn draw(&mut self, mgr: &mut VulkanManager, simulator: &Simulator, ui_state: &mut UIState) {
        let mut recreate_swapchain = false;
        let previous_frame_end = mgr.windows.get_primary_renderer_mut().unwrap()
            .acquire(Some(Duration::from_millis(1000)), |_| {
                recreate_swapchain = true;
            })
            .unwrap();
        
        if recreate_swapchain || self.window_resized {
            if self.window_resized {
                if self.last_resize_event.elapsed().as_millis() > 30 {
                    mgr.windows.get_primary_renderer_mut().unwrap().resize();
                    self.window_resized = false;
                }
            }
            self.update(mgr, simulator, ui_state);
        }
        
        let command_buffer = self.command_buffer(&mgr, ui_state);
        
        let mut future = previous_frame_end
            .then_execute(mgr.context.graphics_queue().clone(), command_buffer.clone())
            .unwrap()
            .boxed();
        
        let window_renderer = mgr.windows.get_primary_renderer_mut().unwrap();
        future = ui_state.gui
            .draw_on_image(future, window_renderer.swapchain_image_view());
        
        window_renderer.present(future, false);
    }

    pub fn update(&mut self, mgr: &VulkanManager, simulator: &Simulator, ui_state: &UIState) {
        let window_renderer = mgr.windows.get_primary_renderer().unwrap();
        let window_size = window_renderer.window().inner_size();
        
        self.viewport = Self::get_viewport(window_size.width as f32, window_size.height as f32, ui_state.gui_width);
        self.pipeline = Self::get_pipeline(
            mgr,
            &self.vertex_shader,
            &self.fragment_shader,
            &self.render_pass,
            self.viewport.clone(),
        );
        self.descriptor_set = Self::get_descriptor_set(mgr, simulator, &self.pipeline);
        self.framebuffers = Self::get_framebuffers(window_renderer.swapchain_image_views(), &self.render_pass);
    }

    pub fn command_buffer(&self, mgr: &VulkanManager, ui_state: &UIState) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            mgr.command_buffer_allocator.clone(),
            mgr.context.graphics_queue().queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )
        .unwrap();

        let push_constants = fs::PushConstantData {
            visible_layer: ui_state.visible_layer as i32,
        };
        
        let window_renderer = mgr.windows.get_primary_renderer().unwrap();
        
        unsafe {
            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                        ..RenderPassBeginInfo::framebuffer(self.framebuffers[window_renderer.image_index() as usize].clone())
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    },
                )
                .unwrap()
                .bind_pipeline_graphics(self.pipeline.clone())
                .unwrap()
                .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    self.descriptor_set.clone(),
                )
                .unwrap()
                .bind_vertex_buffers(0, self.vertex_buffer.clone())
                .unwrap()
                .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(SubpassEndInfo::default())
                .unwrap();
        }

        builder.build().unwrap()
    }

    fn get_viewport(window_width: f32, window_height: f32, gui_width: f32) -> viewport::Viewport {
        viewport::Viewport {
            offset: [gui_width, 0.0],
            extent: [(window_width - gui_width).max(gui_width), window_height],
            depth_range: 0.0..=1.0,
        }
    }

    fn get_render_pass(mgr: &VulkanManager) -> Arc<RenderPass> {
        let window_renderer = mgr.windows.get_primary_renderer().unwrap();
        
        vulkano::single_pass_renderpass!(
            mgr.context.device().clone(),
            attachments: {
                color: {
                    format: window_renderer.swapchain_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            },
        )
        .unwrap()
    }

    fn get_framebuffers(image_views: &[Arc<ImageView>], render_pass: &Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
        image_views
            .iter()
            .map(|view| {
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    fn get_pipeline(mgr: &VulkanManager, vs: &Arc<ShaderModule>, fs: &Arc<ShaderModule>, render_pass: &Arc<RenderPass>, viewport: viewport::Viewport) -> Arc<GraphicsPipeline> {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let vertex_input_state = VertexContainer::per_vertex().definition(&vs).unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            mgr.context.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(mgr.context.device().clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        GraphicsPipeline::new(
            mgr.context.device().clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(input_assembly::InputAssemblyState::default()),
                viewport_state: Some(viewport::ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(rasterization::RasterizationState::default()),
                multisample_state: Some(multisample::MultisampleState::default()),
                color_blend_state: Some(color_blend::ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    color_blend::ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    fn get_descriptor_set(
        mgr: &VulkanManager,
        simulator: &Simulator,
        pipeline: &Arc<GraphicsPipeline>,
    ) -> Arc<DescriptorSet> {
        let layout = &pipeline.layout().set_layouts()[0];
        DescriptorSet::new(
            mgr.descriptor_set_allocator.clone(),
            layout.clone(),
            [
                WriteDescriptorSet::sampler(0, simulator.grid_sampler.clone()),
                WriteDescriptorSet::image_view(1, simulator.grid_view.clone()),
            ],
            [],
        )
        .unwrap()
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vert.glsl"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/frag.glsl"
    }
}
