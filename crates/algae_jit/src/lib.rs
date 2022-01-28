use std::{path::Path, fs::File, io::Read, error::Error};

use rspirv::{dr::Loader, binary::{parse_words, Disassemble, Parser}};

#[derive(Debug)]
pub enum JitError{
    FailedToParseSpirvBinary,
}

impl core::fmt::Display for JitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self{
            JitError::FailedToParseSpirvBinary => write!(f, "Failed to parse SpirV Binary")
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
pub struct AlgaeJit{
    module: rspirv::dr::Module,
}

impl AlgaeJit{

    ///Loads the spirv module. Returns an error if the spirv module is invalid.
    pub fn new(spirv_module: impl AsRef<Path>) -> Result<Self, Box<dyn Error>>{

        let mut spirv_file = File::open(spirv_module)?;
        let size = spirv_file.metadata().unwrap().len();
        let mut code: Vec<u8> = Vec::with_capacity(size as usize);
        let read = spirv_file.read_to_end(&mut code)?;
        assert!(read as u64 == size, "Failed to read whole spirv file");

        //parse bytes into spirv words
        let mut loader = Loader::new();
        Parser::new(&code, &mut loader);
        
        //load the spirv module and parse it
        let module = loader.module();

        println!("Loaded spirv module:\n\n {}", module.disassemble());
        
        Ok(AlgaeJit{
            module
        })
    }

    ///Returns the current SpirV byte code
    pub fn get_module(&self) -> Vec<u32>{
        todo!()
    }
}
