
#![deny(warnings)]

use algae_jit::AlgaeJit;
use frame_builder::FrameBuilder;
use marp_surface_winit::winit::{event_loop::{EventLoop, ControlFlow}, window::Window, event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}};

use crate::vkcontext::MarpContext;

///Set up of vulkan device, queues etc.
mod vkcontext;

///simple render pass that starts a compute shader and writes the output
///to an image
mod pass_renderer;

///Blits an image to a swapchain image
mod pass_image_to_swapchain;

///Frame builder. Hosts swapchain image handling, subresource generation and recording of the command buffer.
mod frame_builder;

fn main(){

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut ctx = MarpContext::new(&window, &event_loop);
    
    let compiler = AlgaeJit::new("resources/test_shader.spv").unwrap();

    let mut fb = FrameBuilder::new(&ctx, compiler);


    event_loop.run(move |event, _target, control_flow|{

        *control_flow = ControlFlow::Poll;

        match event{
            Event::WindowEvent{
                event: WindowEvent::KeyboardInput{
                    input: KeyboardInput{
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                    ..
                },
                ..
            } => {
                match (state, keycode){
                    (ElementState::Pressed, VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            Event::WindowEvent{event: WindowEvent::CloseRequested, ..} => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                fb.render(&mut ctx, &window);
            },
            _ => {}
        }
        
    });
    
}
