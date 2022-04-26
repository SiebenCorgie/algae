//#![deny(warnings)]

use algae::{
    glam::Vec2,
    operations::{
        Abs, Addition, Constant, Length, MapInput, Max, Min, Subtraction,
        Variable, VecSelectElement, OrderedOperations, AccessResult,
    },
};
use algae_jit::AlgaeJit;
use frame_builder::FrameBuilder;
use marp_surface_winit::winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

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

fn main() {
    #[cfg(feature = "logging")]
    simple_logger::SimpleLogger::new().init().unwrap();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut ctx = MarpContext::new(&window, &event_loop);

    let mut compiler = AlgaeJit::new("resources/test_shader.spv").unwrap();
    /*
    let mut circle_function = Subtraction {
        minuent: Box::new(Length {
            //inner: Box::new(Variable::new("coord", Vec2::new(0.0, 0.0)))

            inner: Box::new(Addition{
                a: Box::new(Variable::new("coord", Vec2::new(0.0, 0.0))),
                b: Box::new(Variable::new("offset", Vec2::new(0.0, 0.0)))
            }),

        }),
        subtrahend: Box::new(Constant { value: 100.0 }),
    };
     */

    /*
    float sdBox( in vec2 p, in vec2 b )
    {
        vec2 d = abs(p)-b;
        return length(max(d,0.0)) + min(max(d.x,d.y),0.0);
    }
    */

    
    let mut bfunc: OrderedOperations<(), _> = OrderedOperations::new(
        "d", Box::new(Subtraction {
            minuent: Box::new(Abs {
                inner: Box::new(Variable::new("coord", Vec2::ZERO)),
            }), //abs(p)
            subtrahend: Box::new(Constant::new(Vec2::new(50.0, 60.0))), //extend b
        })        
    ).push(
        "result", Box::new(Addition {
            a: Box::new(Length {
                inner: Box::new(Max {
                    a: Box::new(AccessResult::<Vec2>::new("d")),
                    b: Box::new(Constant::new(Vec2::ZERO))
                }),
            }),
            b: Box::new(Min {
                a: Box::new(Max {
                    a: Box::new(VecSelectElement::<Vec2, _> {
                        element: 0,
                        inner: Box::new(AccessResult::<Vec2>::new("d")),
                    }),
                    b: Box::new(VecSelectElement::<Vec2, _> {
                        element: 1,
                        inner: Box::new(AccessResult::<Vec2>::new("d")),
                    }),
                }),
                b: Box::new(Constant::new(0.0f32)),
            }),
        })
    );
     

    let mut op: OrderedOperations<(), _> = algae_grammar::rexpr!{
        let d: Vec2 = Sub(Abs(Add(Var(coord, Vec2(0.0f32, 0.0f32)), Var(offset, Vec2(0.0f32, 0.0f32)))),  Const(Vec2(200.0f32, 50.0f32)));
        let res: f32 = Add(Length(Max(d, Const(Vec2(0.0, 0.0)))),  Min(Max(VecSelectElement(d, 0), VecSelectElement(d, 1)), Const(0.0f32)));
	
        return res;
    };
    
    /*
    let mut box_function = Link {
        //Calculate
        first: Box::new(Subtraction {
            minuent: Box::new(Abs {
                inner: Box::new(Variable::new("coord", Vec2::ZERO)),
            }), //abs(p)
            subtrahend: Box::new(Constant {
                value: Vec2::new(50.0, 60.0),
            }), //extend b
        }),
        second: Box::new(Addition {
            a: Box::new(Length {
                inner: Box::new(Max {
                    a: Box::new(ReturnInput::new()),
                    b: Box::new(MapInput::new(
                        Box::new(Constant { value: Vec2::ZERO }),
                        |_i| (), //map to nothing
                    )),
                }),
            }),
            b: Box::new(Min {
                a: Box::new(Max {
                    a: Box::new(VecSelectElement::<Vec2, _> {
                        element: 0,
                        inner: Box::new(ReturnInput::new()),
                    }),
                    b: Box::new(VecSelectElement::<Vec2, _> {
                        element: 1,
                        inner: Box::new(ReturnInput::new()),
                    }),
                }),
                b: Box::new(MapInput::new(Box::new(Constant { value: 0.0f32 }), |_i| ())),
            }),
        }),
    };
*/
    compiler.injector().inject((), &mut op);

    let mut fb = FrameBuilder::new(&ctx, compiler);

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => match (state, keycode) {
                (ElementState::Pressed, VirtualKeyCode::Escape) => {
                    *control_flow = ControlFlow::Exit
                }
                (ElementState::Pressed, VirtualKeyCode::A) => {
                    fb.on_push_const(|c| c.offset[0] -= 10.0);
                }
                (ElementState::Pressed, VirtualKeyCode::D) => {
                    fb.on_push_const(|c| c.offset[0] += 10.0);
                }
                (ElementState::Pressed, VirtualKeyCode::W) => {
                    fb.on_push_const(|c| c.offset[1] -= 10.0);
                }
                (ElementState::Pressed, VirtualKeyCode::S) => {
                    fb.on_push_const(|c| c.offset[1] += 10.0);
                }
                _ => {}
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                fb.render(&mut ctx, &window);
            }
            _ => {}
        }
    });
}
