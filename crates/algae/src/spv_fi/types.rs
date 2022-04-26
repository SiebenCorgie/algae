use glam::{IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use rspirv::{
    dr::{
        Instruction, Module,
        Operand::{self, LiteralInt32},
    },
    spirv::{Op, Word},
};

use crate::{DataId, Serializer};

use super::SpvError;


///Runtime representation of a spirv type. Can either be parsed from an instruction,
///or derived from a rust type at runtime via the [IntoSpvType](IntoSpvType) trait.
#[derive(Clone, Debug)]
pub enum SpvType {
    Bool,
    Int {
        ///True if the int is sigend
        signed: bool,
        ///width in bit. Usually 8,16,32, sometimes 64.
        width: u32,
    },
    Float {
        ///Width in bit, usually either 16 or 32.
        width: u32,
    },
    Vec {
        ///Data type of the vec, usually a bool, int or float.
        data_type: Box<SpvType>,
        num_elements: u32,
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

impl SpvType {
    ///Parses a type `instruction` in the context of `module`. Returns None if either the instruction is not a type instruction,
    ///or internal parsing failed.
    pub fn from_instruction(module: &Module, instruction: &Instruction) -> Result<Self, SpvError> {
        if !is_op_type(&instruction.class.opcode) {
            #[cfg(feature = "logging")]
            log::error!(
                "Tried to parse non-type instruction as type. Instruction was: {:#?}",
                instruction
            );

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

            Op::TypeBool => Ok(SpvType::Bool),
            Op::TypeInt => match (&instruction.operands[0], &instruction.operands[1]) {
                (LiteralInt32(width), LiteralInt32(sign)) => match sign {
                    0 => Ok(SpvType::Int {
                        signed: false,
                        width: *width,
                    }),
                    1 => Ok(SpvType::Int {
                        signed: true,
                        width: *width,
                    }),
                    _ => {
                        #[cfg(feature = "logging")]
                        log::error!("Tried to parse integer type, but sign was neither 0 not 1 as expected. Instruction:\n{:#?}", instruction);

                        Err(SpvError::TypeUnparsable)
                    }
                },
                _ => {
                    #[cfg(feature = "logging")]
                    log::error!("Tried to parse integer type, but operand 0/1 where no integer literals as expected. Instruction:\n{:#?}", instruction);

                    Err(SpvError::TypeUnparsable)
                }
            },
            Op::TypeFloat => match &instruction.operands[0] {
                LiteralInt32(width) => Ok(SpvType::Float { width: *width }),
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

                if let LiteralInt32(n) = &instruction.operands[1] {
                    Ok(SpvType::Vec {
                        data_type: Box::new(vec_type),
                        num_elements: *n,
                    })
                } else {
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
                    (
                        SpvType::Vec {
                            data_type,
                            num_elements,
                        },
                        LiteralInt32(width),
                    ) => Ok(SpvType::Matrix {
                        data_type: Box::new(*data_type),
                        height: num_elements,
                        width: *width,
                    }),
                    _ => {
                        #[cfg(feature = "logging")]
                        log::error!("Tried to parse matrix type, but either the element type was no vector, or the width argument no integer literal. Instruction:\n{:#?}", instruction);

                        Err(SpvError::TypeUnparsable)
                    }
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
                    let length_type =
                        SpvType::from_instruction(module, type_instruction(module, *r))?;
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
                log::error!(
                    "Tried to parse type, but was no supported type. Instruction:\n{:#?}",
                    instruction
                );

                Err(SpvError::TypeUnparsable)
            }
        }
    }

    ///Searches for, or creates the type Id for `self`. Returns None if SpvType is a float or integer literal.
    pub fn spirv_type_id(&self, serializer: &mut Serializer) -> Option<Word> {
        match self {
            SpvType::Bool => Some(serializer.builder_mut().type_bool()),
            SpvType::Float { width } => Some(serializer.builder_mut().type_float(*width)),
            SpvType::Int { signed, width } => Some(
                serializer
                    .builder_mut()
                    .type_int(*width, if *signed { 1 } else { 0 }),
            ),
            SpvType::Vec {
                data_type,
                num_elements,
            } => {
                let sub_element_type = data_type.spirv_type_id(serializer)?;
                Some(
                    serializer
                        .builder_mut()
                        .type_vector(sub_element_type, *num_elements),
                )
            }
            SpvType::Matrix {
                data_type,
                width,
                height,
            } => {
                //Matrix is build from collumns of vecs, therefore get the vecs type id.
                let vec_type = SpvType::Vec {
                    data_type: data_type.clone(),
                    num_elements: *height,
                }
                .spirv_type_id(serializer)?;
                //Now build the vec trom it
                Some(serializer.builder_mut().type_matrix(vec_type, *width))
            }
            SpvType::Struct { elements } => {
                //TODO kinda unoptimitzed, would be nicer to not allocate here. But we need to check if we got a type Id for echt element.
                let mut element_ids = Vec::with_capacity(elements.len());
                for e in elements {
                    if let Some(eid) = e.spirv_type_id(serializer) {
                        element_ids.push(eid);
                    } else {
                        return None;
                    }
                }
                Some(serializer.builder_mut().type_struct(element_ids))
            }
            _ => None,
        }
    }
}

impl PartialEq for SpvType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SpvType::Bool, SpvType::Bool) => true,
            (
                SpvType::Int {
                    signed: a_signed,
                    width: a_width,
                },
                SpvType::Int {
                    signed: b_signed,
                    width: b_width,
                },
            ) => a_signed == b_signed && a_width == b_width,
            (SpvType::Float { width: a_width }, SpvType::Float { width: b_width }) => {
                a_width == b_width
            }
            (
                SpvType::Vec {
                    data_type: adt,
                    num_elements: ane,
                },
                SpvType::Vec {
                    data_type: bdt,
                    num_elements: bne,
                },
            ) => adt == bdt && ane == bne,
            (
                SpvType::Matrix {
                    data_type: adt,
                    height: ah,
                    width: aw,
                },
                SpvType::Matrix {
                    data_type: bdt,
                    height: bh,
                    width: bw,
                },
            ) => adt == bdt && aw == bw && ah == bh,
            (SpvType::Struct { elements: ae }, SpvType::Struct { elements: be }) => {
                for (a, b) in ae.iter().zip(be.iter()) {
                    if a != b {
                        return false;
                    }
                }

                true
            }
            (
                SpvType::Array {
                    data_type: adt,
                    num_elements: ane,
                },
                SpvType::Array {
                    data_type: bdt,
                    num_elements: bne,
                },
            ) => adt == bdt && ane == bne,
            (SpvType::LiteralFloat32(a), SpvType::LiteralFloat32(b)) => a == b,
            (SpvType::LiteralFloat64(a), SpvType::LiteralFloat64(b)) => a == b,
            (SpvType::LiteralInt32(a), SpvType::LiteralInt32(b)) => a == b,
            (SpvType::LiteralInt64(a), SpvType::LiteralInt64(b)) => a == b,
            _ => false,
        }
    }
}

