#![feature(proc_macro_diagnostic)]


extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse::{Parse, ParseStream}, Result, Type, Ident, Token, Expr};
use quote::{ToTokens, quote};

use proc_macro2::TokenStream as TokenStream2;

use proc_macro_error::proc_macro_error;
use proc_macro_error::abort_call_site;

type ArrowToken = Token!(->);
type ClosureToken = Token!(|);
type CommaToken = Token!(,);
type ColonToken = Token!(:);

#[derive(Clone)]
struct Argument{
    ///Name of the argument
    name: Ident,
    ///value name of the argument
    ty: Type,
}


///Transforms self back into tokens
impl ToTokens for Argument{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let ty = &self.ty;
        tokens.extend(quote!(
            #name : #ty
        ).into_iter())
    }
}


impl Parse for Argument{
    fn parse(stream: ParseStream) -> Result<Self>{
        //assumes that we are right before the parser stream.
        let name: Ident = stream.parse()?;
        let _colon: ColonToken = stream.parse()?;
        let ty: Type = stream.parse()?;

        Ok(Argument{
            name,
            ty
        })
    }
}

struct Signature {
    arguments: Vec<Argument>,
    result_type: Type,
    default_expression: Option<Expr>
}


impl Parse for Signature {
    fn parse(stream: ParseStream) -> Result<Self> {

        
        //assument the synthax |ident0a: ident0b , ident1n: ident1n, .., identna, identnb| -> type
        //parse the ident pairs until the last |, then read out the type information

        let _start: ClosureToken = stream.parse()?;

        let mut arguments: Vec<Argument> = Vec::new();
        //Now parse the stream until we discover the ending token.
        loop{
            if stream.peek(Ident){
                arguments.push(stream.parse()?);
            }

            //Check if the next is comma or end closure token,
            //if is comma, continue searching otherwise break
            if stream.peek(Token![,]){
                let _colon: CommaToken = stream.parse()?;
            }else if stream.peek(Token![|]){
                let _end: ClosureToken = stream.parse()?;
                break;
            }else{
                abort_call_site!("Expected \"|\" or \",\"");
            }
        }


        //now get the arrow as well as the result type
        let _arrow: ArrowToken = stream.parse()?;

        let result_type: Type = stream.parse()?;

        let default_expression = if let Ok(exp) = stream.parse(){
            Some(exp)
        }else{
            None
        };
        
        Ok(Signature{
            arguments,
            result_type,
            default_expression
        })
    }
}

//Wrapper that calculates the id at compile time and emmits the correct assignment
struct Wrapper{
    pub hash: u32,
    pub ty: Type,
    pub assignment: Ident,
}

impl From<Argument> for Wrapper{
    fn from(arg: Argument) -> Self{
        let hash = simple_hash(&arg.name.to_string());
        Wrapper{
            hash,
            ty: arg.ty,
            assignment: arg.name
        }
    }
}

impl ToTokens for Wrapper{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let hash = &self.hash;
        let assignment = &self.assignment;
        let ty = &self.ty;


        let to_append: TokenStream2 = quote!(
            let #assignment: InjectorArg<#ty> = InjectorArg{
                id: #hash,
                value: #assignment
            };
        );
        let stream: TokenStream2 = TokenStream2::from_iter(to_append.into_iter());
        
        tokens.extend(stream);
    }
}

///Simple xor hash based on a name
fn simple_hash(name: &str) -> u32{
    //32bit xor hash atm. We do this by xoring every four bits of "name".
    //if name is not divisible by 32bit we stuff it with zeros
    let mut val: u32 = 0;

    let bytes = name.as_bytes();
    let num_bytes = bytes.len();
    //xor all bytes
    let mut idx = 0;
    while idx < num_bytes{        
        let a0 = bytes[idx] as u32; //Note the first allways succeeds baed on the check above
        val = val ^ (a0 << ((idx % 4) * 8));
        idx += 1;
    }
    
    val
}

struct WrappedFunctionSignature{
    assignment: Ident,
    ty: Type,
}


impl From<&Wrapper> for WrappedFunctionSignature{
    fn from(wrapper: &Wrapper) -> Self{
        WrappedFunctionSignature{
            assignment: wrapper.assignment.clone(),
            ty: wrapper.ty.clone()
        }
    }
}

impl ToTokens for WrappedFunctionSignature{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        
        let assignment = &self.assignment;
        let ty = &self.ty;


        let to_append: TokenStream2 = quote!(
            #assignment : InjectorArg<#ty>
        );
        let stream: TokenStream2 = TokenStream2::from_iter(to_append.into_iter());
        
        tokens.extend(stream);
    }
}

///Wraps each input parameter into a [VariableSignature] where the id is a hash of the parameters name, and the value
///is set at runtime.
///
/// # Example
///
/// ```ignore
/// algae_inject!(|a: f32, b: Vec2, c: i32| -> f32{
///     0.0
/// });
/// //.. at some point in some function algae_inject can be called simply like this
/// let result = algae_inject(3.0, Vec2::ONE, -4);
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn algae_inject(input: TokenStream) -> TokenStream {
    let signature = syn::parse_macro_input!(input as Signature);
    //At this point we got the signature. Now start construction of out injection
    //point

    let function_signature = signature.arguments.clone();
    //Only argument idents
    let argument_names: Vec<Ident>  = signature.arguments.iter().map(|arg| arg.name.clone()).collect();
    //Wrapp all function arguments into the wrapper struct
    let wrapper: Vec<Wrapper> = signature.arguments.into_iter().map(|arg| Wrapper::from(arg)).collect();
    //Builds call signature
    let wrapper_signature: Vec<WrappedFunctionSignature> = wrapper.iter().map(|w| WrappedFunctionSignature::from(w)).collect();
    
    let result_type = signature.result_type;
    let default_expression = &signature.default_expression;
    
    
    //Start out by appending the the definition of the injector arg
    let token_stream = quote!{

        #[repr(C)]
        pub struct InjectorArg<T>{
            pub id: u32,
            pub value: T
        }

        //Anaonym uninlined function
        #[inline(never)]
        fn injector(#(#wrapper_signature),*) -> #result_type{
            //NOTE this is some hacky stuff. Basically we are "using" the wrapped data
            let mut a: u32 = 0;
            
            #(a = a ^ #argument_names.id;)*
            
            #default_expression
        }
        
        //Setup inject function        
        fn algae_inject(#(#function_signature),*) -> #result_type{
            //Expand signature wrapping
            #(#wrapper)*
            let res = injector(#(#argument_names),*);
            res
        }

    };
    


    TokenStream::from(token_stream)
}
