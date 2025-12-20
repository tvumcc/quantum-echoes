use vulkano::*;
use vulkano::swapchain::*;
use vulkano::instance::*;
use vulkano::device::*;
use vulkano::memory::allocator::*;
use vulkano::command_buffer::allocator::*;
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

use winit::window::WindowBuilder;
use winit::event_loop::*;
use winit::event::*;

use std::sync::Arc;

fn main() {
    let event_loop = EventLoop::new();
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let required_extensions = Surface::required_extensions(&event_loop);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        }
    ).expect("Failed to create instance");

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

    window.set_visible(false);
    window.set_title("Quantum Echoes");


    let physical_device = instance.enumerate_physical_devices().unwrap().next().unwrap();
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .position(|queue_family_properties| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("couldn't find a graphical queue family") as u32;

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    }; 
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions,
            ..Default::default()
        }        
    )
    .expect("failed to create device");
    let queue = queues.next().unwrap();
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default()
    ); 
    

    // Initialize the swapchain and its corresponding images
    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilties");

    let dimensions = window.inner_size();
    let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
    let image_format = physical_device
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0;

    let (mut swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count + 1,
            image_format,
            image_extent: dimensions.into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            composite_alpha,
            ..Default::default()
        }
    ).unwrap();

    // Vertex Buffer Creation
    let vertex_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        vec![
            VertexContainer {position: [-0.5, -0.5]},
            VertexContainer {position: [ 0.0,  0.5]},
            VertexContainer {position: [ 0.5, -0.25]}
        ]
    ).unwrap();

    let vs = vs::load(device.clone()).expect("failed to create vertex shader module");
    let fs = fs::load(device.clone()).expect("failed to create fragment shader module");

    let mut viewport = viewport::Viewport {
        offset: [0.0, 0.0],
        extent: window.inner_size().into(),
        depth_range: 0.0..=1.0
    };
    let render_pass = get_render_pass(device.clone(), &swapchain);
    let framebuffers = get_framebuffers(&images, &render_pass);
    let pipeline = get_pipeline(
        device.clone(),
        vs.clone(),
        fs.clone(),
        render_pass.clone(),
        viewport.clone()
    );
    let mut command_buffers = get_command_buffers(
        &command_buffer_allocator,
        &queue,
        &pipeline,
        &framebuffers,
        &vertex_buffer
    );

    let mut window_resized = false;
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

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
                window_resized = true;
            }
            Event::MainEventsCleared => {
                let image_extent: [u32; 2] = window.inner_size().into();
                if image_extent.contains(&0) {
                    return;
                }

                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                    
                    let new_dimensions = window.inner_size();

                    let (new_swapchain, new_images) = swapchain
                        .recreate(SwapchainCreateInfo {
                            image_extent: new_dimensions.into(),
                            ..swapchain.create_info()
                        })
                        .expect("failed to recreate swapchain: {e}");
                    swapchain = new_swapchain;
                    let new_framebuffers = get_framebuffers(&new_images, &render_pass);

                    if window_resized {
                        window_resized = false;

                        viewport.extent = new_dimensions.into();
                        let new_pipeline = get_pipeline(
                            device.clone(),
                            vs.clone(),
                            fs.clone(),
                            render_pass.clone(),
                            viewport.clone()
                        );
                        command_buffers = get_command_buffers(
                            &command_buffer_allocator,
                            &queue,
                            &new_pipeline,
                            &new_framebuffers,
                            &vertex_buffer
                        );
                    }
                }

                let (image_i, suboptimal, acquire_future) = 
                    match swapchain::acquire_next_image(swapchain.clone(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                recreate_swapchain = suboptimal;

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffers[image_i as usize].clone())
                    .unwrap()
                    .then_swapchain_present(
                        queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                    )
                    .then_signal_fence_and_flush();

                match future.map_err(Validated::unwrap) {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                    }
                }

                if !window.is_visible().unwrap() {
                    window.set_visible(true);
                }
            }
            _ => ()
        }
    }); 
}

fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
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
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: viewport::Viewport
) -> Arc<GraphicsPipeline> {
    let vs = vs.entry_point("main").unwrap();
    let fs = fs.entry_point("main").unwrap();

    let vertex_input_state = VertexContainer::per_vertex()
        .definition(&vs.info().input_interface)
        .unwrap();

    let stages = [
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs)
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .unwrap()
    ).unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    GraphicsPipeline::new(
        device.clone(),
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
    command_buffer_allocator: &StandardCommandBufferAllocator,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    framebuffers: &Vec<Arc<Framebuffer>>,
    vertex_buffer: &Subbuffer<[VertexContainer]>
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

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
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(SubpassEndInfo::default())
                .unwrap();

            builder.build().unwrap()
        })
        .collect() 
}

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

#[derive(BufferContents, vertex_input::Vertex)]
#[repr(C)]
struct VertexContainer {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}