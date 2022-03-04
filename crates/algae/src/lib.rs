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
use operations::native::Constant;

pub mod formula;
pub mod spv_fi;
use spv_fi::IntoSpvType;
use algae_gpu::simple_hash;

///Runtime serializer of a algae function. `'a` is the SpirV-Builders's lifetime, `'b` is the injection functions lifetime.
pub struct Serializer<'a, 'b>{
    //The spirv builder 
    pub(crate) builder: &'a mut Builder,
    pub(crate) interface: &'b SpvFi,
}

impl<'a, 'b> Serializer<'a, 'b>{

    pub fn new(builder: &'a mut Builder, interface: &'b SpvFi) -> Self{
        Serializer{
            builder,
            interface
        }
    }
    
    pub fn type_void(&mut self) -> Word{
        self.builder.type_void()
    }

    pub fn type_f32(&mut self) -> Word{
        self.builder.type_float(32)
    }

    pub fn type_vec_f32(&mut self, n_components: usize) -> Word{
        let tf32 = self.type_f32();
        self.builder.type_vector(tf32, n_components as u32)
    }
    
    ///returns the type of 4x4 f32 matrix
    pub fn type_mat_f32(&mut self, width: usize, height: usize) -> Word{
        let tvec4 = self.type_vec_f32(height);
        self.builder.type_matrix(tvec4, width as u32)
    }

    pub fn type_u32(&mut self) -> Word{
        self.builder.type_int(32, 0)
    }
    
    pub fn type_i32(&mut self) -> Word{
        self.builder.type_int(32, 1)
    }

    pub fn builder(&self) -> &Builder{
        &self.builder
    }

    pub fn builder_mut(&mut self) -> &mut Builder{
        &mut self.builder
    }

    
    ///Tries to find a variable of type `T` in the runtime signature of the function. Returns the data id  at which the data is loaded if one is found. Otherwise the variables defined default value is loaded there.
    /// or nothing.
    pub fn get_variable<T: IntoSpvType>(&mut self, name: &str, default_value: T) -> DataId<T>{

        
        let shash = simple_hash(name);
        let spvtype = T::into_spv_type();

        if let Some(param) = self.interface.get_parameter::<T>(shash, &spvtype){
            //inline load procedure
            let did = self.builder.composite_extract(param.spirv_type_id, None, param.composite_id, [1]).unwrap();
            
            DataId{
                id: did,
                ty: PhantomData
            }
        }else{
            #[cfg(feature = "logging")]
            log::warn!("Could not find variable \"{}\" in function interface, falling back to constant", name);
            panic!("Shite");
            //Constant{value: default_value}.serialize(self, ())
        }
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


