use rspirv::dr::Operand;
use glam::{Vec2, Vec3, Vec4};

use crate::{DataId, Operation, BoxOperation};

pub struct Subtraction<I, O>{
    pub minuent: BoxOperation<I, O>,
    pub subtrahend: BoxOperation<I, O>,
}

impl<I: Clone> Operation for Subtraction<I, f32>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
        let ra = self.minuent.serialize(serializer, input.clone());
        let rb = self.subtrahend.serialize(serializer, input);
        let t_f32 = serializer.type_f32();
        DataId::from(serializer.builder_mut().f_sub(t_f32, None, ra.id, rb.id).unwrap())
    }
}


pub struct Multiply<I, O>{
    pub a: BoxOperation<I, O>,
    pub b: BoxOperation<I, O>,
}

impl<I: Clone> Operation for Multiply<I, f32>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
        let ra = self.a.serialize(serializer, input.clone());
        let rb = self.b.serialize(serializer, input);
        let t_f32 = serializer.type_f32();
        DataId::from(serializer.builder_mut().f_mul(t_f32, None, ra.id, rb.id).unwrap())
    }
}




pub struct Square<I, O>{
    pub inner: BoxOperation<I, O>
}

impl<I> Operation for Square<I, f32>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
        let ra = self.inner.serialize(serializer, input);
        let t_f32 = serializer.type_f32();
        DataId::from(serializer.builder_mut().f_mul(t_f32, None, ra.id, ra.id).unwrap())
    }
}

macro_rules! impl_sq_fvec {
    ($vecty:ty, $nc:expr) => {        
        impl<I> Operation for Square<I, $vecty>{
            type Input = I;
            type Output = DataId<$vecty>;

            fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
                let ra = self.inner.serialize(serializer, input);
                let tvec = serializer.type_vec_f32($nc);
                DataId::from(serializer.builder_mut().f_mul(tvec, None, ra.id, ra.id).unwrap())
            }
        }
    }
}

impl_sq_fvec!(Vec2, 2);
impl_sq_fvec!(Vec3, 3);
impl_sq_fvec!(Vec4, 4);


///Uses the extende instructionset of spriv's GL460 profile for a sqrt operation
pub struct Sqrt<I>{
    inner: BoxOperation<I, f32>,
}

impl<I> Operation for Sqrt<I>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output{

        let result = self.inner.serialize(serializer, input);

        //make sure the instructionset is loaded
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");
        let tf32 = serializer.type_f32();
        //now call its sqrt function with out result
        DataId::from(serializer.builder_mut().ext_inst(tf32, None, ext_instset_id, 31, [Operand::IdRef(result.id)]).unwrap())
    }
}
