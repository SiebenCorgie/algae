use spirv_builder::{MetadataPrintout, SpirvBuilder};


const SHADER_DIR: &'static str = "../../resources/";
fn main() {
    SpirvBuilder::new("../test_shader", "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::Full)
        .spirv_metadata(spirv_builder::SpirvMetadata::Full)
        .build()
        .expect("Failed to build test shader");

    //If not there, create the resources folder
    std::fs::create_dir_all(SHADER_DIR).expect("Failed to create resources dir");
    
    //copy spirv file into resource folder
    std::fs::copy(
        "../../target/spirv-builder/spirv-unknown-vulkan1.1/release/deps/test_shader.spv.dir/module",
        "../../resources/test_shader.spv"
    ).unwrap();
}
