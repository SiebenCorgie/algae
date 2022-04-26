#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use sexpr::SExpr;
use syn::{
    parse::{Parse, ParseStream},
    Expr, Ident, Result, Token, Type,
};

use proc_macro2::TokenStream as TokenStream2;

use proc_macro_error::abort_call_site;
use proc_macro_error::proc_macro_error;

use crate::rexpr::RExpr;

///s-expression parser
mod sexpr;

///rust expression parser
mod rexpr;

///Algae grammar macro. Allows writing human readable math function that are turned into an algae serializeable [Operation](algae::Operation) at runtime.
#[proc_macro]
#[proc_macro_error]
pub fn formula(input: TokenStream) -> TokenStream {

    let token_stream = quote!{
        println!("Hello from macro!");
    };
    TokenStream::from(token_stream)
}


///Turns a [SExpr](sexpr::SExpr) token stream into a (Operation)[algae::Operation]
#[proc_macro]
#[proc_macro_error]
pub fn sexpr(input: TokenStream) -> TokenStream {

    eprintln!("TOKENS: {}", input);
    //parse sexpression
    let expr = syn::parse_macro_input!(input as SExpr);
    
    let token_stream = quote!{
        #expr
    };
    
    TokenStream::from(token_stream)
}

///Turns a [RExpr](rexpr::RExpr) token stream into a (Operation)[algae::Operation]
#[proc_macro]
#[proc_macro_error]
pub fn rexpr(input: TokenStream) -> TokenStream {    
    //parse sexpression
    let expr = syn::parse_macro_input!(input as RExpr);


    let token_stream = quote!{
        {
            #expr
        }
    };
    
    TokenStream::from(token_stream)
}




