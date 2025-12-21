use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
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

use crate::app::App;

pub struct Simulator {
    grid_u: Arc<Image>,
    pub grid_view: Arc<ImageView>,
    pub grid_sampler: Arc<Sampler>,

    pipeline: Arc<ComputePipeline>,
}

impl Simulator {
    pub fn new(app: &App) -> Self {
        let grid_u = Image::new(
            app.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
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
            let cs = cs::load(app.device.clone()).unwrap().entry_point("main").unwrap();
            let stage = PipelineShaderStageCreateInfo::new(cs);
            let layout = PipelineLayout::new(
                app.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(app.device.clone())
                    .unwrap(),
            ).unwrap();

            ComputePipeline::new(
                app.device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout)
            ).unwrap()
        };

        let grid_view = ImageView::new_default(grid_u.clone()).unwrap();
        let grid_sampler = Sampler::new(
            app.device.clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            }
        ).unwrap();

        Simulator {
            grid_u,
            grid_view,
            grid_sampler,

            pipeline,
        }
    }

    pub fn compute(&self, app: &App) {
        let layout = self.pipeline.layout().set_layouts().get(0).unwrap();
        let set = PersistentDescriptorSet::new(
            &app.descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::image_view(0, self.grid_view.clone())], // 0 is the binding
            [],
        )
        .unwrap(); 

        let mut builder = AutoCommandBufferBuilder::primary(
            &app.command_buffer_allocator,
            app.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
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
            .dispatch([1024, 1024, 1])
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let future = sync::now(app.device.clone())
            .then_execute(app.queue.clone(), command_buffer)
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