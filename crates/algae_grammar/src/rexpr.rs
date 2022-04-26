use std::hash::BuildHasherDefault;

use fxhash::{FxHashSet, FxHashMap};
use proc_macro_error::abort;
use quote::{ToTokens, quote};
use syn::{parse::{Parse, ParseStream}, Token, parenthesized, token::Paren, Lit, Ident};

use proc_macro2::TokenStream as TokenStream2;

///Operand identifiers. Basically a higher level parsed version of operands. If no higher level version is found at
///parsing time the literal or identifier is saved and matched at runtime.
#[derive(Clone, Debug)]
enum SubArg{
    ///If the first *thing* was a identifier. Happens for operation calls and macro/local variables that where declared before with a
    /// "let" binding.
    Ident(Ident),
    ///Some kind of inline literal that later will be a constant.
    Lit(Lit),
    
    ///Usually a literal that just occues /alone/
    LocalVariable{
        ident: Ident,
        var_ty: Ident,
    },
}


#[derive(Clone, Debug)]
pub struct SubOp{
    opident: SubArg,
    operands: Vec<SubOp>,
}

impl SubOp{
    ///Tests all operands of self for type safety. Returns Ok(()) if all are compatible
    fn is_type_compartible(&self) -> syn::Result<()>{
        todo!("Type checking not yet implemented")
    }

    ///Parses this op in some context. The context is needed to check the compile time availability of macro-local variables.
    fn parse_in_context(var_context: &FxHashMap<Ident, Ident>, stream: ParseStream) -> syn::Result<Self>{
        //Check if this token is a ident, or a literal
        let lah = stream.lookahead1();
        let ident = if lah.peek(Lit){
            //we are a single literal, can return early
            let lit = stream.parse::<Lit>()?;
            //if there is a "," token, take it away
            if stream.peek(Token![,]){
                let _coltoken = stream.parse::<Token![,]>()?;
            }
            return Ok(SubOp{
                operands: Vec::new(),
                opident: SubArg::Lit(lit)
            });
        }else if lah.peek(Ident){
            let ident = stream.parse::<Ident>()?;
            ident
        }else{
            //Was neither ident nor literal, shouldn't be 
            return Err(lah.error());
        };
        

        //Check what is next, if it is a "(" we are followed by a list of operands, if it is a "," we are
        // probably a variable. In that case we return and let the parent parse the variable
        if stream.peek(Token![,]){
            let _comtoken = stream.parse::<Token![,]>()?;
            return Ok(SubOp{
                opident: SubArg::Ident(ident),
                operands: Vec::new()
            });
        }
        
        //The operands are within the braces
        let mut operands = Vec::new();
        let content;
        parenthesized!(content in stream); //unwrap the content
        while !content.is_empty(){
            //check if we are at a separating ","
            if content.peek(Token![,]){
                let _ctoken = content.parse::<Token![,]>()?;
            }
            //parse all operands
            let new_op = SubOp::parse_in_context(var_context, &content)?;
            operands.push(new_op);
        }
        
        
        Ok(Self{
            opident: SubArg::Ident(ident),
            operands
        })
    }


    ///Finds all variable literals in a tree and transforms them into macro local variables
    ///for later access
    fn transform_local_variables(&mut self, is_in_var_construction: bool, ctx: &FxHashMap<Ident, Ident>) -> syn::Result<()>{
	match self.opident.clone(){
	    SubArg::Lit(_l) => {
		//we are just some literal, can return
                Ok(())
	    },
	    SubArg::Ident(i) => {
		//we are an identifier.
		//This is a local variable definition if there are no operands, and we are not
		//trying to construct a variable atm.

		if self.operands.len() == 0 && !is_in_var_construction{
                    //we must be a variable. Check if we can find the variable, if so, set it as opident
                    if let Some(local_type) = ctx.get(&i){
                        //Found, therfore overwrite
                        self.opident = SubArg::LocalVariable{
                            ident: i.clone(),
                            var_ty: local_type.clone()
                        };
                    }else{
                        abort!(i.span(), "Is a macro local variable, but has not been created before via a let binding, created variables where: {:#?}", ctx.keys());
                        
                    }
		}
		
		let is_construction = if let "Var" = i.to_string().as_str(){
		    true
		}else{
		    is_in_var_construction //otherwise inherit the one we got
		};

		for op in &mut self.operands{
		    op.transform_local_variables(is_construction, ctx)?;
		}
                Ok(())
	    }
	    SubArg::LocalVariable{..} => {Ok(())}
	}
    }
}


