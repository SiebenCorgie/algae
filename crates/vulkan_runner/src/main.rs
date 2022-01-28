use algae_jit::AlgaeJit;


fn main(){
    println!("Loading shader!");
    
    let compiler = AlgaeJit::new("resources/test_shader.spv").unwrap();
    
}
