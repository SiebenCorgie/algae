use std::{error::Error, fs::File, io::Read, path::Path};

use rspirv::{
    binary::{Assemble, Disassemble, Parser},
    dr::Loader,
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
    ///Data level representation of the loaded (template) spirv module
    module: rspirv::dr::Module,

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

        assert!(module.disassemble().len() > 0, "module is empty");

        println!("Loaded spirv module:\n\n {}", module.disassemble());

        Ok(AlgaeJit {
            module,
            binary: Vec::with_capacity(Self::BINARY_CAPACITY),
        })
    }

    ///Returns the current SpirV byte code.
    pub fn get_module(&mut self) -> &[u32] {
        self.binary.clear();
        self.module.assemble_into(&mut self.binary);
        &self.binary
    }
}
