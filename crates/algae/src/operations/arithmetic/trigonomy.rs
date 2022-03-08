use rspirv::dr::Operand;

use crate::{spv_fi::IntoSpvType, BoxOperation, DataId, Operation, Serializer};

///Calculates the sine of some value.
pub struct Sine<I> {
    pub inner: BoxOperation<I, f32>,
}

impl<I> Operation for Sine<I> {
    type Input = I;
    type Output = DataId<f32>;
    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        //Uses the extended instructionset to get the length of an vector
        let res = self.inner.serialize(serializer, input);
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(tf32, None, ext_instset_id, 13, [Operand::IdRef(res.id)])
                .unwrap(),
        )
    }
}

///Calculates the cosine of some value.
pub struct Cosine<I> {
    pub inner: BoxOperation<I, f32>,
}

impl<I> Operation for Cosine<I> {
    type Input = I;
    type Output = DataId<f32>;
    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        //Uses the extended instructionset to get the length of an vector
        let res = self.inner.serialize(serializer, input);
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(tf32, None, ext_instset_id, 14, [Operand::IdRef(res.id)])
                .unwrap(),
        )
    }
}

///Calculates the Tangent of some value.
pub struct Tangent<I> {
    pub inner: BoxOperation<I, f32>,
}

impl<I> Operation for Tangent<I> {
    type Input = I;
    type Output = DataId<f32>;
    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        //Uses the extended instructionset to get the length of an vector
        let res = self.inner.serialize(serializer, input);
        let tf32 = f32::spirv_type_id(serializer).unwrap();
        //Load extended instruction set
        let ext_instset_id = serializer.builder_mut().ext_inst_import("GLSL.std.450");

        //Now execute the sinus function
        DataId::from(
            serializer
                .builder_mut()
                .ext_inst(tf32, None, ext_instset_id, 15, [Operand::IdRef(res.id)])
                .unwrap(),
        )
    }
}
