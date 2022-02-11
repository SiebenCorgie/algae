use rspirv::dr::Operand;

use crate::{DataId, Operation, SerializerExt, BoxOperation};


use super::vector::VectorN;


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
        DataId::from(serializer.f_sub(t_f32, None, ra.id, rb.id).unwrap())
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
        DataId::from(serializer.f_mul(t_f32, None, ra.id, rb.id).unwrap())
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
        DataId::from(serializer.f_mul(t_f32, None, ra.id, ra.id).unwrap())
    }
}


impl<const N: usize, I> Operation for Square<I, VectorN<N>>{
    type Input = I;
    type Output = DataId<VectorN<N>>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
        let ra = self.inner.serialize(serializer, input);
        let tvec = serializer.type_vec_f32(N);
        DataId::from(serializer.f_mul(tvec, None, ra.id, ra.id).unwrap())
    }
}


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
        let ext_instset_id = serializer.ext_inst_import("GLSL.std.450");
        let tf32 = serializer.type_f32();
        //now call its sqrt function with out result
        DataId::from(serializer.ext_inst(tf32, None, ext_instset_id, 31, [Operand::IdRef(result.id)]).unwrap())
    }
}
