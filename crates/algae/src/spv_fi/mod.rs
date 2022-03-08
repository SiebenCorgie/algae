use std::error::Error;

use rspirv::{
    dr::{Builder, Module, Operand},
    spirv::Op,
};

mod types;
pub use types::{parse_parameter, IntoSpvType, Parameter, SpvType};

///Errors that can occurs while working with raw Spv data. For instance when parsing types, or searching for algaes entry point.
#[derive(Clone, Debug)]
pub enum SpvError {
    ///Occurs when a instruction is parsed into a spv type, but the instruction is no SpirV type instruction.
    NoTypeInstruction,
    ///Occurs when the entry function for algae is called with a parameter that is not wrapped into a variable descriptor.
    ///This means that the variables name_hash can not be found.
    ParameterNoVariableDescriptor,
    ///Occurs when a instruction is a type instruction, but cannot be parsed into the [SpvType](types::SpvType) struct for some other reason.
    TypeUnparsable,
}

impl std::fmt::Display for SpvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpvError::NoTypeInstruction => write!(f, "Instruction was no SpirV Type instruction."),
            SpvError::ParameterNoVariableDescriptor => write!(
                f,
                "Injection parameter is malformed. Please check the injection macros output!"
            ),
            SpvError::TypeUnparsable => write!(f, "Type cannot be parsed for unknown reason"),
        }
    }
}

impl Error for SpvError {}

///Filters `module` for `id` assuming this is a integer constant and returns it.
fn find_integer_constant(module: &Module, id: &Operand) -> u32 {
    module
        .all_inst_iter()
        .find_map(|element| {
            if element.class.opcode == Op::Constant && element.result_id == Some(id.unwrap_id_ref())
            {
                Some(element.operands[0].unwrap_literal_int32())
            } else {
                None
            }
        })
        .unwrap()
}

///Searches for the result of the composite or load operation with `id`.
fn find_parameter_spv_type_id(module: &Module, id: &Operand) -> u32 {
    module
        .all_inst_iter()
        .find_map(|element| {
            //Check if this is the id of the searched for parameters load/construction
            //If so, retrieve the type id of this load/construct
            if element.result_id == Some(id.unwrap_id_ref()) {
                match element.class.opcode {
                    Op::CompositeConstruct | Op::Load => Some(element.result_type.unwrap()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap()
}

///Runtime function interface of an injection point.
///
/// At runtime the spirv module dictates the input and output type of the injected function. The JIT-Compiler has to make sure that
/// only a function adhering to those requirements is injected.
#[derive(Clone)]
pub struct SpvFi {
    pub parameter: Vec<Parameter>,
}

impl SpvFi {
    ///Analyses a functions interface
    pub fn new(
        module: &Module,
        builder: &mut Builder,
        entry_function_name: &str,
    ) -> Result<Self, SpvError> {
        //Select the targeted function
        builder
            .select_function_by_name(entry_function_name)
            .expect("Could not get sdf function");

        //get the id and use that for indexing the module
        let function_id = builder.selected_function().unwrap();
        let abs_function = module
            .functions
            .get(function_id)
            .expect("Failed to get function based on id");

        //We are now parsing the parameters of the function to rust types. We do this by recursively parsing the operands
        let mut parameter: Vec<_> = abs_function
            .parameters
            .iter()
            .filter_map(|p| match parse_parameter(module, p) {
                Ok(r) => Some(r),
                Err(_e) => {
                    #[cfg(feature = "logging")]
                    log::warn!("Could not parse parameter {}:\n{:#?}", _e, p);
                    None
                }
            })
            .collect();

        //after parsing the functions parameter type, search for the function call
        // and check out the constant values of the id fields (which must be the first field of each parameter).
        let mut function_call_op = None;
        let mut ops = module.all_inst_iter().enumerate();
        while let (Some((idx, op)), None) = (ops.next(), function_call_op) {
            if let Op::FunctionCall = op.class.opcode {
                function_call_op = Some(idx); //mark idx
            }
        }

        //now got to function call and move back until there are no OpCompositeConstructs left
        //FIXME: this is kind of shaky. But should work since our macro is generating this code anyways.
        //       Could be made more stable by at least checking that we are hooking on the correct "no inline" function call and not some
        //       random other uninlined function.
        //NOTE: we know the number of parameters, we just want to assign the correct hash
        //
        let constuct_idx_start = function_call_op.unwrap() - parameter.len();
        for (idx, par) in parameter.iter_mut().enumerate() {
            let composite = module
                .all_inst_iter()
                .nth(constuct_idx_start + idx)
                .unwrap();

            par.name_hash = find_integer_constant(module, &composite.operands[0]);
            par.spirv_type_id = find_parameter_spv_type_id(module, &composite.operands[1]);
        }

        Ok(SpvFi { parameter })
    }

    ///Tries to find a variable of type `T` in the runtime signature of the function. Returns the data id  at which the data is loaded if one is found. Otherwise the variables defined default value is loaded there.
    /// or nothing.
    pub fn get_parameter<T: IntoSpvType>(
        &self,
        hash: u32,
        spvtype: &SpvType,
    ) -> Option<&Parameter> {
        for p in &self.parameter {
            if p.name_hash == hash && spvtype == &p.ty {
                return Some(p);
            }
        }
        None
    }
}
