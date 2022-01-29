/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use std::sync::Arc;

use marp::{
    ash::vk::{
        AccessFlags, Filter, ImageAspectFlags, ImageBlit, ImageLayout, ImageSubresourceLayers,
        Offset3D, PipelineStageFlags,
    },
    command_buffer::{
        AccessMaskEvent, CommandBuffer, ImageLayoutTransitionEvent, ImageMemoryBarrierBuilder,
        PipelineBarrier,
    },
    image::AbstractImage,
};

///Simple copy pass copying src to dst
pub(crate) struct ImgToSwapchain;

impl ImgToSwapchain {
    fn pre_transition(
        command_buffer: Arc<CommandBuffer>,
        src_img: Arc<dyn AbstractImage + Send + Sync>,
        dst: Arc<dyn AbstractImage + Send + Sync>,
    ) -> Arc<CommandBuffer> {
        //Assuming that the src image is always in "General" layout before being passed here.

        command_buffer
            .pipeline_barrier(
                PipelineBarrier::<2>::new(
                    PipelineStageFlags::COMPUTE_SHADER, //Wait for compute shader
                    PipelineStageFlags::TRANSFER,       //Wanna do a transfer
                )
                .with_image_barrier(
                    ImageMemoryBarrierBuilder::new(
                        src_img,
                        ImageLayoutTransitionEvent::Transition {
                            from: ImageLayout::GENERAL,
                            to: ImageLayout::TRANSFER_SRC_OPTIMAL,
                        },
                    )
                    .with_access_flags_event(AccessMaskEvent::Transition {
                        from: AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
                        to: AccessFlags::TRANSFER_READ,
                    }),
                )
                .with_image_barrier(ImageMemoryBarrierBuilder::new(
                    dst,
                    ImageLayoutTransitionEvent::Transition {
                        from: ImageLayout::PRESENT_SRC_KHR,
                        to: ImageLayout::TRANSFER_DST_OPTIMAL,
                    },
                ))
                .build(),
            )
            .unwrap();
        command_buffer
    }

    pub(crate) fn record(
        mut command_buffer: Arc<CommandBuffer>,
        src_img: Arc<dyn AbstractImage + Send + Sync>,
        dst_img: Arc<dyn AbstractImage + Send + Sync>,
    ) -> Arc<CommandBuffer> {
        command_buffer = Self::pre_transition(command_buffer, src_img.clone(), dst_img.clone());

        //We use the blit command, since the image formats (rgb -> bgr) or something else might not add up.
        let srcext = src_img.extent();
        let dstext = dst_img.extent();

        command_buffer
            .cmd_blit_image(
                src_img.clone(),
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst_img.clone(),
                ImageLayout::TRANSFER_DST_OPTIMAL,
                vec![ImageBlit::builder()
                    .src_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .src_offsets([
                        Offset3D { x: 0, y: 0, z: 0 },
                        Offset3D {
                            x: srcext.width as i32,
                            y: srcext.height as i32,
                            z: 1,
                        },
                    ])
                    .dst_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .dst_offsets([
                        Offset3D { x: 0, y: 0, z: 0 },
                        Offset3D {
                            x: dstext.width as i32,
                            y: dstext.height as i32,
                            z: 1,
                        },
                    ])
                    .build()],
                Filter::LINEAR,
            )
            .expect("Failed to blit src to swapchain image");

        command_buffer = Self::post_transition(command_buffer, src_img.clone(), dst_img.clone());

        command_buffer
    }

    fn post_transition(
        command_buffer: Arc<CommandBuffer>,
        src_img: Arc<dyn AbstractImage + Send + Sync>,
        dst: Arc<dyn AbstractImage + Send + Sync>,
    ) -> Arc<CommandBuffer> {
        command_buffer
            .pipeline_barrier(
                PipelineBarrier::<2>::new(
                    PipelineStageFlags::TRANSFER,     //Wait for compute shader
                    PipelineStageFlags::ALL_COMMANDS, //Wanna do a transfer
                )
                .with_image_barrier(
                    ImageMemoryBarrierBuilder::new(
                        src_img,
                        ImageLayoutTransitionEvent::Transition {
                            from: ImageLayout::TRANSFER_SRC_OPTIMAL,
                            to: ImageLayout::GENERAL,
                        },
                    )
                    .with_access_flags_event(AccessMaskEvent::Transition {
                        from: AccessFlags::TRANSFER_READ,
                        to: AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
                    }),
                )
                .with_image_barrier(ImageMemoryBarrierBuilder::new(
                    dst,
                    ImageLayoutTransitionEvent::Transition {
                        from: ImageLayout::TRANSFER_DST_OPTIMAL,
                        to: ImageLayout::PRESENT_SRC_KHR,
                    },
                ))
                .build(),
            )
            .unwrap();
        command_buffer
    }
}
