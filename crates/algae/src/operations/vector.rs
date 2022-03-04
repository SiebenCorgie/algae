use rspirv::dr::Operand;
use glam::{Vec2, Vec3, Vec4};

use crate::{Operation, DataId};

///Returns the euclidian length of a vector `V`. For float vectors up to 4 elements spirv's extended instruction set can be used.
/// otherwise a fallback based in [fast inverse square-root](https://en.wikipedia.org/wiki/Fast_inverse_square_root) might be used.
pub struct Length<V, I>{
    pub inner: Box<dyn Operation<Input = I, Output = DataId<V>>>
}

macro_rules! impl_length {
    ($vecty:ty) => {
        impl<I> Operation for Length<$vecty, I>{
            type Input = I;
            type Output = DataId<f32>;

            fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let res = self.inner.serialize(serializer, input);
                let tf32 = serializer.type_f32();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(serializer.builder_mut().ext_inst(tf32, None, ext_instset_id, 66, [Operand::IdRef(res.id)]).unwrap())
            }
        }
    }
}

impl_length!(Vec2);
impl_length!(Vec3);
impl_length!(Vec4);

///Selects the `element` of the vector.
///
/// Note that `element` must be within the number of elements of the concrete vector `V`
pub struct VecSelectElement<V, I>{
    pub element: u32,
    pub inner: Box<dyn Operation<Input = I, Output = DataId<V>>>,
}


macro_rules! impl_vec_select {
    ($vecty:ty, $num_comp:expr) => {
        impl<I> Operation for VecSelectElement<$vecty, I>{
            type Input = I;
            type Output = DataId<f32>;

            fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {

                assert!(self.element < $num_comp, "Tried to select element {}, but vector is of length {}", self.element, $num_comp);
                let tyf32 = serializer.type_f32();
                
                let vector_return = self.inner.serialize(serializer, input);
                
                DataId::from(serializer.builder_mut().vector_extract_dynamic(tyf32, None, vector_return.id, self.element).unwrap())
            }
        }
    }
}

impl_vec_select!(Vec2, 2);
impl_vec_select!(Vec3, 3);
impl_vec_select!(Vec4, 4);
