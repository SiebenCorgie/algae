
use std::{marker::PhantomData, any::TypeId, hash::BuildHasherDefault};

use fxhash::FxHashMap;
use rspirv::spirv::Word;

use crate::{BoxOperation, DataId, Operation, Serializer};

///A result where the type is only known at runtime.
#[derive(Clone, Debug)]
struct AnonymResult{
    id: Word,
    ty: TypeId,
}

///Provides a runtime accessor for defined results based on a name.
#[derive(Clone, Debug)]
pub struct ResultContext{
    results: FxHashMap<String, AnonymResult>,
}

impl<'a> ResultContext{
    ///If available returns a result of type `T` with the given name.
    fn get<T: 'static>(&self, name: &str) -> Option<DataId<T>>{
        let tid = TypeId::of::<T>();
        if let Some(r) = self.results.get(name){
            //Name is the same, check that the types match.
            if r.ty == tid{
                Some(DataId::from(r.id))
            }else{
                None
            }
        }else{
            None
        }
    }

    fn insert(&mut self, name: String, res: AnonymResult){
        #[cfg(feature="logging")]
        let lname = name.clone();
        
        if let Some(_old) = self.results.insert(name, res){
            #[cfg(feature="logging")]
            log::warn!("result with name {} existed in context. Overwriting...", lname);
            //TODO/FIXME: Currently this will just silently pass. If the types of the old value
            //            At that name and the new values do not match, this can lead to panics, or
            //            unexpected behaviour.
            //            With checks enabled however this could be a mechanic to allow semi
            //            mutable values via overwrite.
        }
    }
}

///Operation that allows JIT-Compiletime access to a intermediate result of an [ResultContext] of type `T` and a given name.
///
/// # Safety
/// Panics if the result can't be accessed. Use [AccessOrDefault] to return a default value instead if the result does not exist.
pub struct AccessResult<T>{
    name: String,
    ty: PhantomData<T>
}

impl<T> AccessResult<T>{
    pub fn new(name: impl Into<String>) -> Self{
        AccessResult{
            name: name.into(),
            ty: PhantomData
        }
    }
}

//FIXME: The `ResultContext` should be passed by reference.
impl<T: 'static> Operation for AccessResult<T>{
    type Input = ResultContext;
    type Output = DataId<T>;

    fn serialize(&mut self, _serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        input.get(&self.name).expect(&format!("Expected result with name {}", self.name))
    }
}

///Allows bundling multiple operations in order, where each operation has access to the result of the former
/// operations.
///
/// The input `I` to the operation can be of type `DataId<I>`, in that case it is accessable via the name "input".
///
/// The input `I` to the operation can be of type `ResultContext`, in that case the local context is extended with
/// the supplied super context. This happens for instance if a OrderedOperation is called within a OrderedOperation.
///
/// Returns the result of the last operation
pub struct OrderedOperations<I, O>{
    //order of operations string is the name that is used for accessing the result.
    operations: Vec<(String, Box<dyn FnMut(&mut Serializer, ResultContext) -> AnonymResult + 'static>)>,
    input: PhantomData<I>,    
    output: PhantomData<O>,
}

impl<I: 'static, O: 'static> OrderedOperations<I, O>{
    pub fn new(op_name: impl Into<String>, op: BoxOperation<ResultContext, O>) -> Self{

        //create struct and push new op
        let newop: OrderedOperations<I, O> = OrderedOperations{
            input: PhantomData,
            operations: Vec::new(),
            output: PhantomData
        };

        newop.push(op_name, op)
    }

    pub fn push<R: 'static>(self, op_name: impl Into<String>, op: BoxOperation<ResultContext, R>) -> OrderedOperations<I, R>{

        let name = op_name.into();
        
        #[cfg(feature="logging")]
        log::info!("Adding operation with name {}", name);
        
        let OrderedOperations { input, mut operations, output: _ } = self;
        
        let anym_function: Box<dyn FnMut(&mut Serializer, ResultContext) -> AnonymResult + 'static> = Box::new({
            let mut innerop = op;
            move |serialized, ctx|{
                //Mask the inner operation by warapping it into the anonym map.
                let typed_res = innerop.serialize(serialized, ctx);
                AnonymResult{
                    id: typed_res.id,
                    ty: TypeId::of::<R>(),
                }
            }
        });
        operations.push((name, anym_function));
        
        OrderedOperations{
            input,
            operations,
            output: PhantomData
        }
    }
}


///Implementation that inherits some super context when being serialized.
impl<O: 'static> Operation for OrderedOperations<ResultContext, O>{
    type Input = ResultContext;
    type Output = DataId<O>;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        let mut context = input;

        let mut last_result = None;
        //Now serialize each operation with context
        for (opname, op) in self.operations.iter_mut(){
            //FIXME: hashmap clone should not be 
            let res = op(serializer, context.clone());
            //update last known result id
            last_result = Some(res.clone());
            //Push the new runtime result id into the context
            context.insert(opname.clone(), res);
        }

        //assert that the type id is correct for sanity purposes
        let result = last_result.unwrap();
        assert!(TypeId::of::<O>() == result.ty, "result type Id did not match");
        //Should be save to cast to actual dataid
        DataId::from(result.id)
    }
}

///Implementation for a chain that does not inherit any value.
impl<O: 'static> Operation for OrderedOperations<(), O>{
    type Input = ();
    type Output = DataId<O>;

    fn serialize(&mut self, serializer: &mut Serializer, input: Self::Input) -> Self::Output {
        //Create a local context and use the inheriting implementation
        let context = ResultContext{
            results: FxHashMap::with_capacity_and_hasher(2, BuildHasherDefault::default())
        };

        let mut metaop: OrderedOperations<ResultContext, O> = OrderedOperations{
            input: PhantomData,
            output: PhantomData,
            operations: Vec::new() //used to temporarly swap out operations.
        };

        //Swap ops
        core::mem::swap(&mut self.operations, &mut metaop.operations);
        
        let result = metaop.serialize(serializer, context);

        //Swap back
        core::mem::swap(&mut self.operations, &mut metaop.operations);
        
        result
    }
}
