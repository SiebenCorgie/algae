/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use std::sync::Arc;

use algae_jit::AlgaeJit;
use marp::{
    ash::vk::{
        AccessFlags, DescriptorPoolSize, DescriptorType, Extent2D, Format, ImageLayout,
        PipelineBindPoint, PipelineStageFlags, ShaderStageFlags,
    },
    buffer::SharingMode,
    command_buffer::{
        AccessMaskEvent, CommandBuffer, ImageLayoutTransitionEvent, ImageMemoryBarrierBuilder,
        PipelineBarrier,
    },
    descriptor::{DescResource, DescriptorPool, DescriptorSet, PushConstant, StdDescriptorPool},
    device::Device,
    image::{Image, ImageInfo, ImageType, ImageUsage, MipLevel},
    memory::MemoryUsage,
    pipeline::{ComputePipeline, PipelineLayout},
    shader::{Stage, ShaderModule, AbstractShaderModule},
};

use crate::frame_builder::dispatch_size;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct PushConst{
    #[allow(dead_code)]
    color: [f32; 4]
}

pub(crate) struct PassData {
    pub(crate) target_image: Arc<Image>,
    descriptor_set: Arc<DescriptorSet>,
    is_transitioned: bool,
}

pub(crate) struct ImagePass {
    pub(crate) data: Vec<PassData>,
    pipeline: Arc<ComputePipeline>,
    
    pub(crate) shader_loader: AlgaeJit,
    camera: PushConstant<PushConst>,
}

impl ImagePass {
    pub fn new(
        mut jit: AlgaeJit,
        device: Arc<Device>,
        extent: Extent2D,
        num_slots: usize,
    ) -> Self {
        let descriptor_pool = StdDescriptorPool::new(
            device.clone(),
            vec![
                DescriptorPoolSize::builder()
                    .ty(DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(num_slots as u32 * 1) //output
                    .build(),
            ]
            .as_slice(),
            num_slots as u32,
        )
        .unwrap();

        let mut data = Vec::with_capacity(num_slots);
        for _i in 0..num_slots {
            let target_image = Image::new(
                device.clone(),
                ImageInfo::new(
                    ImageType::Image2D {
                        width: extent.width,
                        height: extent.height,
                        samples: 1,
                    },
                    Format::R32G32B32A32_SFLOAT,
                    None,
                    Some(MipLevel::Specific(1)),
                    ImageUsage {
                        transfer_src: true,
                        storage: true,
                        color_aspect: true,
                        ..Default::default()
                    },
                    MemoryUsage::GpuOnly,
                    None,
                ),
                SharingMode::Exclusive,
            )
            .unwrap();

            //Allocate the descriptor set
            let mut descriptor_set = descriptor_pool.next();
            descriptor_set
                .add(DescResource::new_image(
                    0,
                    vec![(target_image.clone(), None, ImageLayout::GENERAL)],
                    DescriptorType::STORAGE_IMAGE,
                ))
                .unwrap();

            let descriptor_set = descriptor_set.build().unwrap();

            data.push(PassData {
                target_image,
                descriptor_set,
                is_transitioned: false,
            });
        }

        //Create initial push constant
        let camera_const = PushConstant::new(
            PushConst {
                color: [1.0; 4]
            },
            ShaderStageFlags::COMPUTE,
        );

        //Setup the compute pipeline.
        let pipe_layout = PipelineLayout::new(
            device.clone(),
            vec![*data[0].descriptor_set.layout()],
            vec![*camera_const.range()],
        )
        .unwrap();

        let shader = jit.get_module();
        let shader_module = ShaderModule::new_from_code(
            device.clone(),
            shader.to_vec()
        ).unwrap();

        let shader = shader_module.to_stage(Stage::Compute, "main");

        let pipeline = ComputePipeline::new(device.clone(), shader, pipe_layout).unwrap();

        ImagePass {
            data,
            pipeline,
            shader_loader: jit,
            camera: camera_const,
        }
    }

    pub(crate) fn get_final_image(&self, slot: usize) -> Option<Arc<Image>> {
        if let Some(data) = self.data.get(slot) {
            Some(data.target_image.clone())
        } else {
            None
        }
    }

    pub(crate) fn pre(
        &mut self,
        command_buffer: Arc<CommandBuffer>,
        slot_index: usize,
    ) -> Arc<CommandBuffer> {
        //Transition every resource that isn't yet in the correct layout
        if self.data[slot_index].is_transitioned {
            return command_buffer;
        }

        //We want our images to be in the general layout, so we can write to them in the primary pass
        command_buffer
            .pipeline_barrier(
                PipelineBarrier::<1>::new(
                    PipelineStageFlags::ALL_COMMANDS,   //Wait for compute shader
                    PipelineStageFlags::COMPUTE_SHADER, //Wanna do a transfer
                )
                .with_image_barrier(
                    ImageMemoryBarrierBuilder::new(
                        self.data[slot_index].target_image.clone(),
                        ImageLayoutTransitionEvent::Initialise(ImageLayout::GENERAL),
                    )
                    .with_access_flags_event(AccessMaskEvent::Acquire(AccessFlags::SHADER_WRITE)),
                )
                .build(),
            )
            .unwrap();

        //Mark this slot as transitioned
        self.data[slot_index].is_transitioned = true;
        command_buffer
    }

    pub(crate) fn record(
        &mut self,
        command_buffer: Arc<CommandBuffer>,
        slot_index: usize,
    ) -> Arc<CommandBuffer> {
        //Bind descriptorset
        command_buffer
            .cmd_bind_descriptor_sets(
                PipelineBindPoint::COMPUTE,
                self.pipeline.layout(),
                0,
                vec![self.data[slot_index].descriptor_set.clone()],
                vec![],
            )
            .expect("Failed to bind Primary pass Descriptorset");

        command_buffer
            .cmd_bind_pipeline(PipelineBindPoint::COMPUTE, self.pipeline.clone())
            .expect("Failed to bind primary pipeline");
        //upload current camera
        command_buffer
            .cmd_push_constants(self.pipeline.layout(), &self.camera)
            .unwrap();
        command_buffer
            .cmd_dispatch(dispatch_size(&self.data[slot_index].target_image))
            .expect("Failed to schedule primary pass");

        command_buffer
    }
}