///If implemented allows a type to be reflected into a spirv type at runtime
pub trait IntoSpvType {
    ///Returns the SpvType version of `Self`.
    fn into_spv_type() -> SpvType;
    ///Serializes `self` as a constant into `serializer`, returning the `DataId<Self>` to this constant.
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self>
    where
        Self: Sized;
    ///Shorthand for `Self::into_spv_type().spirv_type_id(serializer)`. Returns a types id in the context
    /// of a certain serializer.
    fn spirv_type_id(serializer: &mut Serializer) -> Option<Word> {
        Self::into_spv_type().spirv_type_id(serializer)
    }
}

impl IntoSpvType for bool {
    fn into_spv_type() -> SpvType {
        SpvType::Bool
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        if *self {
            DataId::from(serializer.builder_mut().constant_true(ty))
        } else {
            DataId::from(serializer.builder_mut().constant_false(ty))
        }
    }
}
impl IntoSpvType for f32 {
    fn into_spv_type() -> SpvType {
        SpvType::Float { width: 32 }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_f32(ty, *self))
    }
}
impl IntoSpvType for f64 {
    fn into_spv_type() -> SpvType {
        SpvType::Float { width: 64 }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_f64(ty, *self))
    }
}
impl IntoSpvType for i32 {
    fn into_spv_type() -> SpvType {
        SpvType::Int {
            signed: true,
            width: 32,
        }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .constant_u32(ty, u32::from_be_bytes(self.to_be_bytes())),
        ) //note constructing unsigend version of the i32.
    }
}
impl IntoSpvType for i64 {
    fn into_spv_type() -> SpvType {
        SpvType::Int {
            signed: true,
            width: 64,
        }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .constant_u64(ty, u64::from_be_bytes(self.to_be_bytes())),
        ) //note constructing unsigend version of the i32.
    }
}
impl IntoSpvType for u32 {
    fn into_spv_type() -> SpvType {
        SpvType::Int {
            signed: false,
            width: 32,
        }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_u32(ty, *self))
    }
}
impl IntoSpvType for u64 {
    fn into_spv_type() -> SpvType {
        SpvType::Int {
            signed: false,
            width: 64,
        }
    }
    fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self> {
        let ty = Self::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_u64(ty, *self))
    }
}

