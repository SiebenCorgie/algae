/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr, asm_experimental_arch),
    register_attr(spirv),
    no_std
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

use spirv_std;
use spirv_std::glam::{UVec3, Vec2, Vec3Swizzles, Vec4};
use spirv_std::Image;

//Note this is needed to compile on cpu
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[derive(Clone)]
#[repr(C)]
pub struct PushConst {
    offset: [f32; 2],
    pad0: [f32; 2],
}

algae_gpu::algae_inject!(|coord: Vec2, offset: Vec2| -> f32 {
    let a = coord.value + offset.value;
    a.dot(a)
});

#[spirv(compute(threads(8, 8, 1)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(push_constant)] push: &PushConst,
    #[spirv(descriptor_set = 0, binding = 0)] target_image: &Image!(2D, format=rgba32f, sampled=false),
) {
    let color = Vec4::new(
        (id.x as f32 / 100.0) % 1.0,
        (id.y as f32 / 100.0) % 1.0,
        (id.z as f32 / 100.0) % 1.0,
        1.0,
    );

    let coord = id.as_vec3().xy();
    /*
    let coord = VariableSignature{
        id: 0,
        value: id.as_vec3().xy()
    };

    let pushsig = VariableSignature{
        id: 1,
        value: push.clone()
    };
     */

    let color = if algae_inject(coord, Vec2::from(push.offset)) > 0.0 {
        Vec4::ZERO
    } else {
        color
    };

    unsafe {
        target_image.write(id.xy(), color);
    }
}

//Gonna emitted some stuff
