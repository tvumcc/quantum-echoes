use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::*;
use vulkano::swapchain::*;
use vulkano::memory::allocator::*;
use vulkano::command_buffer::*;
use vulkano::pipeline::*;
use vulkano::pipeline::layout::*;
use vulkano::pipeline::graphics::*;
use vulkano::pipeline::graphics::vertex_input::*;
use vulkano::buffer::*;
use vulkano::image::*;
use vulkano::image::view::*;
use vulkano::sync::{self, GpuFuture};
use vulkano::render_pass::*;
use vulkano::shader::*;

use std::sync::Arc;

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
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,

    viewport: viewport::Viewport,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    pipeline: Arc<GraphicsPipeline>,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,

    descriptor_set: Arc<DescriptorSet>,

    vertex_buffer: Subbuffer<[VertexContainer]>,

    pub window_resized: bool,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>
}

impl QuadRenderer {
    pub fn new(mgr: &VulkanManager, simulator: &Simulator) -> Self {
        mod vs {
            vulkano_shaders::shader! {
                ty: "vertex",
                path: "vert.glsl"
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: "frag.glsl"
            }
        }

        let vertex_buffer = Buffer::from_iter(
            mgr.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![
                VertexContainer {position: [-1.0, -1.0], uv: [0.0, 0.0]},
                VertexContainer {position: [-1.0,  1.0], uv: [0.0, 1.0]},
                VertexContainer {position: [ 1.0,  1.0], uv: [1.0, 1.0]},
                VertexContainer {position: [-1.0, -1.0], uv: [0.0, 0.0]},
                VertexContainer {position: [ 1.0,  1.0], uv: [1.0, 1.0]},
                VertexContainer {position: [ 1.0, -1.0], uv: [1.0, 0.0]}
            ]
        ).unwrap();

        let vs = vs::load(mgr.context.device().clone()).expect("failed to create vertex shader module");
        let fs = fs::load(mgr.context.device().clone()).expect("failed to create fragment shader module");

        let (swapchain, images) = QuadRenderer::get_swapchain(mgr);

        let viewport = Self::get_viewport(mgr, 0.0);
        let render_pass = Self::get_render_pass(mgr, &swapchain);
        let framebuffers = Self::get_framebuffers(&images, &render_pass);
        let pipeline = Self::get_pipeline(mgr, &vs, &fs, &render_pass, viewport.clone());

        let descriptor_set = Self::get_descriptor_set(mgr, simulator, &pipeline);

        let command_buffers = Self::get_command_buffers(mgr, &pipeline, &framebuffers, &vertex_buffer, &descriptor_set);

        QuadRenderer {
            vertex_shader: vs,
            fragment_shader: fs,
            swapchain,
            images,

            viewport,
            render_pass,
            framebuffers,
            pipeline,
            command_buffers,
            descriptor_set,

            vertex_buffer,
            window_resized: false,
            recreate_swapchain: false,
            previous_frame_end: Some(sync::now(mgr.context.device().clone()).boxed())
        }
    }

    pub fn draw(&mut self, mgr: &VulkanManager, simulator: &Simulator, ui_state: &mut UIState) {
        let image_extent: [u32; 2] = mgr.windows.get_primary_window().unwrap().inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.window_resized || self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.recreate_swapchain(mgr);
            
            if self.window_resized {
                self.window_resized = false;
                self.update_pipeline_and_command_buffers(mgr, simulator, &ui_state);
            }
        }

        let (image_i, suboptimal, acquire_future) = 
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        self.recreate_swapchain = suboptimal;

        let future = self.previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(mgr.context.graphics_queue().clone(), self.command_buffers[image_i as usize].clone())
            .unwrap();

        let after_future = ui_state.gui.draw_on_image(future, ImageView::new_default(self.images[image_i as usize].clone()).unwrap())
            .then_swapchain_present(
                mgr.context.graphics_queue().clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();


        match after_future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(mgr.context.device().clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
            }
        }

