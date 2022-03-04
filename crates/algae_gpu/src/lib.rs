#![no_std]
#[deny(warnings)]

pub use algae_inject::algae_inject;

//Inject proc macro with function like synthax.
//pub use algae_inject::algae_inject;

///Simple xor hash based on a name
pub const fn simple_hash(name: &str) -> u32{
    //32bit xor hash atm. We do this by xoring every four bits of "name".
    //if name is not divisible by 32bit we stuff it with zeros
    let mut val: u32 = 0;

    let bytes = name.as_bytes();
    let num_bytes = bytes.len();
    //xor all bytes
    let mut idx = 0;
    while idx < num_bytes{        
        let a0 = bytes[idx] as u32; //Note the first allways succeeds baed on the check above
        val = val ^ (a0 << ((idx % 4) * 8));
        idx += 1;
    }
    
    val
}

#[macro_export]
macro_rules! const_signature{
    ($name:ident : $var:expr) => {
        {
            use algae_gpu::simple_hash;
            VariableSignature{
                id: simple_hash(stringify!($name)),
                value: $var
            }
        }
        
    }
}

///Signature of a variable. Keyed by its id and a runtime value
#[derive(Clone, Debug)]
pub struct VariableSignature<T>{
    ///The fields id based on s seahash of its name
    pub id: u32,
    pub value: T
}

impl<T> VariableSignature<T>{
    pub fn new(name: &'static str, value: T) -> Self{

        let id = simple_hash(name);

        VariableSignature{
            id,
            value
        }
    }
}




#[cfg(test)]
mod signature_tests{
    use crate::simple_hash;


    const TEST_NAMES: [&'static str; 5] = [
        "test",
        "teddy",
        "testuslongusnames",
        "testuseventmorelongusnamesthatisprobablylongerthanweneed",
        "TestWithNon@ASCII¶¶ŧ←¢¢Characters“„”¶ŧe@µ“µ”¢„"
    ];
    
    #[test]
    ///Generates a set of hashes and assesses equalness
    fn hash_eq(){

        for t in &TEST_NAMES{
            assert!(simple_hash(t) == simple_hash(t), "hash does not match");
            assert!(simple_hash(t) != 0, "Wrong hash");
        }
    }

    #[test]
    fn hash_collision(){
        //Test for collision in our set
        for i in 0..TEST_NAMES.len(){
            for o in (i+1) ..TEST_NAMES.len(){
                assert!(simple_hash(TEST_NAMES[i]) != simple_hash(TEST_NAMES[o]), "Hash Collision!");
            }
        }
    }
}
