use algae_gpu::{const_signature, simple_hash, VariableSignature};

fn main() {
    const SIG: VariableSignature<f32> = const_signature!(varname: 32.0);

    assert!(simple_hash("varname") == SIG.id);

    println!("Const signature: {:?}", SIG);
}
