//! Most basic operations, like constants or variable loading.

use glam::Vec2;

use crate::{Operation, DataId, SerializerExt};

use super::vector::VectorN;

pub struct Constant<T>{
    pub value: T
}

impl Operation for Constant<f32>{
    type Input = ();
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, _input: Self::Input) -> Self::Output {
        let ty_f32 = serializer.type_f32();
        DataId::from(serializer.constant_f32(ty_f32, self.value))
    }
}

impl Operation for Constant<Vec2>{
    type Input = ();
    type Output = DataId<VectorN<2>>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, _input: Self::Input) -> Self::Output {

        let ty_vec2 = serializer.type_vec_f32(2);
        //Setup float constants
        let cx = Constant{value: self.value.x}.serialize(serializer, ());
        let cy = Constant{value: self.value.y}.serialize(serializer, ());

        //Setup const composite
        DataId::from(serializer.constant_composite(ty_vec2, [cx.id, cy.id]))
    }        
}

impl Operation for Constant<u32>{
    type Input = ();
    type Output = DataId<u32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, _input: Self::Input) -> Self::Output {
        let ty_u32 = serializer.type_u32();
        DataId::from(serializer.constant_u32(ty_u32, self.value))
    }
}

///Data id implements Operation a well, which allows us to use formerly calculated values as input
///for otherwise nested operations.
impl<T: Clone> Operation for DataId<T>{
    type Input = ();
    type Output = DataId<T>;

    fn serialize(&mut self, _serializer: &mut crate::Serializer, _input: Self::Input) -> Self::Output {
        self.clone()
    }
}
