use rspirv::{dr::Operand, spirv::Word};

use crate::{Operation, DataId, SerializerExt, Serializer};

/// N-component long float vector
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct VectorN<const N: usize>;
impl<const N: usize> VectorN<N>{
    fn spirv_type(ser: &mut Serializer) -> Word{
        let tf32 = ser.type_f32();
        ser.type_vector(tf32, N as u32)
    }
}

pub struct Length<const N: usize, I>{
    pub inner: Box<dyn Operation<Input = I, Output = DataId<VectorN<N>>>>
}


impl<const N: usize, I> Operation for Length<N, I>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {
        //Uses the extended instructionset to get the length of an vector
        let res = self.inner.serialize(serializer, input);
        let tf32 = serializer.type_f32();

        //Load instructionset
        let ext_instset_id = serializer.ext_inst_import("GLSL.std.450");
        //Call
        DataId::from(serializer.ext_inst(tf32, None, ext_instset_id, 66, [Operand::IdRef(res.id)]).unwrap())
    }
}

///Selects the element `S` of the vector with size N.
///
/// Note that `S<N` must hold true.
pub struct VecSelectElement<const S: usize, const N: usize, I>{
    pub inner: Box<dyn Operation<Input = I, Output = DataId<VectorN<N>>>>,
}

impl<const S: usize, const N: usize, I> Operation for VecSelectElement<S, N, I>{
    type Input = I;
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut crate::Serializer, input: Self::Input) -> Self::Output {

        assert!(S < N, "Tried to select element {}, but vector is of length {}", S, N);
        
        let tyf32 = serializer.type_f32();
        
        let vector_return = self.inner.serialize(serializer, input);
        
        DataId::from(serializer.vector_extract_dynamic(tyf32, None, vector_return.id, S as u32).unwrap())
    }
}