///Implements IntoSpvType for a glam vector
macro_rules! impl_into_spv_vec{
    ($vecty:ty, $basety:ty, $ne:expr, $($element_name:ident),+) => {
        impl IntoSpvType for $vecty {
            fn into_spv_type() -> SpvType {
                SpvType::Vec {
                    data_type: Box::new(<$basety>::into_spv_type()),
                    num_elements: $ne,
                }
            }
            fn constant_serialize(&self, serializer: &mut Serializer) -> DataId<Self>{
                let ty = Self::spirv_type_id(serializer).unwrap();

                let ids = [
                    $(
                        self.$element_name.constant_serialize(serializer).id
                    ),+
                ];
                DataId::from(
                    serializer.builder_mut().constant_composite(
                        ty,
                        ids
                    )
                )
            }
        }
    }
}

impl_into_spv_vec!(Vec2, f32, 2, x, y);
impl_into_spv_vec!(Vec3, f32, 3, x, y, z);
impl_into_spv_vec!(Vec4, f32, 4, x, y, z, w);

impl_into_spv_vec!(IVec2, i32, 2, x, y);
impl_into_spv_vec!(IVec3, i32, 3, x, y, z);
impl_into_spv_vec!(IVec4, i32, 4, x, y, z, w);

impl_into_spv_vec!(UVec2, u32, 2, x, y);
impl_into_spv_vec!(UVec3, u32, 3, x, y, z);
impl_into_spv_vec!(UVec4, u32, 4, x, y, z, w);

/* TODO implement matrix types as constant
impl IntoSpvType for Mat2 {
    fn into_spv_type() -> SpvType {
        SpvType::Matrix {
            data_type: Box::new(f32::into_spv_type()),
            width: 2,
            height: 2,
        }
    }
}
impl IntoSpvType for Mat3 {
    fn into_spv_type() -> SpvType {
        SpvType::Matrix {
            data_type: Box::new(f32::into_spv_type()),
            width: 3,
            height: 3,
        }
    }
}
impl IntoSpvType for Mat4 {
    fn into_spv_type() -> SpvType {
        SpvType::Matrix {
            data_type: Box::new(f32::into_spv_type()),
            width: 4,
            height: 4,
        }
    }
}
*/
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
    //let data_id = type_instruction.result_id.unwrap();
    let descriptor = SpvType::from_instruction(module, &type_instruction)?;

    //we assume that the parameter type is a struct where the first field is a constant u32 representing the hash.
    //Check that
    let ty = match descriptor {
        SpvType::Struct { mut elements } => {
            if elements[0]
                != (SpvType::Int {
                    width: 32,
                    signed: false,
                })
                && elements.len() == 2
            {
                #[cfg(feature = "logging")]
                log::error!(
                    "Tried to parse parameter, but parameter's first element is no integer."
                );

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
