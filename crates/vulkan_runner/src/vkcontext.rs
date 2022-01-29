/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::sync::Arc;

#[cfg(feature = "validation")]

use marp::miscellaneous::{InstanceLayer, Debugging};

use marp::{device::{Device, PhysicalDevice, DeviceBuilder, Queue, QueueFamily}, swapchain::{Swapchain, surface::Surface}, ash::vk::{Extent2D, PhysicalDeviceVulkan12Features, SurfaceFormatKHR, PresentModeKHR}, miscellaneous::{AppInfo, Version, InstanceExtensions, DeviceExtension}, instance::Instance, image::ImageUsage};
use marp_surface_winit::{winit::{window::Window, event_loop::EventLoop}, WinitSurface};

#[cfg(all(unix, not(target_os = "android")))]
///Checks if the event loop is on wayland if on a unix system.
pub fn is_wayland<E>(events_loop: &EventLoop<E>) -> bool {
    use marp_surface_winit::winit::platform::unix::EventLoopWindowTargetExtUnix;

    events_loop.is_wayland()
}

#[cfg(windows)]
///Always returns false since windows has no wayland implementation.
pub fn is_wayland<E>(events_loop: &winit::event_loop::EventLoop<E>) -> bool {
    false
}

fn select_format(formats: Vec<SurfaceFormatKHR>) -> SurfaceFormatKHR {
    //This "Format and search for any unormed" filter function is ugly. In a perfect world we'd decide the "best" format and
    //handle colorspace transform in a correct post progress step. As specially HDR output could profit a lot here.
    let mut filtered: Vec<_> = formats
        .into_iter()
        .filter_map(|f| {
            if format!("{:?}", f.format).contains("UNORM") {
                Some(f)
            } else {
                None
            }
        })
        .collect();
    assert!(
        filtered.len() > 0,
        "Could not find a linear swapchain format, currently assuming there is one."
    ); //TODO Fix

    #[cfg(feature = "logging")]
    log::info!(
        "Found usable {} formats, using {:?}",
        filtered.len(),
        &filtered[0]
    );
    filtered.remove(0)
}

///Tries to select a physical device. This searches primarily for a device with present support.
/// if several with present support are found the one with
///
/// Returns the queue_index that can be used to present on `surface`, and the physical device.
fn select_physical_device(devices: Vec<Arc<PhysicalDevice>>, surface: &Arc<dyn Surface + Send + Sync>) -> Arc<PhysicalDevice>{
    //The current candidate properties in order of their weight.
    // (device_index, Some(present_queue_index), memory_size)
    let mut candidate_properties = (0, None, 0);
    if devices.len() == 0{
        panic!("Vulkan is implemented, but no physical device was found!");
    }

    for (idx, d) in devices.iter().enumerate(){
        //Check if that queue can present, if so it is a candidate, therefore check memory size as well and, if better set as new candidate.
        //FIXME: This selection filter is kind of shitty... might create a better one.
        if let Ok(present_queue_idx) = d.find_present_queue_family(surface){
            let mem_size = d.get_device_properties().limits.max_memory_allocation_count;

            if candidate_properties.1.is_none() || (candidate_properties.1.is_some() && candidate_properties.2 < mem_size){
                candidate_properties = (idx, Some(present_queue_idx), mem_size);
            }
        }
    }

    if candidate_properties.1.is_none(){
        panic!("Could not find device we can present on!");
    }

    //If we came till here a correct device is found, therefore return it
    devices[candidate_properties.0].clone()
}


///selects all queues of a physical device that should be created
fn select_queues(physical_device: &Arc<PhysicalDevice>) -> Vec<(QueueFamily, f32)>{
    //Check all available queues and rank them based on their importants
    physical_device
        .get_queue_families()
        .iter()
        .filter_map(|q| {
            match (
                q.get_queue_type().graphics,
                q.get_queue_type().compute,
                q.get_queue_type().transfer,
            ) {
                (true, true, true) => Some((*q, 1.0 as f32)), //allrounder queue
                _ => None
            }
        })
        .collect::<Vec<_>>()
}



///Holds all data needed for general marp interaction.
pub struct MarpContext{
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain>,

    #[cfg(feature = "watch_shaders")]
    shader_watcher: ShaderWatcher,
    
    
    ///The last known extent of the swapchain.
    current_extent: Extent2D,
}


