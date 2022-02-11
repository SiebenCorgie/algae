//! # 🦠 Algae
//! 
//! Algae is the algebra layer that can be jit compiled into an existing SpirV module.
//!

pub use glam;

use std::marker::PhantomData;

pub use rspirv;
use rspirv::{dr::Builder, spirv::Word};

pub mod operations;



///Serializer used for building the spirv bytecode
pub type Serializer = Builder;

///Provides several methods to make working with the serializer easier.
trait SerializerExt{
    fn type_void(&mut self) -> Word;
    fn type_f32(&mut self) -> Word;
    fn type_vec_f32(&mut self, n_components: usize) -> Word;
    fn type_mat_f32(&mut self, width: usize, height: usize) -> Word;

    fn type_u32(&mut self) -> Word;
    fn type_i32(&mut self) -> Word;
}

impl SerializerExt for Serializer{
    fn type_void(&mut self) -> Word{
        self.type_void()
    }

    fn type_f32(&mut self) -> Word{
        self.type_float(32)
    }

    fn type_vec_f32(&mut self, n_components: usize) -> Word{
        let tf32 = self.type_f32();
        self.type_vector(tf32, n_components as u32)
    }
    
    ///returns the type of 4x4 f32 matrix
    fn type_mat_f32(&mut self, width: usize, height: usize) -> Word{
        let tvec4 = self.type_vec_f32(height);
        self.type_matrix(tvec4, width as u32)
    }

    fn type_u32(&mut self) -> Word{
        self.type_int(32, 0)
    }
    
    fn type_i32(&mut self) -> Word{
        self.type_int(32, 1)
    }
}


///Type data id with type tag
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct DataId<T>{
    pub id: Word,
    ty: PhantomData<T>
}

impl<T> From<Word> for DataId<T>{
    fn from(word: Word) -> Self {
        DataId{
            id: word,
            ty: PhantomData
        }
    }
}

pub type BoxOperation<I, O> = Box<dyn Operation<Input = I, Output = DataId<O>>>;

pub trait Operation{
    type Input;
    type Output;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output;
}


