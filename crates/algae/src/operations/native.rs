//! Most basic operations, like constants or variable loading.

use std::{process::Output, marker::PhantomData};

use glam::Vec2;

use crate::{Operation, DataId, Serializer, spv_fi::IntoSpvType};

pub struct Constant<T>{
    pub value: T
}

impl Operation for Constant<f32>{
    type Input = ();
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        let ty_f32 = serializer.type_f32();
        DataId::from(serializer.builder_mut().constant_f32(ty_f32, self.value))
    }
}

impl Operation for Constant<Vec2>{
    type Input = ();
    type Output = DataId<Vec2>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {

        let ty_vec2 = serializer.type_vec_f32(2);
        //Setup float constants
        let cx = Constant{value: self.value.x}.serialize(serializer, ());
        let cy = Constant{value: self.value.y}.serialize(serializer, ());

        //Setup const composite
        DataId::from(serializer.builder_mut().constant_composite(ty_vec2, [cx.id, cy.id]))
    }        
}

impl Operation for Constant<u32>{
    type Input = ();
    type Output = DataId<u32>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        let ty_u32 = serializer.type_u32();
        DataId::from(serializer.builder_mut().constant_u32(ty_u32, self.value))
    }
}

///Data id implements Operation a well, which allows us to use formerly calculated values as input
///for otherwise nested operations.
impl<T: Clone> Operation for DataId<T>{
    type Input = ();
    type Output = DataId<T>;

    fn serialize(&mut self, _serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        self.clone()
    }
}


///Runtime setable variable identified by the given name. Type safety is checked at runtime.
pub struct Variable<T: Sized + 'static>{
    ///Default value of the variable if it is not set at runtime.
    pub default_value: T,
    ///Identifying variable name.
    pub name: String
}

impl<T: Sized + 'static> Variable<T>{
    pub fn new(name: &str, default_value: T) -> Self{
        Variable{
            default_value,
            name: String::from(name)
        }
    }
}

/*
impl<T: IntoSpvType + Clone + Sized + 'static> Operation for Variable<T> where Constant<T>: Operation<Input = (), Output = DataId<T>>{
    type Input = ();
    type Output = DataId<T>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        if let Some(variable_location) = serializer.interface.get_variable(&self.name){
            variable_location
        }else{
            #[cfg(feature = "logging")]
            log::warn!("Could not find variable {} in function interface, falling back to constant", self.name);
            Constant{value: self.default_value.clone()}.serialize(serializer, ())
        }
    }
}
*/

impl Operation for Variable<Vec2>{
    type Input = ();
    type Output = DataId<Vec2>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        serializer.get_variable::<Vec2>(&self.name, self.default_value.clone())
    }
}