impl ToTokens for SubOp{
    fn to_tokens(&self, tokens: &mut TokenStream2){
	//We expect this subop to be wrapped by the parent into a box if needed.
	//Therefore, when emitting we mostly have to find out own name within algae,
	//and prepare the arguments of this operation. This currently mainly means wrapping
	//everything into a box

	match &self.opident{
	    SubArg::Lit(l) => {
                tokens.extend(quote!(#l))
            }, //literal is easy, we can just emit that one
	    SubArg::LocalVariable{
                ident,
                var_ty
            } => {
                tokens.extend(quote!{
		    algae::operations::AccessResult::<#var_ty>::new(stringify!(#ident)),
	        })
            },
	    SubArg::Ident(ident) => {
		//Got an ident, which is probably a function call.
		//There is one exception though. A ident without arguments. That is usally a macro
		//local variable. We check that first.
		if self.operands.len() == 0{
		    //emit the variable loading
		    tokens.extend(quote!{
			#ident
		    });
		}else{
		    //is not a local variable access, therefore actually parse the name
		    match ident.to_string().as_str(){
			"Add" => {
			    assert!(self.operands.len() == 2, "Expected 2 operands for Add");
			    let a = &self.operands[0];
			    let b = &self.operands[1];
			    tokens.extend(quote! {
				algae::operations::Addition{
				    a: Box::new(#a),
				    b: Box::new(#b)
				}
			    });
			}
			"Sub" => {
			    assert!(self.operands.len() == 2, "Expected 2 operands for Sub");
			    let subtrahend = &self.operands[1];
			    let minuent = &self.operands[0];
			    tokens.extend(quote! {
				algae::operations::Subtraction{
				    minuent: Box::new(#minuent),
				    subtrahend: Box::new(#subtrahend)
				}
			    });
			},
			"Abs" => {
			    assert!(self.operands.len() == 1);
			    let op = &self.operands[0];
			    tokens.extend(quote!{
				algae::operations::Abs{
				    inner: Box::new(#op)
				}
			    });
			},
			"Length" => {
			    assert!(self.operands.len() == 1);
			    let op = &self.operands[0];
			    tokens.extend(quote!{
				algae::operations::Length{
				    inner: Box::new(#op)
				}
			    });
			},
			"Max" => {
			    assert!(self.operands.len() == 2);
			    let l = &self.operands[0];
			    let r = &self.operands[1];

			    tokens.extend(quote!{
				algae::operations::Max{
				    a: Box::new(#l),
				    b: Box::new(#r)
				}
			    });
			}
			"Min" => {
			    assert!(self.operands.len() == 2);
			    let l = &self.operands[0];
			    let r = &self.operands[1];

			    tokens.extend(quote!{
				algae::operations::Min{
				    a: Box::new(#l),
				    b: Box::new(#r)
				}
			    });
			}
			"Vec2" => {
			    //Vector contructor, we assume its just a constant for now
			    assert!(self.operands.len() == 2, "Vec2 constructor expected two operands, but got {}", self.operands.len());
			    let one = &self.operands[0];
			    let two = &self.operands[1];
			    tokens.extend(quote!{
				algae::glam::Vec2::new(#one, #two)
			    })
			}
			"VecSelectElement" => {
			    assert!(self.operands.len() == 2);
			    let vec = &self.operands[0];
			    let element = &self.operands[1];
			    
			    tokens.extend(quote!{
				algae::operations::VecSelectElement{
                                    element: #element,
                                    inner: Box::new(#vec)
                                }
                                    
			    })
			}
                        "Const" => {
                            assert!(self.operands.len() == 1);
                            let inner = &self.operands[0];
                            tokens.extend(quote!{
				algae::operations::Constant::new(
				    #inner
				)
			    });
                        }
			"Var" => {
			    assert!(self.operands.len() == 2);
			    let varname = &self.operands[0];
			    let var_default_value = &self.operands[1];

			    tokens.extend(quote!{
				algae::operations::Variable::new(
				    stringify!(#varname),
				    #var_default_value
				)
			    });
			}
			t => {
			    tokens.extend(quote!(a))
			}
		    }
		}
	    }
	}
    }   
}


///Rust syntax based algae expression.
///
/// # Usage
/// In general a Rust expression looks similar to a function in Rust. Supported statements are `let` statements as well as any recursive declaration of algae operations.
///
/// For instance the following would be a simple Box signed distance field that at first calculates `d` based on a variable "ext" and then uses this "d" parameter
/// to calculate the actual distance:
///
/// ```
/// let d = Abs(Var(p, Vec2(0.0, 0.0))) - Var(ext, Vec2(1.0, 1.0)); //returns a vec2 
/// let res = length(max(d, 0.0 ) ) + min(max(d[0], d[1]), 0.0);
/// return res;
/// ```
///
/// Simple expressions without temporary variables are also possible. In that case the expression is directly translated
/// to the operation. For instance a circle sdf expression might look like this:
///
///```
///  length(Var(coord, Vec2(0.0, 0.0))) - Var(radius, 1.0)
///```
///
/// In general operation calls are characterized by the name of the operation in [algae](algae::operations) where the arguments are either the
/// in the order of the public fields of the operation, or in the order of the `new` constructor.
///
/// # Note
/// The actual syntax might change later since this is a algebra specific macro. For instance the `Abs(x)` function could change to `|x|` which is
/// known from mathematical notations for the "absolute" value of something.
#[derive(Debug)]
pub enum RExpr{
    ///A multi sub-expression 
    MultiExpr{
        ///The list of sub operations keyed by their assigned variable name
        idents: FxHashMap<Ident, Ident>, //all idents
        exp_order: Vec<(Ident, SubOp)>,
        //Literal of the value that has to be returned at the end of the operation
        return_key: Ident,
    },
    ///Expression with just one sub operation
    Single(SubOp)
    
}

impl Parse for RExpr{
    fn parse(stream: ParseStream) -> syn::Result<Self>{

        //First action is to test wether we have a let statement or not. If it is a let we can expect at least
        //one expression and one return statement. If not, we might have a "single line" expression which is returned as is.
        
        if stream.peek(Token![let]){
            let mut idents = FxHashMap::with_capacity_and_hasher(2, BuildHasherDefault::default());
            let mut exp_order = Vec::new();

            //Parse all sub ops
            //Check if the next is a let, if not stop
            while stream.peek(Token![let]){
                // check that the piece after the let is a variable name that is not picked yet, if so walk behind the `=`
                //and parse the subop.
                let _lettoken = stream.parse::<Token![let]>()?;
                match stream.parse::<Ident>(){
                    Ok(ident) => {
                        //parse type information
                        let ty: Ident = {
                            //exprecting a : token, then the type
                            let _t = stream.parse::<Token![:]>()?;
                            let ty: Ident = stream.parse::<Ident>()?;
                            ty
                        };
                        
                        //Assuming that the next "thing" is the `=` token.
                        //TODO: Later we might allow type hints, but for the moment we act as an interpreter that "hopes for the best"
                        let _eqtoken = stream.parse::<Token![=]>()?;
                        
                        //the next must be the actual operation stream we are therfore parsing that and, if successful push it into our map
                        let mut sub_op = SubOp::parse_in_context(&idents, stream)?;

			//After parsing in context, make post parsing transformations
			//Aka. hacks
			sub_op.transform_local_variables(false, &idents)?;
			
                        //Now check that the left token after parsing the SubOp is a ; token. Otherwise something might be wrong
                        let _coltoken = stream.parse::<Token![;]>()?;
                        
                        idents.insert(ident.clone(), ty.clone());
                        exp_order.push((ident, sub_op));
                    }
                    Err(e) => {
                        abort!(e.span(), "Expected unused identifier (in this case variable name)");
                    }
                }
            }

            //Returned therefore the next item must be the return statement that tells us which variable to use for return.
            let return_key = match stream.parse::<Token![return]>(){
                Ok(_) => {
                    //is return, therefore next should be ident of the thing to be returned
                    stream.parse::<Ident>()?
                },
                Err(e) => {
                    abort!(e.span(), "Expected return statement");
                }
            };
	    
            //Now assert that the return ident exists, otherwise the formula makes no sense
            if !idents.contains_key(&return_key){
                abort!(return_key.span(), "Returning value {}, but was not defined before!", return_key);
            }

	    //Takeout the last ";" to clear the pars stream
	    let _fintoken = stream.parse::<Token![;]>();
            
            Ok(RExpr::MultiExpr {
                exp_order,
                idents,
                return_key
            })
        }else{
            //using an empty hash set as context since we cant have variables
            let map: FxHashMap<Ident, Ident> = FxHashMap::with_capacity_and_hasher(0, BuildHasherDefault::default());
            //TODO: within algae it is possible to inherit a variable context. In this
            //      grammar however it is not, since we can't check that stuff at compiletime.
            //      Mayeb we expose a more "unsafe" macro?
            let sub_op = SubOp::parse_in_context(&map, stream)?;
            Ok(RExpr::Single(sub_op))
        }
    }
}

///Serializes the rust expression into an actual [Operation](algae::Operation)
impl ToTokens for RExpr{
    fn to_tokens(&self, tokens: &mut TokenStream2){

	//Depending on the expression type we either start by creating
	//a context in which each expression is written, or can read from,
	// or in the case of a single expression
	//just tokenize the single expression.

	//eprintln!("ExpressionTree:\n{:#?}", self);
	
	match &self{
	    RExpr::MultiExpr{exp_order, idents, return_key} => {
		let mut order = exp_order.to_vec();
		let _idents = idents.clone();
		let _return_key = return_key;

		let (first_key, first_exp) = order.remove(0); //Assuming we got at least one.

		//start ordered operation, each following is just "pushed"
		//onto the builder.
		tokens.extend(quote!{
		    algae::operations::OrderedOperations::new(
			stringify!(#first_key),
			Box::new(#first_exp)
		    )
		});

		for (op_var, op) in order{
		    tokens.extend(quote!{
			.push(
			    stringify!(#op_var),
			    Box::new(#op)
			)
		    })
		}
	    }
	    RExpr::Single(expr) => {
		tokens.extend(quote!{
		    Box::new(#expr)
		});
	    }
	};
    }
}
