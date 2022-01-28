use algae_jit::AlgaeJit;




#[test]
fn jit_creation(){
    assert!(AlgaeJit::new("./").is_err(), "Creating a module from directory should return error");
}
