use glam::{Vec2, Vec3, Vec4};
use rspirv::dr::Operand;
use std::marker::PhantomData;

use super::{Abs, Max, Min};
use crate::operations::{
    Addition, Division, Multiplication, Sqrt, Square, Subtraction, VecSelectElement,
};
use crate::spv_fi::IntoSpvType;
use crate::DataId;
use crate::Operation;

impl<I: Clone> Operation for Addition<I, f32> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let ra = self.a.serialize(serializer, input.clone());
        let rb = self.b.serialize(serializer, input);
        let t_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .f_add(t_f32, None, ra.id, rb.id)
                .unwrap(),
        )
    }
}
macro_rules! vec_op_add {
    ($vecty:ty, $nel:expr) => {
        //In general this operations work by extracing each component of each type, doing the `op`
        //on each pair, and then assembling the a new variable of the type.

        impl<I: Clone> Operation for Addition<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                let t_f32 = f32::spirv_type_id(serializer).unwrap();
                let t_vec = <$vecty>::spirv_type_id(serializer).unwrap();

                //get inner results
                let ra = self.a.serialize(serializer, input.clone());
                let rb = self.b.serialize(serializer, input);

                //load each element-pair, add them and collet the element ids
                let mut result_ids = [0u32; $nel];
                for idx in 0..$nel {
                    let extracted_a = VecSelectElement {
                        element: idx,
                        inner: Box::new(ra),
                    }
                    .serialize(serializer, ());
                    let extracted_b = VecSelectElement {
                        element: idx,
                        inner: Box::new(rb),
                    }
                    .serialize(serializer, ());

                    //issue add and safe result at correct place
                    result_ids[idx as usize] = serializer
                        .builder_mut()
                        .f_add(t_f32, None, extracted_a.id, extracted_b.id)
                        .unwrap();
                }

                //now build new vector and return
                let vector_id = serializer
                    .builder_mut()
                    .composite_construct(t_vec, None, result_ids)
                    .unwrap();

                DataId {
                    id: vector_id,
                    ty: PhantomData,
                }
            }
        }
    };
}
vec_op_add!(Vec2, 2);
vec_op_add!(Vec3, 3);
vec_op_add!(Vec4, 4);

impl<I: Clone> Operation for Subtraction<I, f32> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let ra = self.minuent.serialize(serializer, input.clone());
        let rb = self.subtrahend.serialize(serializer, input);
        let t_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .f_sub(t_f32, None, ra.id, rb.id)
                .unwrap(),
        )
    }
}
macro_rules! vec_op_sub {
    ($vecty:ty, $nel:expr) => {
        //In general this operations work by extracing each component of each type, doing the `op`
        //on each pair, and then assembling the a new variable of the type.

        impl<I: Clone> Operation for Subtraction<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                let t_f32 = f32::spirv_type_id(serializer).unwrap();
                let t_vec = <$vecty>::spirv_type_id(serializer).unwrap();

                //get inner results
                let ra = self.minuent.serialize(serializer, input.clone());
                let rb = self.subtrahend.serialize(serializer, input);

                //load each element-pair, add them and collet the element ids
                let mut result_ids = [0u32; $nel];
                for idx in 0..$nel {
                    let extracted_a = VecSelectElement {
                        element: idx,
                        inner: Box::new(ra),
                    }
                    .serialize(serializer, ());
                    let extracted_b = VecSelectElement {
                        element: idx,
                        inner: Box::new(rb),
                    }
                    .serialize(serializer, ());

                    //issue add and safe result at correct place
                    result_ids[idx as usize] = serializer
                        .builder_mut()
                        .f_sub(t_f32, None, extracted_a.id, extracted_b.id)
                        .unwrap();
                }

                //now build new vector and return
                let vector_id = serializer
                    .builder_mut()
                    .composite_construct(t_vec, None, result_ids)
                    .unwrap();

                DataId {
                    id: vector_id,
                    ty: PhantomData,
                }
            }
        }
    };
}
vec_op_sub!(Vec2, 2);
vec_op_sub!(Vec3, 3);
vec_op_sub!(Vec4, 4);

