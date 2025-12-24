use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::image::sampler::{Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::*;
use vulkano::memory::allocator::*;
use vulkano::format::*;
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo};

use vulkano::sync::{self, GpuFuture};

use std::sync::Arc;

use crate::app::VulkanManager;
use crate::ui_state::UIState;

pub struct Simulator {
    grid_u: Arc<Image>,
    pub grid_view: Arc<ImageView>,
    pub grid_sampler: Arc<Sampler>,

    pipeline: Arc<ComputePipeline>,

    pub width: u32,
    pub height: u32
}

impl Simulator {
    pub fn new(mgr: &VulkanManager) -> Self {
        let grid_u = Image::new(
            mgr.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R32G32B32A32_SFLOAT,
                extent: [1024, 1024, 1],
                usage: ImageUsage::STORAGE | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let pipeline = {
            let cs = cs::load(mgr.context.device().clone()).unwrap().entry_point("main").unwrap();
            let stage = PipelineShaderStageCreateInfo::new(cs);
            let layout = PipelineLayout::new(
                mgr.context.device().clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(mgr.context.device().clone())
                    .unwrap(),
            ).unwrap();

            ComputePipeline::new(
                mgr.context.device().clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout)
            ).unwrap()
        };

        let grid_view = ImageView::new_default(grid_u.clone()).unwrap();
        let grid_sampler = Sampler::new(
            mgr.context.device().clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            }
        ).unwrap();

        Simulator {
            grid_u,
            grid_view,
            grid_sampler,

            pipeline,
            width: 1024,
            height: 1024,
        }
    }

    pub fn resize(&mut self, mgr: &VulkanManager, width: u32, height: u32) {
        self.grid_u = Image::new(
            mgr.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R32G32B32A32_SFLOAT,
                extent: [width as u32, height as u32, 1],
                usage: ImageUsage::STORAGE | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();
        self.grid_view = ImageView::new_default(self.grid_u.clone()).unwrap();
        self.grid_sampler = Sampler::new(
            mgr.context.device().clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            }
        ).unwrap();

        self.width = width;
        self.height = height;
    }

    pub fn compute(&self, mgr: &VulkanManager, ui_state: &UIState) {
        let layout = self.pipeline.layout().set_layouts().get(0).unwrap();
        let set = DescriptorSet::new(
            mgr.descriptor_set_allocator.clone(),
            layout.clone(),
            [WriteDescriptorSet::image_view(0, self.grid_view.clone())], // 0 is the binding
            [],
        )
        .unwrap(); 

        let mut builder = AutoCommandBufferBuilder::primary(
            mgr.command_buffer_allocator.clone(),
            mgr.context.compute_queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let push_constants = cs::PushConstantData {
            brush_x: ui_state.brush_x as i32,
            brush_y: ui_state.brush_y as i32,
            brush_enabled: ui_state.brush_enabled as i32,
            brush_radius: ui_state.brush_radius as i32,
        };

        unsafe {
            builder
                .bind_pipeline_compute(self.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    self.pipeline.layout().clone(),
                    0,
                    set,
                )
                .unwrap()
                .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                .unwrap()
                .dispatch([self.width, self.height, 1])
                .unwrap();
        }

        let command_buffer = builder.build().unwrap();

        let future = sync::now(mgr.context.device().clone())
            .then_execute(mgr.context.compute_queue().clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();
    }
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute.glsl"
    }
}