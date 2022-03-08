//! Most basic operations, like constants or variable loading, and simple ordering of operations.

use std::marker::PhantomData;

use glam::Vec2;

use crate::{spv_fi::IntoSpvType, BoxOperation, DataId, Operation, Serializer};

pub struct Constant<T> {
    pub value: T,
}

impl Operation for Constant<f32> {
    type Input = ();
    type Output = DataId<f32>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        let ty_f32 = f32::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_f32(ty_f32, self.value))
    }
}

impl Operation for Constant<Vec2> {
    type Input = ();
    type Output = DataId<Vec2>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        let ty_vec2 = Vec2::spirv_type_id(serializer).unwrap();
        //Setup float constants
        let cx = Constant {
            value: self.value.x,
        }
        .serialize(serializer, ());
        let cy = Constant {
            value: self.value.y,
        }
        .serialize(serializer, ());

        //Setup const composite
        DataId::from(
            serializer
                .builder_mut()
                .constant_composite(ty_vec2, [cx.id, cy.id]),
        )
    }
}

impl Operation for Constant<u32> {
    type Input = ();
    type Output = DataId<u32>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        let ty_u32 = u32::spirv_type_id(serializer).unwrap();
        DataId::from(serializer.builder_mut().constant_u32(ty_u32, self.value))
    }
}

///Data id implements Operation a well, which allows us to use formerly calculated values as input
///for otherwise nested operations.
impl<T: Clone> Operation for DataId<T> {
    type Input = ();
    type Output = DataId<T>;

    fn serialize(&mut self, _serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        self.clone()
    }
}

///Allows returning a supplied input which is a `DataId<T>` as output
pub struct ReturnInput<T> {
    d: PhantomData<T>,
}

impl<T> ReturnInput<T> {
    pub fn new() -> Self {
        ReturnInput { d: PhantomData }
    }
}

impl<T> Operation for ReturnInput<T> {
    type Input = DataId<T>;
    type Output = DataId<T>;

    fn serialize(&mut self, _serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        input
    }
}

///Transforms the input parameter `I` based on the provided mapping function. Then calls the inner operation with the transformed input value
pub struct MapInput<I, NI, O> {
    pub inner_operation: BoxOperation<NI, O>,
    pub mapping: Box<dyn Fn(I) -> NI>,
}

impl<I, NI, O> MapInput<I, NI, O> {
    pub fn new(operation: BoxOperation<NI, O>, map: impl Fn(I) -> NI + 'static) -> Self {
        MapInput {
            inner_operation: operation,
            mapping: Box::new(map),
        }
    }
}

impl<I, NI, O> Operation for MapInput<I, NI, O> {
    type Input = I;
    type Output = DataId<O>;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        let mapped_input: NI = (self.mapping)(input);
        self.inner_operation.serialize(serializer, mapped_input)
    }
}

///Runtime setable variable identified by the given name. Type safety is checked at runtime.
pub struct Variable<T: Sized + 'static> {
    ///Default value of the variable if it is not set at runtime.
    pub default_value: T,
    ///Identifying variable name.
    pub name: String,
}

impl<T: Sized + 'static> Variable<T> {
    pub fn new(name: &str, default_value: T) -> Self {
        Variable {
            default_value,
            name: String::from(name),
        }
    }
}

impl<T> Operation for Variable<T>
where
    T: IntoSpvType + Clone + 'static,
    Constant<T>: Operation<Input = (), Output = DataId<T>>,
{
    type Input = ();
    type Output = DataId<T>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        serializer.get_variable::<T>(&self.name, self.default_value.clone())
    }
}

///Links two operation trees. First serializes `first` with `I` as input, then serializes `second` with `first`'s output (`FO`) as input.
///Allows chaining two operation trees.
pub struct Link<I, FO, O> {
    pub first: BoxOperation<I, FO>,
    pub second: BoxOperation<DataId<FO>, O>,
}

impl<I, FO, O> Operation for Link<I, FO, O> {
    type Input = I;
    type Output = DataId<O>;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        let sa = self.first.serialize(serializer, input);
        self.second.serialize(serializer, sa)
    }
}
