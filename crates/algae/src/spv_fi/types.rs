use glam::{Vec2, Vec3, Vec4, IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Mat2, Mat3, Mat4};
use rspirv::{
    dr::{
        Builder, Instruction, Module,
        Operand::{self, LiteralInt32},
    },
    spirv::{Op, Word},
};

use super::SpvError;



///Runtime representation of a spirv type. Can either be parsed from an instruction,
///or derived from a rust type at runtime via the [IntoSpvType](IntoSpvType) trait.
#[derive(Clone, Debug)]
pub enum SpvType {
    Void,
    Bool,
    Int{
        ///True if the int is sigend
        signed: bool,
        ///width in bit. Usually 8,16,32, sometimes 64. 
        width: u32
    },
    Float{
        ///Width in bit, usually either 16 or 32.
        width: u32
    },
    Vec{
        ///Data type of the vec, usually a bool, int or float.
        data_type: Box<SpvType>,
        num_elements: u32
    },
    Matrix {
        data_type: Box<SpvType>,
        width: u32,
        height: u32,
    },
    Struct {
        elements: Vec<SpvType>,
    },
    Array {
        data_type: Box<SpvType>,
        num_elements: u32,
    },
    LiteralFloat32(f32),
    LiteralFloat64(f64),
    LiteralInt32(u32),
    LiteralInt64(u64),
}

