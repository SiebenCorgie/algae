//! # 🦠 Algae
//!
//! Algae is the algebra layer that can be jit compiled into an existing SpirV module.
//!

pub use glam;
use spv_fi::SpvFi;

use std::marker::PhantomData;

pub use rspirv;
use rspirv::{dr::Builder, spirv::Word};

pub mod operations;

use operations::Constant;

///SpirV analyzer related functions.
pub mod spv_fi;
use spv_fi::IntoSpvType;

use algae_gpu::simple_hash;

///Runtime serializer of a algae function. `'a` is the SpirV-Builders's lifetime, `'b` is the injection functions lifetime.
pub struct Serializer<'a, 'b> {
    //The spirv builder
    pub(crate) builder: &'a mut Builder,
    pub(crate) interface: &'b SpvFi,
}

impl<'a, 'b> Serializer<'a, 'b> {
    pub fn new(builder: &'a mut Builder, interface: &'b SpvFi) -> Self {
        Serializer { builder, interface }
    }

    pub fn builder(&self) -> &Builder {
        &self.builder
    }

    pub fn builder_mut(&mut self) -> &mut Builder {
        &mut self.builder
    }

    ///Tries to find a variable of type `T` in the runtime signature of the function. Returns the data id  at which the data is loaded if one is found. Otherwise the variables defined default value is loaded there.
    /// or nothing.
    pub fn get_variable<T: IntoSpvType>(&mut self, name: &str, default_value: T) -> DataId<T>
    where
        Constant<T>: Operation<Input = (), Output = DataId<T>>,
    {
        let shash = simple_hash(name);
        let spvtype = T::into_spv_type();

        if let Some(param) = self.interface.get_parameter::<T>(shash, &spvtype) {
            //inline load procedure
            let did = self
                .builder
                .composite_extract(param.spirv_type_id, None, param.composite_id, [1])
                .unwrap();

            DataId {
                id: did,
                ty: PhantomData,
            }
        } else {
            #[cfg(feature = "logging")]
            log::warn!(
                "Could not find variable \"{}\" in function interface, falling back to constant",
                name
            );

            let mut con = Constant {
                value: default_value,
            };
            con.serialize(self, ())
        }
    }
}

///Type data id with type tag
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct DataId<T> {
    pub id: Word,
    ty: PhantomData<T>,
}

impl<T> From<Word> for DataId<T> {
    fn from(word: Word) -> Self {
        DataId {
            id: word,
            ty: PhantomData,
        }
    }
}

///Small abstraction over a boxed operation with input type `I` and output type `DataId<O>`.
pub type BoxOperation<I, O> = Box<dyn Operation<Input = I, Output = DataId<O>>>;

///Operation in an operation tree. `Input` can be used to pass down Jit-Compile-Time data, for instance for dynamic
/// feature selection. `Output` will usually be a [DataId](DataId) which tracks the return value of this operation.
///
/// Implementation assume that the operation serializes a valid code. Meaning that for instance the resulting `DataId<T>` actually
/// saves a value of type `T` at this id.
pub trait Operation {
    type Input;
    type Output;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output;
}
