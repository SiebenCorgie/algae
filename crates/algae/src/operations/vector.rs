use glam::{Vec2, Vec3, Vec4};
use rspirv::dr::Operand;

use crate::{
    operations::{Abs, Max, Min},
    spv_fi::IntoSpvType,
    BoxOperation, DataId, Operation,
};

///Normalizes the `inner` vector. I.e. makes it the length 1.
pub struct Normalize<V, I> {
    pub inner: Box<dyn Operation<Input = I, Output = DataId<V>>>,
}

macro_rules! impl_normalize {
    ($vecty:ty) => {
        impl<I> Operation for Normalize<$vecty, I> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let res = self.inner.serialize(serializer, input);
                let tv = <$vecty>::spirv_type_id(serializer).unwrap();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(tv, None, ext_instset_id, 69, [Operand::IdRef(res.id)])
                        .unwrap(),
                )
            }
        }
    };
}

impl_normalize!(Vec2);
impl_normalize!(Vec3);
impl_normalize!(Vec4);

///Returns the euclidian length of a vector `V`. For float vectors up to 4 elements spirv's extended instruction set can be used.
/// otherwise a fallback based in [fast inverse square-root](https://en.wikipedia.org/wiki/Fast_inverse_square_root) might be used.
pub struct Length<V, I> {
    pub inner: Box<dyn Operation<Input = I, Output = DataId<V>>>,
}

macro_rules! impl_length {
    ($vecty:ty) => {
        impl<I> Operation for Length<$vecty, I> {
            type Input = I;
            type Output = DataId<f32>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let res = self.inner.serialize(serializer, input);
                let tf32 = f32::spirv_type_id(serializer).unwrap();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(tf32, None, ext_instset_id, 66, [Operand::IdRef(res.id)])
                        .unwrap(),
                )
            }
        }
    };
}

impl_length!(Vec2);
impl_length!(Vec3);
impl_length!(Vec4);

///Returns the [cross product](https://en.wikipedia.org/wiki/Cross_product) between two vectors of the same type.
pub struct Cross<V, I> {
    pub a: BoxOperation<I, V>,
    pub b: BoxOperation<I, V>,
}

macro_rules! impl_cross {
    ($vecty:ty) => {
        impl<I: Clone> Operation for Cross<$vecty, I> {
            type Input = I;
            type Output = DataId<f32>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let ra = self.a.serialize(serializer, input.clone());
                let rb = self.b.serialize(serializer, input);
                let tvec = <$vecty>::spirv_type_id(serializer).unwrap();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(
                            tvec,
                            None,
                            ext_instset_id,
                            68,
                            [Operand::IdRef(ra.id), Operand::IdRef(rb.id)],
                        )
                        .unwrap(),
                )
            }
        }
    };
}

impl_cross!(Vec2);
impl_cross!(Vec3);
impl_cross!(Vec4);

///Selects the `element` of the vector.
///
/// Note that `element` must be within the number of elements of the concrete vector `V`
pub struct VecSelectElement<V, I> {
    pub element: u32,
    pub inner: Box<dyn Operation<Input = I, Output = DataId<V>>>,
}

macro_rules! impl_vec_select {
    ($vecty:ty, $num_comp:expr) => {
        impl<I> Operation for VecSelectElement<$vecty, I> {
            type Input = I;
            type Output = DataId<f32>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                assert!(
                    self.element < $num_comp,
                    "Tried to select element {}, but vector is of length {}",
                    self.element,
                    $num_comp
                );
                let tyf32 = f32::spirv_type_id(serializer).unwrap();

                let vector_return = self.inner.serialize(serializer, input);

                DataId::from(
                    serializer
                        .builder_mut()
                        .composite_extract(tyf32, None, vector_return.id, [self.element])
                        .unwrap(),
                )
            }
        }
    };
}

impl_vec_select!(Vec2, 2);
impl_vec_select!(Vec3, 3);
impl_vec_select!(Vec4, 4);

macro_rules! impl_vec_fabs {
    ($vecty:ty) => {
        ///Uses the extended instruction set to implement abs via the `FAbs` instruction for [$vecty](glam::<$vecty>).
        impl<I> Operation for Abs<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let res = self.inner.serialize(serializer, input);
                let tv = <$vecty>::spirv_type_id(serializer).unwrap();
                //Load extended instruction set
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

                //Now execute the sinus function
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(tv, None, ext_instset_id, 4, [Operand::IdRef(res.id)])
                        .unwrap(),
                )
            }
        }
    };
}

impl_vec_fabs!(Vec2);
impl_vec_fabs!(Vec3);
impl_vec_fabs!(Vec4);

macro_rules! impl_maxf {
    ($vecty:ty) => {
        impl<I: Clone> Operation for Max<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let ra = self.a.serialize(serializer, input.clone());
                let rb = self.b.serialize(serializer, input);
                let tvec = <$vecty>::spirv_type_id(serializer).unwrap();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(
                            tvec,
                            None,
                            ext_instset_id,
                            40,
                            [Operand::IdRef(ra.id), Operand::IdRef(rb.id)],
                        )
                        .unwrap(),
                )
            }
        }
    };
}

impl_maxf!(Vec2);
impl_maxf!(Vec3);
impl_maxf!(Vec4);

macro_rules! impl_minf {
    ($vecty:ty) => {
        impl<I: Clone> Operation for Min<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                //Uses the extended instructionset to get the length of an vector
                let ra = self.a.serialize(serializer, input.clone());
                let rb = self.b.serialize(serializer, input);
                let tvec = <$vecty>::spirv_type_id(serializer).unwrap();

                //Load instructionset
                let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
                //Call
                DataId::from(
                    serializer
                        .builder_mut()
                        .ext_inst(
                            tvec,
                            None,
                            ext_instset_id,
                            37,
                            [Operand::IdRef(ra.id), Operand::IdRef(rb.id)],
                        )
                        .unwrap(),
                )
            }
        }
    };
}

impl_minf!(Vec2);
impl_minf!(Vec3);
impl_minf!(Vec4);
