use std::fmt::Display;

use crate::{Operation, AbstractRegister, Registar, Serialzer};


///Pixel location at which the shader is run. Gets set per invocation.
pub struct PixelLocation;

///Euclidean length of a vector
pub struct Length<Input, SubOutput>{
    value: Box<dyn Operation<Input = Input, Output = SubOutput>>
}

impl<Input, SubOutput> Length<Input, SubOutput>{
    pub fn new(value: Box<dyn Operation<Input = Input, Output = SubOutput>>) -> Self{
        Length{
            value
        }
    }
}

impl<Input, SubOutput> Operation for Length<Input, SubOutput>{
    type Input = Input;
    type Output = f32;

    fn serialize(&self, serializer: &mut Serialzer, registar: &mut Registar, input_register: AbstractRegister<Self::Input>) -> AbstractRegister<Self::Output> {
        let out_reg = registar.alloc();

        let result = self.value.serialize(serializer, registar, input_register);

        serializer.append(&format!("
{out_reg} = Length({result})
"));

        out_reg
    }
}


///Simple subtraction of two value
pub struct Subtraction<Input, SubOutput>{
    minuent: Box<dyn Operation<Input = Input, Output = SubOutput>>,
    subtrahend: Box<dyn Operation<Input = Input, Output = SubOutput>>,
}

impl<Input, SubOutput> Subtraction<Input, SubOutput>{
    pub fn new(
        minuent: Box<dyn Operation<Input = Input, Output = SubOutput>>,
        subtrahend: Box<dyn Operation<Input = Input, Output = SubOutput>>
    ) -> Self{
        Subtraction{
            minuent,
            subtrahend
        }
    }
}

impl<Input: Clone, SubOutput> Operation for Subtraction<Input, SubOutput>{
    type Input = Input;
    type Output = SubOutput;

    fn serialize(&self, serializer: &mut Serialzer, registar: &mut Registar, input_register: AbstractRegister<Self::Input>) -> AbstractRegister<Self::Output> {
        let outreg = registar.alloc();
        let minuent = self.minuent.serialize(serializer, registar, input_register.clone());
        let subtrahend = self.subtrahend.serialize(serializer, registar, input_register.clone());

        serializer.append(&format!("
{outreg} = Sub {minuent} {subtrahend}
"));

        outreg
    }
}



///A variable of type `T` that is loaded from a shader local buffer at runtime
pub struct Variable<T>{
    ///Default value of this variable used when not set otherwise
    pub default_value: T,
    pub name: String
}

impl<T> Variable<T>{
    pub fn new(name: &str, value: T) -> Self{
        Variable{
            name: String::from(name),
            default_value: value
        }
    }
}

impl<Output> Operation for Variable<Output>{
    type Input = ();
    type Output = Output;
    fn serialize(&self, serializer: &mut Serialzer, registar: &mut Registar, input_register: AbstractRegister<Self::Input>) -> AbstractRegister<Self::Output> {
        let varreg = registar.alloc();

        serializer.append(&format!("
{} = Load({})
", varreg, self.name));
        varreg
    }
}




///A constant of type `T`
pub struct Constant<T>{
    pub value: T
}

impl<T> Constant<T>{
    pub fn new(value: T) -> Self{
        Constant{
            value
        }
    }
}

impl<Output: Display> Operation for Constant<Output>{
    type Input = ();
    type Output = Output;
    fn serialize(&self, serializer: &mut Serialzer, registar: &mut Registar, _input_register: AbstractRegister<Self::Input>) -> AbstractRegister<Self::Output> {
        let varreg = registar.alloc();

        serializer.append(&format!("
{} = Const({})
", varreg, self.value));
        varreg
    }
}
