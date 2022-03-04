use algae_gpu::simple_hash;


const STRINGS: [&'static str; 5] = [
        "test",
        "teddy",
        "testuslongusnames",
        "testuseventmorelongusnamesthatisprobablylongerthanweneed",
        "TestWithNon@ASCII¶¶ŧ←¢¢Characters“„”¶ŧe@µ“µ”¢„"
    ];


fn main(){
    println!("Hashing:");
    for s in &STRINGS{
        println!("    hash({}) = {}", s, simple_hash(s));
    }
}