impl SpvType{
    ///Parses a type `instruction` in the context of `module`. Returns None if either the instruction is not a type instruction,
    ///or internal parsing failed. 
    pub fn from_instruction(module: &Module, instruction: &Instruction) -> Result<Self, SpvError>{

        if !is_op_type(&instruction.class.opcode){
            #[cfg(feature = "logging")]
            log::error!("Tried to parse non-type instruction as type. Instruction was: {:#?}", instruction);
            
            return Err(SpvError::NoTypeInstruction);
        }

        match instruction.class.opcode {
            
            Op::Constant => match &instruction.operands[0] {
                Operand::LiteralFloat32(f) => Ok(SpvType::LiteralFloat32(*f)),
                Operand::LiteralFloat64(f) => Ok(SpvType::LiteralFloat64(*f)),
                Operand::LiteralInt32(i) => Ok(SpvType::LiteralInt32(*i)),
                Operand::LiteralInt64(i) => Ok(SpvType::LiteralInt64(*i)),
                _ => {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse constant instruction but was neither int nor float. Instruction was: {:#?}", instruction);
                    
                    Err(SpvError::TypeUnparsable)
                }
            },
            
            Op::TypeVoid => Ok(SpvType::Void),
            Op::TypeBool => Ok(SpvType::Bool),
            Op::TypeInt => match (&instruction.operands[0], &instruction.operands[1]) {
                (LiteralInt32(width), LiteralInt32(sign)) => {
                    match sign{
                        0 => Ok(SpvType::Int{
                            signed: false,
                            width: *width
                        }),
                        1 => Ok(SpvType::Int{
                            signed: true,
                            width: *width
                        }),
                        _ => {
                            #[cfg(feature = "logging")]
                            log::error!("Tried to parse integer type, but sign was neither 0 not 1 as expected. Instruction:\n{:#?}", instruction);

                            Err(SpvError::TypeUnparsable)
                        }
                    }
                },
                _ => {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse integer type, but operand 0/1 where no integer literals as expected. Instruction:\n{:#?}", instruction);

                    Err(SpvError::TypeUnparsable)
                },
            },
            Op::TypeFloat => match &instruction.operands[0] {
                LiteralInt32(width) => Ok(SpvType::Float{width: *width}),
                _ => {
                    
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse float type, but operand 0 was no integer literals as expected. Instruction:\n{:#?}", instruction);

                    Err(SpvError::TypeUnparsable)
                }
            },
            
            Op::TypeVector => {
                
                //parse the vectors arg type
                let arg_id = if let Operand::IdRef(id) = &instruction.operands[0] {
                    *id
                } else {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse vector type, but operand 0 was no IdRef as expected. Instruction:\n{:#?}", instruction);

                    return Err(SpvError::TypeUnparsable);
                };

                //Now find the correct instruction based on the argument id
                let arg_instruction = type_instruction(module, arg_id);

                let vec_type = SpvType::from_instruction(module, &arg_instruction)?;

                if let LiteralInt32(n) = &instruction.operands[1]{
                    Ok(SpvType::Vec {
                        data_type: Box::new(vec_type),
                        num_elements: *n,
                    })
                }else{
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse vector type, but operand 1 was no integer literals as expected. Instruction:\n{:#?}", instruction);

                    Err(SpvError::TypeUnparsable)
                }
            }
            Op::TypeMatrix => {

                                //parse the vectors arg type
                let arg_id = if let Operand::IdRef(id) = &instruction.operands[0] {
                    *id
                } else {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse vector type, but operand 0 was no IdRef as expected. Instruction:\n{:#?}", instruction);

                    return Err(SpvError::TypeUnparsable);
                };

                //Now find the correct instruction based on the argument id
                let arg_instruction = type_instruction(module, arg_id);

                let matrix_type = SpvType::from_instruction(module, &arg_instruction)?;

                match (matrix_type, &instruction.operands[1]) {
                    (SpvType::Vec {
                        data_type,
                        num_elements,
                    }, LiteralInt32(width)) => Ok(SpvType::Matrix {
                        data_type: Box::new(*data_type),
                        height: num_elements,
                        width: *width,
                    }),
                    _ => {
                        
                        #[cfg(feature = "logging")]
                        log::error!("Tried to parse matrix type, but either the element type was no vector, or the width argument no integer literal. Instruction:\n{:#?}", instruction);

                        Err(SpvError::TypeUnparsable)
                    },
                }
            }
            Op::TypeStruct => {

                //When parsing a struct we generate basically a list of fields of this struct
                let mut elements = Vec::with_capacity(instruction.operands.len());

                for op in &instruction.operands {
                    let arg_instruction = match op {
                        Operand::IdRef(id) => type_instruction(module, *id),
                        _ => {
                            #[cfg(feature = "logging")]
                            log::error!("Tried to parse struct type, but one of the operands was no IdRef as expected. Instruction:\n{:#?}", instruction);

                            return Err(SpvError::TypeUnparsable);
                        }
                    };

                    let element_type = SpvType::from_instruction(module, arg_instruction)?;
                    elements.push(element_type);
                }

                Ok(SpvType::Struct { elements })
            }
            Op::TypeArray => {
                //Get the arrays type and length
                let array_type = if let Operand::IdRef(r) = &instruction.operands[0] {
                    SpvType::from_instruction(module, type_instruction(module, *r))?
                } else {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse array type, but operand 0 was no IdRef as expected. Instruction:\n{:#?}", instruction);

                    return Err(SpvError::TypeUnparsable);
                };

                let array_length = if let Operand::IdRef(r) = &instruction.operands[1] {
                    let length_type = SpvType::from_instruction(module, type_instruction(module, *r))?;
                    if let SpvType::LiteralInt32(len) = length_type {
                        len
                    } else {
                        #[cfg(feature = "logging")]
                        log::error!("Tried to parse array type, but operand 1 was no LiteralInt32 as expected. Instruction:\n{:#?}", instruction);

                        return Err(SpvError::TypeUnparsable);
                    }
                } else {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse array type, but operand 0 was no IdRef as expected. Instruction:\n{:#?}", instruction);

                    return Err(SpvError::TypeUnparsable);
                };

                Ok(SpvType::Array {
                    data_type: Box::new(array_type),
                    num_elements: array_length,
                })
            }
            _ => {
                #[cfg(feature = "logging")]
                log::error!("Tried to parse type, but was no supported type. Instruction:\n{:#?}", instruction);

                Err(SpvError::TypeUnparsable)
            },
        }

    }
}

impl PartialEq for SpvType{
    fn eq(&self, other: &Self) -> bool{
        match (self, other){
            (SpvType::Void, SpvType::Void) => true,
            (SpvType::Bool, SpvType::Bool) => true,
            (SpvType::Int{signed: a_signed, width: a_width}, SpvType::Int{signed: b_signed, width: b_width}) => a_signed == b_signed && a_width == b_width,
            (SpvType::Float{width: a_width}, SpvType::Float{width: b_width}) => a_width == b_width,
            (SpvType::Vec{data_type: adt, num_elements: ane}, SpvType::Vec{data_type: bdt, num_elements: bne}) => adt == bdt && ane == bne,
            (SpvType::Matrix{data_type: adt, height: ah, width: aw}, SpvType::Matrix{data_type: bdt, height: bh, width: bw}) => adt == bdt && aw == bw && ah == bh,
            (SpvType::Struct{elements: ae}, SpvType::Struct{elements: be}) => {
                for (a,b) in ae.iter().zip(be.iter()){
                    if a != b{
                        return false
                    }
                }

                true
            },
            (SpvType::Array{data_type: adt, num_elements: ane}, SpvType::Array{data_type: bdt, num_elements: bne}) => adt == bdt && ane == bne,
            (SpvType::LiteralFloat32(a), SpvType::LiteralFloat32(b)) => a == b,
            (SpvType::LiteralFloat64(a), SpvType::LiteralFloat64(b)) => a == b,
            (SpvType::LiteralInt32(a), SpvType::LiteralInt32(b)) => a == b,
            (SpvType::LiteralInt64(a), SpvType::LiteralInt64(b)) => a == b,
            _ => false
        }
    }
}

///If implemented allows a type to be reflected into a spirv type at runtime
pub trait IntoSpvType{
    fn into_spv_type() -> SpvType;
}


impl IntoSpvType for (){
    fn into_spv_type() -> SpvType{ SpvType::Void }
}
impl IntoSpvType for bool{
    fn into_spv_type() -> SpvType{ SpvType::Bool }
}
impl IntoSpvType for f32{
    fn into_spv_type() -> SpvType{ SpvType::Float{width: 32} }
}
impl IntoSpvType for f64{
    fn into_spv_type() -> SpvType{ SpvType::Float{width: 64} }
}
impl IntoSpvType for i32{
    fn into_spv_type() -> SpvType{ SpvType::Int{signed: true, width: 32}}
}
impl IntoSpvType for i64{
    fn into_spv_type() -> SpvType{ SpvType::Int{signed: true, width: 64}}
}
impl IntoSpvType for u32{
    fn into_spv_type() -> SpvType{ SpvType::Int{signed: false, width: 32}}
}
impl IntoSpvType for u64{
    fn into_spv_type() -> SpvType{ SpvType::Int{signed: false, width: 64}}
}


impl IntoSpvType for Vec2{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(f32::into_spv_type()), num_elements: 2}}
}
impl IntoSpvType for Vec3{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(f32::into_spv_type()), num_elements: 3}}
}
impl IntoSpvType for Vec4{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(f32::into_spv_type()), num_elements: 4}}
}


impl IntoSpvType for IVec2{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(i32::into_spv_type()), num_elements: 2}}
}
impl IntoSpvType for IVec3{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(i32::into_spv_type()), num_elements: 3}}
}
impl IntoSpvType for IVec4{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(i32::into_spv_type()), num_elements: 4}}
}

