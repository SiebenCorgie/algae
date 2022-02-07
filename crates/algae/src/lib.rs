//! # 🦠 Algae
//! 
//! Algae is the algebra layer that can be jit compiled into an existing SpirV module.
//!
//!


pub mod operations;

use std::{marker::PhantomData, fmt::Display};
pub use glam;
use glam::{Vec2, Vec3, Vec4, Quat, Mat3, Mat4};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AbstractRegister<T>{
    slot: usize,
    ty: PhantomData<T>
}

impl<T> Display for AbstractRegister<T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AR({})", self.slot)
    }
}

///Single operation in the formula.
pub trait Operation{
    ///Input signature of this operation
    type Input;
    ///Output signature of this operation
    type Output;

    ///Serializes `Self`, gets provided the register of the input value. Returns the output register.
    fn serialize(&self, serializer: &mut Serialzer, registar: &mut Registar, input_register: AbstractRegister<Self::Input>) -> AbstractRegister<Self::Output>;
}


///An assembled formula. Calculates the output `O` from an input `I`.
pub struct Formula<I, O>{
    root_operation: Box<dyn Operation<Input = I, Output = O>>
}

impl<I, O> Formula<I, O>{
    pub fn new(operation: impl Operation<Input = I, Output = O> + 'static) -> Self{
        Formula{
            root_operation: Box::new(operation)
        }
    }

    pub fn serialize(&self) -> Serialzer{
        let mut ser = Serialzer::new();
        let mut registar = Registar::new();

        let input_register = registar.alloc();
        
        let output_register = self.root_operation.serialize(&mut ser, &mut registar, input_register);
        println!("Output at {output_register}");

        ser
    }
}


pub struct Registar{    
    register: usize
}

impl Registar{
    pub fn new() -> Self{
        Registar{
            register: 0,
        }
    }
    
    pub fn alloc<T>(&mut self) -> AbstractRegister<T>{
        let reg = AbstractRegister{
            slot: self.register,
            ty: PhantomData
        };

        self.register += 1;

        reg
    }
}


pub struct Serialzer{
    pub code: String,
}

impl Serialzer{
    pub fn new() -> Self{
        Serialzer{
            code: String::with_capacity(1024)
        }
    }

    pub fn append(&mut self, code: &str){
        self.code.push_str("\n");
        self.code.push_str(code);
    }
}
