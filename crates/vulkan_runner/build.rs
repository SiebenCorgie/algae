use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() {
    SpirvBuilder::new("../test_shader", "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::Full)
        .build()
        .expect("Failed to build test shader");

    //copy spirv file into resource folder
    std::fs::copy(
        "../../target/spirv-builder/spirv-unknown-vulkan1.1/release/deps/test_shader.spv.dir/module",
        "../../resources/test_shader.spv"
    ).unwrap();
}
