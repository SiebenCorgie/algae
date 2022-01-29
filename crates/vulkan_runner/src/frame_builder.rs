/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::sync::Arc;

use algae_jit::AlgaeJit;
use marp::{command_buffer::{CommandBuffer, CommandBufferPool, CommandPool}, sync::{Semaphore, QueueFence}, image::{SwapchainImage, AbstractImage, Image}, ash::vk::{self, Extent2D, PipelineStageFlags}, device::SubmitInfo};
use marp_surface_winit::winit::window::Window;

use crate::{vkcontext::MarpContext, pass_renderer::ImagePass, pass_image_to_swapchain::ImgToSwapchain};

pub const LOCAL_SIZE: [u32; 3] = [8, 8, 1];

///Calculates the correct dispatch size for an image, assuming that the kernel uses `LOCAL_SIZE` as local dispatch size.
pub fn dispatch_size(image: &Arc<Image>) -> [u32; 3] {
    [
        (image.extent().width as f32 / LOCAL_SIZE[0] as f32).ceil() as u32,
        (image.extent().height as f32 / LOCAL_SIZE[1] as f32).ceil() as u32,
        1,
    ]
}


pub struct PerFrameData{
    command_buffer: Arc<CommandBuffer>,

    copy_finished: Arc<Semaphore>,
    in_flight: Option<QueueFence>,

    swapchain_image: Arc<SwapchainImage>,
}

pub struct FrameBuilder{
    frames: Vec<PerFrameData>,

    image_pass: ImagePass,
}

impl FrameBuilder{
    pub fn new(
        ctx: &MarpContext,
        jit: AlgaeJit,
    ) -> Self{
        let swapchain_images = ctx.swapchain.get_images();
        let num_scimgs = swapchain_images.len();
        let extent = swapchain_images[0].extent();
        let command_pool = CommandBufferPool::new(
            ctx.device.clone(),
            ctx.queue.clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )
            .expect("Failed to create command pool");

        let command_buffers = command_pool
            .alloc(num_scimgs, false)
            .expect("Failed to allocate command buffers");

        let frame_infos: Vec<_> = command_buffers
            .into_iter()
            .enumerate()
            .map(|(idx, cb)| {
                PerFrameData {
                    command_buffer: cb,
                    copy_finished: Semaphore::new(ctx.device.clone()).unwrap(),
                    in_flight: None,
                    swapchain_image: swapchain_images[idx].clone(),
                }
            })
            .collect();

        let image_pass = ImagePass::new(
            jit,
            ctx.device.clone(),
            extent,
            num_scimgs
        );
        
        Self {
            image_pass,
            frames: frame_infos,
        }
    }

    fn check_resize(&mut self, ctx: &mut MarpContext, window: &Window){
        let extent = if let Ok(caps) = ctx.swapchain.get_suface_capabilities() {
            match caps.current_extent {
                Extent2D {
                    width: 0xFFFFFFFF,
                    height: 0xFFFFFFFF,
                } => {
                    //Choose based on the window.
                    //Todo make robust agains hidpi scaling
                    Extent2D {
                        width: window.inner_size().width,
                        height: window.inner_size().height,
                    }
                }
                Extent2D { width, height } => Extent2D { width, height },
            }
        } else {
            //Fallback to window provided size
            Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            }
        };

        if ctx.frame_extent() != extent {
            #[cfg(feature = "logging")]
            log::info!("Detected window resize!");
            
            //notify marp context
            ctx.resize_frame(extent);
            //now rebuild self                
            let mut new_builder = FrameBuilder::new(ctx, self.image_pass.shader_loader.clone());
            core::mem::swap(self, &mut new_builder);

        }
    }

    fn get_copy_complete_semaphore(&self, slot: usize) -> Arc<Semaphore> {
        self.frames[slot].copy_finished.clone()
    }
    
    ///Renders a new frame and outputs it when ready
    pub fn render(&mut self, ctx: &mut MarpContext, window: &Window){
        println!("Check window");
        self.check_resize(ctx, window);

        println!("Get next window");
        //Since the image must be "Ok" now, start building the frame now.
        let sem_present_finshed = Semaphore::new(ctx.device.clone()).unwrap();
        let submit_image_index = ctx
            .swapchain
            .acquire_next_image(u64::MAX, sem_present_finshed.clone())
            .unwrap();

        assert!(submit_image_index < ctx.swapchain.image_count());
        let slot = submit_image_index as usize;

        println!("Wait for window");
        //Wait for last copy
        if let Some(inflight) = self.frames[slot].in_flight.take() {
            inflight.wait(u64::MAX).unwrap();
        }


        println!("Record!");
        //Begin copy command buffer
        //Reset command buffer
        let mut command_buffer = self.frames[slot].command_buffer.clone();
        command_buffer.reset().unwrap();
        command_buffer
            .begin_recording(true, false, false, None)
            .unwrap();

        command_buffer = self.image_pass.pre(command_buffer, slot);
        command_buffer = self.image_pass.record(command_buffer, slot);
        //Now copy image to swapchain
        
        command_buffer = ImgToSwapchain::record(
            command_buffer,
            self.image_pass.get_final_image(slot).unwrap(),
            self.frames[slot].swapchain_image.clone(),
        );

        //Finish command buffer and submit
        command_buffer.end_recording().unwrap();

        //Execute the copy tasks and the final "copy to swapchain" task on the present queue
        let new_inflight = ctx.queue
            .queue_submit(vec![SubmitInfo::new(
                vec![(sem_present_finshed, PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)], 
                vec![command_buffer],
                vec![self.frames[slot].copy_finished.clone()], //Signal execution semaphore
            )])
            .unwrap();

        //Setup new inflight fence
        self.frames[slot].in_flight = Some(new_inflight);

        
        //Tell swapchain that it can present this frame when we finished rendering
        if let Err(_e) = ctx.swapchain.queue_present(
            ctx.queue.clone(),
            vec![self
                 .get_copy_complete_semaphore(submit_image_index as usize)],
            submit_image_index,
        ) {
            #[cfg(feature = "logging")]
            log::warn!("Dropped frame");
        }
    }
}

