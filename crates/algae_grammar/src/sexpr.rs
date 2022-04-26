use std::fmt::{Debug, Formatter};

use proc_macro_error::{abort_call_site, emit_call_site_error, abort};
use quote::{quote, ToTokens};
use syn::{parse::{Parse, ParseStream}, Result, Token, Ident, Type, Expr, parenthesized, token::Paren, Lit};

use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::Span;



pub enum Operand{
    Value(Lit),
    Expression(Box<SExpr>)
}

impl Operand{
    fn get_span(&self) -> Span{
        match self{
            Operand::Value(l) => l.span(),
            Operand::Expression(ex) => ex.operation.span()
        }
    }

    fn is_numeric_literal(&self) -> bool{
        match self{
            Operand::Value(Lit::Float(_) | Lit::Int(_)) =>  true,
            _ => false
        }
    }

    fn unwrap_literal(&self) -> Lit{
        match self{
            Operand::Value(l) => l.clone(),
            _ => panic!("Operand was no literal!"),
        }
    }
}

impl Debug for Operand{
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result{
        match self{
            Operand::Value(_) => write!(fmt, "val"),
            Operand::Expression(se) => se.fmt(fmt)
        }
    }
}

impl ToTokens for Operand{
    fn to_tokens(&self, tokens: &mut TokenStream2){
        match self{
            Operand::Value(lit) => {
                //Literals are per definition constants
                tokens.extend(quote!(
                    Box::new(algae::operations::Constant{value: #lit})
                ))
            },
            Operand::Expression(exp) => exp.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
pub struct SExpr{
    operation: Ident,
    operands: Vec<Operand>,
}

//Turns any expression into the final algae code
impl ToTokens for SExpr{
    fn to_tokens(&self, tokens: &mut TokenStream2){
        let SExpr { operation, operands } = self;

        //TODO currently just assuming that we have the correct operands.
        //     but we should actually type check, and check that they are "there".
        
        let sub_stream = match operation.to_string().as_str(){
            "add" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Addition{
                            a: #ela,
                            b: #elb
                        }
                    )
                )
            }
            "sub" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Subtraction{
                            minuent: #ela,
                            subtrahend: #elb
                        }
                    )
                )
            }
            "mul" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Multiplication{
                            a: #ela,
                            b: #elb
                        }
                    )
                )
            }
            "div" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Division{
                            dividend: #ela,
                            divisor: #elb
                        }
                    )
                )
            }
            "cross" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Cross{
                            a: #ela,
                            b: #elb
                        }
                    )
                )
            }
            "vec_select" => {
                let ela = &operands[0];
                let elb = &operands[1];

                if !ela.is_numeric_literal(){
                    abort!(ela.get_span(), "first element must be element to select")
                }
                let ela = ela.unwrap_literal();
                
                quote!(
                    Box::new(
                        algae::operations::VecSelectElement{
                            element: #ela,
                            inner: #elb
                        }
                    )
                )
            }
            "abs" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Abs{
                            inner: #ela
                        }
                    )
                )
            }
            "sin" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Sine{
                            inner: #ela
                        }
                    )
                )
            }
            "cos" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Cosine{
                            inner: #ela
                        }
                    )
                )
            }
            "tan" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Tangent{
                            inner: #ela
                        }
                    )
                )
            }
            "length" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Length{
                            inner: #ela
                        }
                    )
                )
            }
            "normalize" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Normalize{
                            inner: #ela
                        }
                    )
                )
            }
            "square" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Square{
                            inner: #ela
                        }
                    )
                )
            }
            "sqrt" => {
                let ela = &operands[0];
                
                quote!(
                    Box::new(
                        algae::operations::Sqrt{
                            inner: #ela
                        }
                    )
                )
            }
            "min" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Min{
                            a: #ela,
                            b: #elb
                        }
                    )
                )
            }
            "max" => {
                let ela = &operands[0];
                let elb = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Max{
                            a: #ela,
                            b: #elb
                        }
                    )
                )
            }
            //vector constructors
            "vec2" => {
                //vector constructor from literals (usually)
                let x = &operands[0];
                let y = &operands[1];
                
                if !x.is_numeric_literal(){
                    abort!(x.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !y.is_numeric_literal(){
                    abort!(y.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }

                let x = x.unwrap_literal();
                let y = y.unwrap_literal();

                match (&x, &y) {
                    (Lit::Float(_), Lit::Float(_)) => {
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::Vec2::new(#x, #y)
                                }
                            )
                        )
                    }
                    (Lit::Int(_), Lit::Int(_)) => {
                        //Assuming i32 integer
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::IVec2::new(#x, #y)
                                }
                            )
                        )
                    }
                    _ => abort!(self.operation.span(), "Operands are not of the same numeric type")
                }
            }
            "vec3" => {
                //vector constructor from literals (usually)
                let x = &operands[0];
                let y = &operands[1];
                let z = &operands[2];
                
                if !x.is_numeric_literal(){
                    abort!(x.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !y.is_numeric_literal(){
                    abort!(y.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !z.is_numeric_literal(){
                    abort!(z.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }

                let x = x.unwrap_literal();
                let y = y.unwrap_literal();
                let z = z.unwrap_literal();

                match (&x, &y, &z) {
                    (Lit::Float(_), Lit::Float(_), Lit::Float(_)) => {
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::Vec3::new(#x, #y, #z)
                                }
                            )
                        )
                    }
                    (Lit::Int(_), Lit::Int(_), Lit::Int(_)) => {
                        //Assuming i32 integer
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::IVec3::new(#x, #y, #z)
                                }
                            )
                        )
                    }
                    _ => abort!(self.operation.span(), "Operands are not of the same numeric type")
                }
            }
            "vec4" => {
                //vector constructor from literals (usually)
                let x = &operands[0];
                let y = &operands[1];
                let z = &operands[2];
                let w = &operands[3];
                
                if !x.is_numeric_literal(){
                    abort!(x.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !y.is_numeric_literal(){
                    abort!(y.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !z.is_numeric_literal(){
                    abort!(z.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }
                if !w.is_numeric_literal(){
                    abort!(w.get_span(), "vector constructor argument must be numeric literal like 1 or 42.420");
                }

                let x = x.unwrap_literal();
                let y = y.unwrap_literal();
                let z = z.unwrap_literal();
                let w = w.unwrap_literal();

                match (&x, &y, &z, &w) {
                    (Lit::Float(_), Lit::Float(_), Lit::Float(_), Lit::Float(_)) => {
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::Vec4::new(#x, #y, #z, #w)
                                }
                            )
                        )
                    }
                    (Lit::Int(_), Lit::Int(_), Lit::Int(_), Lit::Int(_)) => {
                        //Assuming i32 integer
                        quote!(
                            Box::new(
                                algae::operations::Constant{
                                    value: algae::glam::IVec4::new(#x, #y, #z, #w)
                                }
                            )
                        )
                    }
                    _ => abort!(self.operation.span(), "Operands are not of the same numeric type")
                }
            }
            "var" => {
                let varname = &operands[0];
                let varname = varname.unwrap_literal();
                let default_value = &operands[1];
                quote!(
                    Box::new(
                        algae::operations::Variable::new(#varname, #default_value)
                    )
                )
            }

            _ => {abort!(operation.span(), "Unknown operation {}", operation)}
        };
        
        tokens.extend(sub_stream);
    }
}

#[derive(Debug)]
pub enum BaseType{
    Float,
    Int,
    Bool,
    Undecideable,
    None
}

impl Parse for SExpr{
    fn parse(stream: ParseStream) -> Result<Self>{
        //parses the sexpression. We assume that each expression is in braces
        // within each brace the first parameter is the operation, everything else are operators.
        //
        // operations and operators are split by whitespace. Each operator is either a consant, or an expression.

        if stream.is_empty(){
            abort_call_site!("Tried parsing empty expression!");
        }

        //Assume we are in () braces
        let content;
        parenthesized!(content in stream);

        //Parse operation
        let operation: Ident = content.parse().unwrap();

        let mut operands = Vec::with_capacity(2); //usually two

        //parse all operations, possibly recursing.
        while !content.is_empty(){
            //check if the next character is a brace, in that case the operator is a expression.
            
            let operand = if content.peek(Paren){
                let expr = SExpr::parse(&content).unwrap();
                Operand::Expression(Box::new(expr))
            }else if content.peek(Lit){
                let val: Lit = content.parse().unwrap();
                Operand::Value(val)
            }else{
                break;
            };

            operands.push(operand);
        }

        //TODO do type checking here..

        Ok(SExpr{
            operation,
            operands,
        })
    }
}
