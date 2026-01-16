use vulkano::command_buffer::ClearColorImageInfo;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::format::*;
use vulkano::image::sampler::{Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::*;
use vulkano::memory::allocator::*;
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};

use std::sync::Arc;

use crate::app::VulkanManager;
use crate::ui_state::UIState;

pub struct Simulator {
    grid_u: Arc<Image>,
    pub grid_view: Arc<ImageView>,
    pub grid_sampler: Arc<Sampler>,

    pipeline: Arc<ComputePipeline>,

    pub width: u32,
    pub height: u32,
}

impl Simulator {
    pub fn new(mgr: &VulkanManager) -> Self {
        let width = 1;
        let height = 1;

        let pipeline = {
            let cs = cs::load(mgr.context.device().clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let stage = PipelineShaderStageCreateInfo::new(cs);
            let layout = PipelineLayout::new(
                mgr.context.device().clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(mgr.context.device().clone())
                    .unwrap(),
            )
            .unwrap();

            ComputePipeline::new(
                mgr.context.device().clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .unwrap()
        };

        let grid_u = Self::get_grid_image(mgr, width, height);
        let grid_view = ImageView::new_default(grid_u.clone()).unwrap();
        let grid_sampler = Sampler::new(
            mgr.context.device().clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .unwrap();

        Simulator {
            grid_u,
            grid_view,
            grid_sampler,

            pipeline,
            width,
            height,
        }
    }

    pub fn get_grid_image(mgr: &VulkanManager, width: u32, height: u32) -> Arc<Image> {
        Image::new(
            mgr.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R32G32B32A32_SFLOAT,
                extent: [width, height, 1],
                usage: ImageUsage::STORAGE | ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap()
    }

    pub fn resize(&mut self, mgr: &VulkanManager, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.grid_u = Self::get_grid_image(mgr, width, height);
        self.grid_view = ImageView::new_default(self.grid_u.clone()).unwrap();
        self.zero_grid(mgr);
    }

    pub fn zero_grid(&self, mgr: &VulkanManager) {
        let mut builder = mgr.get_compute_cmdbuffer_builder();

        builder
            .clear_color_image(ClearColorImageInfo {
                clear_value: ClearColorValue::Float([0.0, 0.0, 0.0, 0.0]),
                ..ClearColorImageInfo::image(self.grid_u.clone())
            })
            .unwrap();

        mgr.execute_compute_cmdbuffer_from_builder(builder);
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

        let mut stage0_builder = mgr.get_compute_cmdbuffer_builder();
        let mut stage1_builder = mgr.get_compute_cmdbuffer_builder();

        let mut push_constants = cs::PushConstantData {
            time_step: ui_state.time_step,
            speed: ui_state.speed,
            theta: ui_state.theta,
            brush_x: ui_state.brush_x,
            brush_y: ui_state.brush_y,
            brush_enabled: ui_state.brush_enabled,
            brush_radius: ui_state.brush_radius,
            brush_value: ui_state.brush_value,
            brush_layer: ui_state.brush_layer as i32,
            boundary_condition: ui_state.boundary_condition as i32,
            stage: 0,
        };

        unsafe {
            stage0_builder
                .bind_pipeline_compute(self.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    self.pipeline.layout().clone(),
                    0,
                    set.clone(),
                )
                .unwrap()
                .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                .unwrap()
                .dispatch([self.width, self.height, 1])
                .unwrap();
        }

        push_constants.stage = 1;

        unsafe {
            stage1_builder
                .bind_pipeline_compute(self.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    self.pipeline.layout().clone(),
                    0,
                    set.clone(),
                )
                .unwrap()
                .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                .unwrap()
                .dispatch([self.width, self.height, 1])
                .unwrap();
        }

        mgr.execute_compute_cmdbuffer_from_builder(stage0_builder);
        mgr.execute_compute_cmdbuffer_from_builder(stage1_builder);
    }
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute.glsl"
    }
}