impl MarpContext{
    ///Creates the vulkan context.
    ///
    /// # Panics
    /// If no vulkan driver is present, or features
    /// that are needed to run the renderer.
    //TODO: Add additional information like application name or debugging level?
    pub fn new<E>(
        window: &Window,
        event_loop: &EventLoop<E>,
    ) -> Self{

        let app_info = AppInfo::new(
            "NakoApplicaton".to_string(),
            Version::new(0, 1, 0),
            "Nako".to_string(),
            Version::new(0, 1, 0),
            Version::new(1, 2, 0),
        );
        
        let mut extensions = InstanceExtensions::presentable();

        if !is_wayland(event_loop) {
            #[cfg(feature = "logging")]
            log::info!("Not on wayland!");
            extensions.wayland_surface = false;
        } else {
            #[cfg(feature = "logging")]
            log::info!("On wayland");
        }

        //Load debug layer and output when in validation mode
        #[cfg(all(feature = "validation"))]
        let layer = Some(InstanceLayer::debug_layers());

        #[cfg(not(feature = "validation"))]
        let layer = None;

        #[cfg(feature = "validation")]
        let debug = Some(Debugging {
            should_debug: true,
            on_warning: true,
            on_error: true,
            ..Default::default()
        });

        #[cfg(not(feature = "validation"))]
        let debug = None; //Overwrite for debug builds with all

        let instance = Instance::new(Some(app_info), Some(extensions), layer, debug).unwrap();
        let surface: Arc<dyn Surface + Send + Sync> =
            WinitSurface::new(instance.clone(), window, event_loop).unwrap();
        //Now search for any graphics capable device


        let physical_device = select_physical_device(
            PhysicalDevice::find_physical_device(instance.clone()).unwrap(),
            &surface
        );

        
        #[cfg(feature = "logging")]
        log::info!(
            "Using physical device {:?}",
            //SAFETY: should be save since the chars should be ascii which is a subset of utf8. However we only need the transmute into u8
            // str parsing is safe again
            unsafe{
                std::ffi::CStr::from_ptr(
                    physical_device.get_device_properties().device_name.as_slice() as *const [i8] as *const i8
                )
            }
        );




        //Filter out a graphics queue that also supports compute.
        let queues = select_queues(&physical_device);

        
        #[cfg(feature = "logging")]
        log::info!("Selected queues:\n{:#?}", queues);
        
        //TODO maybe enable debug marker here if debug flag is set
        let features = *physical_device.get_features();
        let vulkan_memory_model = PhysicalDeviceVulkan12Features::builder()
            .shader_int8(true)
            .vulkan_memory_model(true);

        let (device, mut queues) = DeviceBuilder::new(instance, physical_device, queues)
            .with_extension(DeviceExtension::new("VK_KHR_swapchain".to_string(), 1))
            .with_extension(DeviceExtension::new(
                "VK_KHR_vulkan_memory_model".to_string(),
                3,
            ))
            .with_device_features(features)
            .with_additional_feature(vulkan_memory_model)
            .build()
            .expect("Could not create device and queues, note that you need a fairly new graphics card to run nako!");

        assert!(queues.len() > 0, "Could not create graphics capable queue!");
        let queue = queues.remove(0);
        
        //Since we got a vulkan instance running now, setup a swapchain
        let swapchain_formats = surface
            .get_supported_formats(device.get_physical_device())
            .unwrap();
        let format = select_format(swapchain_formats);
        #[cfg(feature = "logging")]
        log::info!("Select surface format: {:?}", format);

        //Since we now have our formats. Create the swapchain
        let swapchain_extent = Extent2D::builder()
            .width(window.inner_size().width as u32)
            .height(window.inner_size().height as u32)
            .build();

        let swapchain = Swapchain::new(
            device.clone(),
            surface,
            swapchain_extent,
            Some(format),
            None,
            Some(PresentModeKHR::IMMEDIATE),
            Some(ImageUsage {
                color_attachment: true,
                transfer_dst: true,
                ..Default::default()
            }),
        )
            .unwrap();

        let sc_transition_fence = swapchain.images_to_present_layout(queue.clone());
        //Wait for frame transition before continuing
        sc_transition_fence.wait(u64::MAX).unwrap();

        MarpContext{
            device,
            queue,
            swapchain,
            current_extent: swapchain_extent
        }
    }

    ///
    pub fn frame_extent(&self) -> Extent2D{
        self.current_extent
    }
    
    ///Resizes the inner swapchain to `new extent`
    pub fn resize_frame(&mut self, new_extent: Extent2D){
        //Sometime we have to resize. We do that by reading the new extent information
        //And setting up all subsystems based on it. While there are more effective ways to do that,
        //its nice and easy for now.

        self.device.wait_idle().unwrap();

        #[cfg(feature = "logging")]
        log::info!(
            "Old ext was {:?}, new is {:?}",
            self.current_extent,
            new_extent
        );
        //TODO handle dpi scaling and this stuff?

        //Recreate swapchain and transform images into present layout.
        self.swapchain.recreate(new_extent);
        let submit_fence = self
            .swapchain
            .images_to_present_layout(self.queue.clone());
        submit_fence.wait(u64::MAX).unwrap();

        //Update stack size to new frame size
        self.current_extent = new_extent;
    }
}
