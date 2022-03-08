//! Collects most math related operations.

use crate::BoxOperation;

mod float;
pub use float::*;

mod integer;
pub use integer::*;

///Special trigonometric functions usually only implemented on floats.
pub(crate) mod trigonomy;

///Addition of two values: `result = minuent - subtrahend`
pub struct Addition<I, O> {
    pub a: BoxOperation<I, O>,
    pub b: BoxOperation<I, O>,
}

///Subtraction of two values: `result = minuent - subtrahend`
pub struct Subtraction<I, O> {
    pub minuent: BoxOperation<I, O>,
    pub subtrahend: BoxOperation<I, O>,
}

///Multiplication of two values: `result = a * b`
pub struct Multiplication<I, O> {
    pub a: BoxOperation<I, O>,
    pub b: BoxOperation<I, O>,
}

///Multiplication of two values: `result = dividend / divisor`
pub struct Division<I, O> {
    pub dividend: BoxOperation<I, O>,
    pub divisor: BoxOperation<I, O>,
}

///Squares the inner result: `result = a*a`
pub struct Square<I, O> {
    pub inner: BoxOperation<I, O>,
}

///Returns the square root of the inner result.
pub struct Sqrt<I> {
    pub inner: BoxOperation<I, f32>,
}

///Returns the absolute (positive) value of the inner result.
pub struct Abs<I, O> {
    pub inner: BoxOperation<I, O>,
}

///Returns the bigger of two elements.
pub struct Max<I, O> {
    pub a: BoxOperation<I, O>,
    pub b: BoxOperation<I, O>,
}

///Returns the smaller of two elements.
pub struct Min<I, O> {
    pub a: BoxOperation<I, O>,
    pub b: BoxOperation<I, O>,
}
