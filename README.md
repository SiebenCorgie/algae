<div align="center">

# 🦠 Algae 

Runtime linear algebra representation for Rust and JIT compiler for function into SpirV-byte code.

</div>

## Alternatives

Algebra crates:

- [alga](https://github.com/dimforge/alga)
- [nalgebra](https://crates.io/crates/nalgebra) newer version of alga (I think)
- [glam](https://crates.io/crates/glam) 1D-3D crate, good for game/graphics development.


## Description

**Disclaimer**

This is a highly experimental crate that might die unexpectedly. 

The aim of this project is to create a user facing API that allows to create linear algebra function. They key point being "at runtime". This basically boils down to a `Function<I, O>` object representing `f: I -> O`. 

This function can either be evaluated, or be injected into a SpirV shading module. Algae has to keep track of the function's `I` and `O` parameter for correct embedding into the SpirV-module. At this point [Nako](https://gitlab.com/tendsinmende/nako) will be moved to use Algae a backend, which will allow us to inject SDF functions into rust-gpu shaders. Probably via a `algae_function!(a: I)` macro. 

In ASCII art the concept looks like this:

```
+-------------+  rust-gpu compiles  +--------------+  load into AlgaeInjector  +--------------------------------+  Build pipeline from module (vulkan/wgpu)  +-----------------------+
| shader file | ------------------> | spirv module | ------------------------> | Inject algae function at macro | -----------------------------------------> | Execute shader on GPU |
+-------------+                     +--------------+                           +--------------------------------+                                            +-----------------------+
```

This will (hopefully) allow us to execute runtime specified functions (and SDFs) on the GPU by JIT compiling the Algae function.


## Getting started

TODO

## Roadmap

- [ ] High level Algebra Interface HAI
- [ ] Shader-toy like Vulkan runner for shaders
- [ ] SpirV "hook" macros and SpirV module analyzer
- [ ] Proof of Concept injecting of standard function into SpirV module
- [ ] Set of functions for algae
- [ ] 


## Contributing

TODO

## License

The whole project is licensed under MPL v2.0, all contributions will be licensed the same. Have a look at Mozilla's [FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) to see if this fits your use-case.
