use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use algae::{
    spv_fi::{SpvError, SpvFi},
    DataId, Operation, Serializer,
};

use rspirv::{
    binary::{Assemble, Disassemble, Parser},
    dr::Loader,
};

#[derive(Debug)]
pub enum JitError {
    FailedToParseSpirvBinary,
    ///Happens if the SpirvBinary is valid, but there are errors in the algae specific entry point.
    FailedToParseEntrypoint(SpvError),
    CouldNotFindFunction(String),
}

impl core::fmt::Display for JitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JitError::FailedToParseSpirvBinary => write!(f, "Failed to parse SpirV Binary"),
            JitError::CouldNotFindFunction(fname) => write!(f,"Failed to find function with name: {} in spirv binary. Is the spirv binary compiled with debug information enabled?", fname),
            JitError::FailedToParseEntrypoint(e) => write!(f,"Failed to parse entry point: {e}"),
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
            injector: Injector::new(module, "test_shader::injector")?,
            binary: Vec::with_capacity(Self::BINARY_CAPACITY),
        })
    }

    pub fn injector(&mut self) -> &mut Injector {
        &mut self.injector
    }

    ///Returns the current SpirV byte code.
    pub fn get_module(&mut self) -> &[u32] {
        self.binary.clear();
        self.injector.module.assemble_into(&mut self.binary);
        &self.binary
    }
}

fn diff(s0: &str, s1: &str) {
    let mut f0 = std::fs::File::create("f0.txt").unwrap();
    f0.write_all(s0.as_bytes()).unwrap();

    let mut f0 = std::fs::File::create("f1.txt").unwrap();
    f0.write_all(s1.as_bytes()).unwrap();

    let output = std::process::Command::new("diff")
        .arg("--color")
        .arg("-y")
        .arg("f0.txt")
        .arg("f1.txt")
        .output()
        .unwrap();

    //remove
    std::fs::remove_file("f0.txt").unwrap();
    std::fs::remove_file("f1.txt").unwrap();

    println!(
        "DIFF: \n\n {} \n\n",
        core::str::from_utf8(&output.stdout).unwrap()
    );
}

///Keeps track where in `src` injection points are located
#[derive(Clone)]
pub struct Injector {
    ///The most up to date module of this function
    module: rspirv::dr::Module,

    //interface of the function at fid
    interface: SpvFi,
    ///inject function id
    fid: usize,
}

impl Injector {
    ///Tries to inject a function with the given input/output signature
    pub fn inject<I, O>(
        &mut self,
        input: I,
        function: &mut dyn Operation<Input = I, Output = DataId<O>>,
    ) {
        let mut working_builder = rspirv::dr::Builder::new_from_module(self.module.clone());

        //move to inject function. This should not fail, since the fi would otherwise not exist.
        working_builder
            .select_function(Some(self.fid))
            .expect("Failed to select inject function!");

        //Start out by creating a new blog in our builder.
        let new_block_id = working_builder.begin_block(None).unwrap();
        let inject_block = working_builder.selected_block().unwrap();

        //Now setup the serializer and start serializing the function
        let mut serializer = Serializer::new(&mut working_builder, &self.interface);

        //Serialize into function
        let return_value = function.serialize(&mut serializer, input);

        //Append the return value
        let ret = serializer.builder_mut().ret_value(return_value.id).unwrap();

        #[cfg(feature = "logging")]
        log::info!("Writing to block {}, id={}", inject_block, new_block_id);

        //We do now is inserting our payload as a second block into the function, afterwards we remove the original first block
        let function_index = working_builder.selected_function().unwrap();

        //Swap out blocks
        let mut new_module = working_builder.module();
        let injected_block = new_module.functions[function_index]
            .blocks
            .remove(inject_block);
        new_module.functions[function_index].blocks[0] = injected_block;

        let after_injection_module = new_module.clone().disassemble();

        diff(&self.module.disassemble(), &after_injection_module);
        //Switch module to the most recent one
        self.module = new_module;
    }

    pub fn new(module: rspirv::dr::Module, inject_function_name: &str) -> Result<Self, JitError> {
        #[cfg(feature = "logging")]
        {
            let original_module = module.disassemble();
            log::info!("{}", original_module);
        }

        //Create a builder to search and analyse the function interface.
        let mut builder = rspirv::dr::Builder::new_from_module(module.clone());

        //Pase the functions interface
        let fi = SpvFi::new(&module, &mut builder, inject_function_name)
            .map_err(|e| JitError::FailedToParseEntrypoint(e))?;

        //Move the builder to the inject function
        if let Err(_e) = builder.select_function_by_name(inject_function_name) {
            #[cfg(feature = "logging")]
            log::error!(
                "Could not select function \"{}\" with error: {}",
                inject_function_name,
                _e
            );

            return Err(JitError::CouldNotFindFunction(String::from(
                inject_function_name,
            )));
        }

        //now safe the functions id
        let fid = builder.selected_function().unwrap();

        Ok(Injector {
            module,
            interface: fi,
            fid,
        })
    }
}
