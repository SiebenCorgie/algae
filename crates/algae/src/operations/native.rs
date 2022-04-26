//! Most basic operations, like constants or variable loading, and simple ordering of operations.

use std::{any::Any, marker::PhantomData};

use crate::{spv_fi::IntoSpvType, BoxOperation, DataId, Operation, Serializer};

#[derive(Clone, Copy, Debug)]
pub struct Constant<I, T> {
    pub inty: PhantomData<I>,
    pub value: T,
}

impl<I, T: 'static> Constant<I, T>{
    pub fn new(value: T) -> Self{
        Constant{
            value,
            inty: PhantomData,
        }
    }
}

///Implements Constant for any type that can be also expressed as a SpirvType
impl<I, T> Operation for Constant<I, T>
where
    T: IntoSpvType,
{
    type Input = I;
    type Output = DataId<T>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        self.value.constant_serialize(serializer)
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
pub struct Variable<I, T: Sized + 'static> {
    pub inty: PhantomData<I>,
    ///Default value of the variable if it is not set at runtime.
    pub default_value: Constant<I, T>,
    ///Identifying variable name.
    pub name: String,
}

impl<I, T: Sized + 'static> Variable<I, T> {
    pub fn new(name: &str, default_value: T) -> Self {
        Variable {
            inty: PhantomData,
            default_value: Constant{value: default_value, inty: PhantomData},
            name: String::from(name),
        }
    }
}

impl<I, T> Operation for Variable<I, T>
where
    I: Any + 'static,
    T: IntoSpvType + Clone + 'static,
    Constant<I, T>: Operation<Input = I, Output = DataId<T>>,
{
    type Input = I;
    type Output = DataId<T>;

    fn serialize(&mut self, serializer: &mut Serializer, _input: Self::Input) -> Self::Output {
        serializer.get_variable::<T>(&self.name, self.default_value.value.clone())
    }
}