impl<I: Clone> Operation for Multiplication<I, f32> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let ra = self.a.serialize(serializer, input.clone());
        let rb = self.b.serialize(serializer, input);
        let t_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .f_mul(t_f32, None, ra.id, rb.id)
                .unwrap(),
        )
    }
}
macro_rules! vec_op_mul {
    ($vecty:ty, $nel:expr) => {
        //In general this operations work by extracing each component of each type, doing the `op`
        //on each pair, and then assembling the a new variable of the type.

        impl<I: Clone> Operation for Multiplication<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                let t_f32 = f32::spirv_type_id(serializer).unwrap();
                let t_vec = <$vecty>::spirv_type_id(serializer).unwrap();

                //get inner results
                let ra = self.a.serialize(serializer, input.clone());
                let rb = self.b.serialize(serializer, input);

                //load each element-pair, add them and collet the element ids
                let mut result_ids = [0u32; $nel];
                for idx in 0..$nel {
                    let extracted_a = VecSelectElement {
                        element: idx,
                        inner: Box::new(ra),
                    }
                    .serialize(serializer, ());
                    let extracted_b = VecSelectElement {
                        element: idx,
                        inner: Box::new(rb),
                    }
                    .serialize(serializer, ());

                    //issue add and safe result at correct place
                    result_ids[idx as usize] = serializer
                        .builder_mut()
                        .f_mul(t_f32, None, extracted_a.id, extracted_b.id)
                        .unwrap();
                }

                //now build new vector and return
                let vector_id = serializer
                    .builder_mut()
                    .composite_construct(t_vec, None, result_ids)
                    .unwrap();

                DataId {
                    id: vector_id,
                    ty: PhantomData,
                }
            }
        }
    };
}
vec_op_mul!(Vec2, 2);
vec_op_mul!(Vec3, 3);
vec_op_mul!(Vec4, 4);

impl<I: Clone> Operation for Division<I, f32> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let ra = self.dividend.serialize(serializer, input.clone());
        let rb = self.divisor.serialize(serializer, input);
        let t_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .f_div(t_f32, None, ra.id, rb.id)
                .unwrap(),
        )
    }
}
macro_rules! vec_op_div {
    ($vecty:ty, $nel:expr) => {
        //In general this operations work by extracing each component of each type, doing the `op`
        //on each pair, and then assembling the a new variable of the type.

        impl<I: Clone> Operation for Division<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                let t_f32 = f32::spirv_type_id(serializer).unwrap();
                let t_vec = <$vecty>::spirv_type_id(serializer).unwrap();

                //get inner results
                let ra = self.dividend.serialize(serializer, input.clone());
                let rb = self.divisor.serialize(serializer, input);

                //load each element-pair, add them and collet the element ids
                let mut result_ids = [0u32; $nel];
                for idx in 0..$nel {
                    let extracted_a = VecSelectElement {
                        element: idx,
                        inner: Box::new(ra),
                    }
                    .serialize(serializer, ());
                    let extracted_b = VecSelectElement {
                        element: idx,
                        inner: Box::new(rb),
                    }
                    .serialize(serializer, ());

                    //issue add and safe result at correct place
                    result_ids[idx as usize] = serializer
                        .builder_mut()
                        .f_div(t_f32, None, extracted_a.id, extracted_b.id)
                        .unwrap();
                }

                //now build new vector and return
                let vector_id = serializer
                    .builder_mut()
                    .composite_construct(t_vec, None, result_ids)
                    .unwrap();

                DataId {
                    id: vector_id,
                    ty: PhantomData,
                }
            }
        }
    };
}
vec_op_div!(Vec2, 2);
vec_op_div!(Vec3, 3);
vec_op_div!(Vec4, 4);

impl<I> Operation for Square<I, f32> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let ra = self.inner.serialize(serializer, input);
        let t_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(
            serializer
                .builder_mut()
                .f_mul(t_f32, None, ra.id, ra.id)
                .unwrap(),
        )
    }
}

macro_rules! impl_sq_fvec {
    ($vecty:ty) => {
        impl<I> Operation for Square<I, $vecty> {
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(
                &mut self,
                serializer: &mut crate::Serializer,
                input: Self::Input,
            ) -> Self::Output {
                let ra = self.inner.serialize(serializer, input);
                let tvec = <$vecty>::spirv_type_id(serializer).unwrap();
                DataId::from(
                    serializer
                        .builder_mut()
                        .f_mul(tvec, None, ra.id, ra.id)
                        .unwrap(),
                )
            }
        }
    };
}

impl_sq_fvec!(Vec2);
impl_sq_fvec!(Vec3);
impl_sq_fvec!(Vec4);

///Planket implementation for anything that returns a float
impl<I> Operation for Sqrt<I> {
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(
        &mut self,
        serializer: &mut crate::Serializer,
        input: Self::Input,
    ) -> Self::Output {
        let result = self.inner.serialize(serializer, input);

        //make sure the instructionset is loaded
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //now call its sqrt function with out result
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(tf32, None, ext_instset_id, 31, [Operand::IdRef(result.id)])
                .unwrap(),
        )
    }
}

///Uses the extended instruction set to implement abs via the `FAbs` instruction for floats.
impl<I> Operation for Abs<I, f32> {
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
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(tf32, None, ext_instset_id, 4, [Operand::IdRef(res.id)])
                .unwrap(),
        )
    }
}

impl<I: Clone> Operation for Max<I, f32> {
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
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(
                    tf32,
                    None,
                    ext_instset_id,
                    40,
                    [Operand::IdRef(ra.id), Operand::IdRef(rb.id)],
                )
                .unwrap(),
        )
    }
}

impl<I: Clone> Operation for Min<I, f32> {
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
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(
                    tf32,
                    None,
                    ext_instset_id,
                    37,
                    [Operand::IdRef(ra.id), Operand::IdRef(rb.id)],
                )
                .unwrap(),
        )
    }
}
