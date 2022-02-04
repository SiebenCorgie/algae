use std::{error::Error, fs::File, io::{Read, Write}, path::Path};

use rspirv::{
    binary::{Assemble, Disassemble, Parser},
    dr::{Loader, Builder}, spirv::{Word, self},
};

#[derive(Debug)]
pub enum JitError {
    FailedToParseSpirvBinary,
}

impl core::fmt::Display for JitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JitError::FailedToParseSpirvBinary => write!(f, "Failed to parse SpirV Binary"),
        }
    }
}

impl Error for JitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

///The main JIT compiler for algae functions. Starts by loading a spirv module from a file and searching for one, or several
/// occasions of algae signatures. At runtime those functions can be queried and replaced with appropriate Algae functions.
#[derive(Clone)]
pub struct AlgaeJit {
    injector: Injector,
    ///assembled shader binary structure
    binary: Vec<u32>,
}

impl AlgaeJit {
    ///Standard number of words the caching vec holds.
    const BINARY_CAPACITY: usize = 500;

    ///Loads the spirv module. Returns an error if the spirv module is invalid.
    pub fn new(spirv_module: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let mut spirv_file = File::open(spirv_module)?;
        let size = spirv_file.metadata().unwrap().len();

        assert!(size > 0);

        let mut code: Vec<u8> = Vec::with_capacity(size as usize);
        let read = spirv_file.read_to_end(&mut code)?;
        assert!(read as u64 == size, "Failed to read whole spirv file");

        println!("Loading {}b, spirv", read);

        //parse bytes into spirv words
        let mut loader = Loader::new();
        let parser = Parser::new(&code, &mut loader);
        parser.parse()?;

        //load the spirv module and parse it
        let module = loader.module();        
        
        Ok(AlgaeJit {
            injector: Injector::new(module),
            binary: Vec::with_capacity(Self::BINARY_CAPACITY),
        })
    }

    ///Returns the current SpirV byte code.
    pub fn get_module(&mut self) -> &[u32] {
        self.binary.clear();
        self.injector.module.assemble_into(&mut self.binary);
        &self.binary
    }
}

fn diff(s0: &str, s1: &str){

    let mut f0 = std::fs::File::create("f0.txt").unwrap();
    f0.write_all(s0.as_bytes()).unwrap();
    
    let mut f0 = std::fs::File::create("f1.txt").unwrap();
    f0.write_all(s1.as_bytes()).unwrap();
    
    let output = std::process::Command::new("diff")
        .arg("--color")
        .arg("-y")
        .arg("f0.txt")
        .arg("f1.txt")
        .output().unwrap();

    //remove
    std::fs::remove_file("f0.txt").unwrap();
    std::fs::remove_file("f1.txt").unwrap();


    println!("DIFF: \n\n {} \n\n", core::str::from_utf8(&output.stdout).unwrap());
    
}

///Keeps track where in `src` injection points are located
#[derive(Clone)]
struct Injector{
    module: rspirv::dr::Module,
    //Start and end line number of the injection function
    sdf_function: usize, 
}

impl Injector{

    fn inject(mut builder: Builder) -> Builder{
        //For now just inject a returned constant value
        let ty_float = builder.type_float(32);
      
        let constf32 = builder.constant_f32(ty_float, 0.0);

        builder.ret_value(constf32).unwrap();
        
        builder
    }

    pub fn new(module: rspirv::dr::Module) -> Self{
        
        let original_module = module.disassemble();
        
        //When creating, search the module for a function that has the SDF functions signature. Meaning
        //Two args (f32 and struct([f32;4])) and a return value of f32

        let mut builder = rspirv::dr::Builder::new_from_module(module.clone());
        builder.select_function_by_name("test_shader::sdf").expect("Could not get sdf function");

        let new_block_id = builder.begin_block(None).unwrap();
        let inject_block = builder.selected_block().unwrap();
        
        println!("Writing to block {}, id={}", inject_block, new_block_id);

        //We do now is inserting our payload as a second block into the function, afterwards we remove the original first block
        let function_index = builder.selected_function().unwrap();
        
        builder = Self::inject(builder);
        
        //Swap out blocks
        let mut new_module = builder.module();
        let injected_block = new_module.functions[function_index].blocks.remove(inject_block);
        new_module.functions[function_index].blocks[0] = injected_block;
        

        let after_injection_module = new_module.disassemble();

        diff(&original_module, &after_injection_module);
        
        Injector{
            module: new_module,
            sdf_function: 0
        }
    }
}