impl IntoSpvType for UVec2{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(u32::into_spv_type()), num_elements: 2}}
}
impl IntoSpvType for UVec3{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(u32::into_spv_type()), num_elements: 3}}
}
impl IntoSpvType for UVec4{
    fn into_spv_type() -> SpvType{ SpvType::Vec{data_type: Box::new(u32::into_spv_type()), num_elements: 4}}
}


impl IntoSpvType for Mat2{
    fn into_spv_type() -> SpvType{ SpvType::Matrix{data_type: Box::new(f32::into_spv_type()), width: 2, height: 2}}
}
impl IntoSpvType for Mat3{
    fn into_spv_type() -> SpvType{ SpvType::Matrix{data_type: Box::new(f32::into_spv_type()), width: 3, height: 3}}
}
impl IntoSpvType for Mat4{
    fn into_spv_type() -> SpvType{ SpvType::Matrix{data_type: Box::new(f32::into_spv_type()), width: 4, height: 4}}
}

fn is_op_type(op: &Op) -> bool {
    match op {
        Op::TypeVoid
        | Op::TypeBool
        | Op::TypeInt
        | Op::TypeFloat
        | Op::TypeVector
        | Op::TypeMatrix
        | Op::TypeImage
        | Op::TypeSampler
        | Op::TypeSampledImage
        | Op::TypeArray
        | Op::TypeRuntimeArray
        | Op::TypeStruct
        | Op::TypeOpaque
        | Op::TypePointer
        | Op::TypeFunction
        | Op::TypeEvent
        | Op::TypeDeviceEvent
        | Op::TypeReserveId
        | Op::TypeQueue
        | Op::TypePipe
        | Op::Constant
        | Op::TypeForwardPointer => true,
        _ => false,
    }
}

//Gets the type instruction from a type id
fn type_instruction(module: &Module, type_code: Word) -> &Instruction {
    module
        .types_global_values
        .iter()
        .filter(|x| x.result_id == Some(type_code))
        .next()
        .unwrap()
}

fn instruction_by_id(module: &Module, code: Word) -> &Instruction{
    module.all_inst_iter()
        .filter(|x| x.result_id == Some(code))
        .next()
        .unwrap()
}

#[derive(Clone, Debug)]
pub struct Parameter {
    ///Id of the composite/function parameter to load this parameter from
    pub composite_id: Word,
    ///Spirv id of this element's type
    pub spirv_type_id: Word,
    ///Hash of this parameters name
    pub name_hash: u32,
    ///The inner spirv type of the second element of. Basically parsed version of `spirv_type_id`. Used for type comparison.
    pub ty: SpvType,
}

///Parses the `operand` within the context of `module`.
pub fn parse_parameter(module: &Module, operand: &Instruction) -> Result<Parameter, SpvError> {
    let op_type = operand.result_type.unwrap();
    let type_instruction = type_instruction(module, op_type);
    let data_id = type_instruction.result_id.unwrap();
    let descriptor = SpvType::from_instruction(module, &type_instruction)?;

    //we assume that the parameter type is a struct where the first field is a constant u32 representing the hash.
    //Check that
    let ty = match descriptor{
        SpvType::Struct{mut elements} => {
            if elements[0] != (SpvType::Int{width: 32, signed: false}) && elements.len() == 2{
                #[cfg(feature = "logging")]
                log::error!("Tried to parse parameter, but parameter's first element is no integer.");

                return Err(SpvError::ParameterNoVariableDescriptor);
            }

            elements.remove(1)
        }
        _ => {
            #[cfg(feature = "logging")]
            log::error!("Tried to parse parameter, but parameter descriptor was malformed.");

            return Err(SpvError::ParameterNoVariableDescriptor);
        }
    };

    Ok(Parameter {
        composite_id: operand.result_id.unwrap(),
        name_hash: 0,
        spirv_type_id: 0,
        ty,
    })
}