        if !mgr.windows.get_primary_window().unwrap().is_visible().unwrap() {
            mgr.windows.get_primary_window().unwrap().set_visible(true);
        }
    }

    pub fn update_pipeline_and_command_buffers(&mut self, mgr: &VulkanManager, simulator: &Simulator, ui_state: &UIState) {
        self.viewport = Self::get_viewport(mgr, ui_state.gui_width);
        self.pipeline = Self::get_pipeline(
            mgr,
            &self.vertex_shader,
            &self.fragment_shader,
            &self.render_pass,
            self.viewport.clone()
        );
        self.descriptor_set = Self::get_descriptor_set(mgr, simulator, &self.pipeline);
        self.command_buffers = Self::get_command_buffers(
            mgr,
            &self.pipeline,
            &self.framebuffers,
            &self.vertex_buffer,
            &self.descriptor_set
        );
    }

    pub fn recreate_swapchain(&mut self, mgr: &VulkanManager) {
        let new_dimensions = mgr.windows.get_primary_window().unwrap().inner_size();
        (self.swapchain, self.images) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: new_dimensions.into(),
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain: {e}");
        self.framebuffers = Self::get_framebuffers(&self.images, &self.render_pass);
    }

    fn get_swapchain(mgr: &VulkanManager) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
        let surface = mgr.windows.get_primary_renderer().unwrap().surface();

        let caps = mgr.context.device().physical_device() 
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilties");

        let dimensions = mgr.windows.get_primary_window().unwrap().inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = mgr.context.device().physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        Swapchain::new(
            mgr.context.device().clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        ).unwrap()
    }

    fn get_viewport(mgr: &VulkanManager, gui_width: f32) -> viewport::Viewport {
        viewport::Viewport {
            offset: [gui_width, 0.0],
            extent: [((mgr.windows.get_primary_window().expect("HEY").inner_size().width) as f32 - gui_width).max(gui_width), mgr.windows.get_primary_window().expect("HEY").inner_size().height as f32],
            depth_range: 0.0..=1.0
        }
    }

    fn get_render_pass(mgr: &VulkanManager, swapchain: &Arc<Swapchain>) -> Arc<RenderPass>{
        vulkano::single_pass_renderpass!(
            mgr.context.device().clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            },
        ).unwrap()
    }

    fn get_framebuffers(images: &[Arc<Image>], render_pass: &Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
        images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                ).unwrap()
            })
            .collect::<Vec<_>>()
    }

    fn get_pipeline(
        mgr: &VulkanManager,
        vs: &Arc<ShaderModule>,
        fs: &Arc<ShaderModule>,
        render_pass: &Arc<RenderPass>,
        viewport: viewport::Viewport
    ) -> Arc<GraphicsPipeline> {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let vertex_input_state = VertexContainer::per_vertex()
            .definition(&vs)
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs)
        ];

        let layout = PipelineLayout::new(
            mgr.context.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(mgr.context.device().clone())
                .unwrap()
        ).unwrap();

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
                    color_blend::ColorBlendAttachmentState::default()
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            }
        ).unwrap()
    }

    fn get_command_buffers(
        mgr: &VulkanManager,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffers: &Vec<Arc<Framebuffer>>,
        vertex_buffer: &Subbuffer<[VertexContainer]>,
        descriptor_set: &Arc<DescriptorSet>
    ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers
            .iter()
            .map(|framebuffer| {
                let mut builder = AutoCommandBufferBuilder::primary(
                    mgr.command_buffer_allocator.clone(),
                    mgr.context.graphics_queue().queue_family_index(),
                    CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                unsafe {
                    builder
                        .begin_render_pass(
                            RenderPassBeginInfo {
                                clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                                ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                            },
                            SubpassBeginInfo {
                                contents: SubpassContents::Inline,
                                ..Default::default()
                            },
                        )
                        .unwrap()
                        .bind_pipeline_graphics(pipeline.clone())
                        .unwrap()
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            pipeline.layout().clone(),
                            0,
                            descriptor_set.clone() 
                        )
                        .unwrap()
                        .bind_vertex_buffers(0, vertex_buffer.clone())
                        .unwrap()
                        .draw(vertex_buffer.len() as u32, 1, 0, 0)
                        .unwrap()
                        .end_render_pass(SubpassEndInfo::default())
                        .unwrap();
                }

                builder.build().unwrap()
            })
            .collect() 
    }

    fn get_descriptor_set(mgr: &VulkanManager, simulator: &Simulator, pipeline: &Arc<GraphicsPipeline>) -> Arc<DescriptorSet> {
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
