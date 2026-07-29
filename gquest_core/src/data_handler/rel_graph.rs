use std::collections::{HashMap, HashSet};

use serde_json::map::Iter;
use thiserror::Error;

use crate::{
    data_handler::{
        data_types::{ConstantValue, ValueType},
        module::Module,
    },
    parser::parsed_expression::MathExpression,
};

#[derive(Debug, Error)]
pub enum RelationGraphError {
    #[error("Tried to use \"{0}\" but no module references this function.")]
    UnknownFunction(String),
    #[error("The given function ref (\"{0}\", {1}) is not known.")]
    UnknownFunctionRef(String, usize),
    #[error("Tried to call the function \"{0}\" with {1} argument instead of {2}.")]
    DifferentArgumentNumber(String, usize, usize),

    #[error(
        "Tried to call the function \"{0}\" with an argument of type {2} instead of {3} for \"{1}\"."
    )]
    DifferentArgumentType(String, String, ValueType, ValueType),
}

/// The unique identifier to a function call : (`function_name`, `id`).
pub type FnRef = (String, usize);

#[derive(Debug)]
pub struct RelationGraph<'a> {
    modules: &'a HashMap<String, Module>,
    relations: Vec<Relation>,
    fn_calls_map: HashMap<String, Vec<FnCall>>,
}

#[derive(Debug, Clone)]
struct Relation {
    id: FnRef,
    /// The set of function relying on *this* function call (not the other way arround).
    dep: HashSet<usize>,
    /// The dependency left to compute for this function call.
    dep_left: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct FnCall {
    fn_name: String,
    args: Vec<MathExpression<FnArg>>,
    graph_id: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FnArg {
    /// The result of the call to another function. Sometimes referred to as a "flattened" function.
    FnCall(FnRef),
    /// A constant value.
    Constant(ConstantValue),
    /// The ref to a dataset.
    Graph,
}

impl<'a> RelationGraph<'a> {
    pub fn new(modules: &'a HashMap<String, Module>) -> Self {
        let mut fn_calls_map = HashMap::new();
        modules.keys().for_each(|k| {
            fn_calls_map.insert(k.to_string(), Vec::new());
        });

        Self {
            modules,
            relations: Vec::new(),
            fn_calls_map,
        }
    }

    /// Adds a dependency between two nodes of the graph, `f1` depends on `f2`.
    pub fn try_add_dep(&mut self, f1: &FnRef, f2: &FnRef) -> Result<(), RelationGraphError> {
        let c1 = self.get_call(f1)?.graph_id;
        let c2 = self.get_call(f2)?.graph_id;

        // f2 is a dependency of f1
        let was_not_present = self
            .relations
            .get_mut(c2)
            .expect("correct graph id")
            .dep
            .insert(c1);

        // if the dependency was not already present
        if was_not_present {
            // so f1 has now an additional dependency left to compute
            self.relations
                .get_mut(c1)
                .expect("correct graph id")
                .dep_left += 1;
        }

        Ok(())
    }

    /// Remove the given function ref from the other dependencies.
    /// Returns the list of the graph function with no dependencies thanks to the removal of this one.
    pub fn try_remove_fn_from_deps(
        &mut self,
        fn_ref: &FnRef,
    ) -> Result<Vec<usize>, RelationGraphError> {
        let mut newly_free = Vec::new();
        let fn_id = self.get_call(fn_ref)?.clone();
        let fn_deps = self
            .relations
            .get(fn_id.graph_id)
            .expect("correct relation")
            .dep
            .clone();

        for dep in fn_deps {
            let fn_dep_on = self.relations.get_mut(dep).expect("correct value");
            fn_dep_on.dep_left -= 1;
            if fn_dep_on.dep_left == 0 {
                newly_free.push(dep);
            }
        }
        // Extra caution by removing the dependencies stored for this call.
        // so as to not run into an overflow if the function is called a second time.
        self.relations
            .get_mut(fn_id.graph_id)
            .expect("correct relation")
            .dep
            .clear();

        Ok(newly_free)
    }

    /// Tries to add a function call to the graph if it didn't already exist.
    pub fn try_add_fn_call(
        &mut self,
        fn_name: String,
        args: &[MathExpression<FnArg>],
    ) -> Result<FnRef, RelationGraphError> {
        // Check if it was already added
        if let Some(fn_ref) = self.get_ref(&fn_name, args) {
            Ok(fn_ref)
        }
        // if it wasn't, perform some additional compatibility verifications
        else {
            // Check its validity
            self.try_valid_call(&fn_name, args)?;

            // Get the new graph id
            let graph_id = self.relations.len();

            // Add it to the calls map
            let calls = self.fn_calls_map.get_mut(&fn_name).expect("module present");
            let call_id = calls.len();
            calls.push(FnCall {
                fn_name: fn_name.clone(),
                args: args.to_vec(),
                graph_id,
            });

            // Add it to the relations map
            let fn_ref = (fn_name, call_id);
            self.relations.push(Relation {
                id: fn_ref.clone(),
                dep: HashSet::new(),
                dep_left: 0,
            });

            Ok(fn_ref)
        }
    }

    /// Checks if the given call:
    /// * matches a module's function
    /// * it's number of arguments
    /// * and their types.  
    fn try_valid_call(
        &self,
        fn_name: &String,
        args: &[MathExpression<FnArg>],
    ) -> Result<(), RelationGraphError> {
        let module = self.get_module(fn_name)?;

        if module.args.len() != args.len() {
            return Err(RelationGraphError::DifferentArgumentNumber(
                fn_name.clone(),
                args.len(),
                module.args.len(),
            ));
        }

        for (i, arg) in args.iter().enumerate() {
            let module_arg = &module.args[i];
            let found_type = self.get_type(arg)?;
            let needed_type = &module_arg.data_type;
            if *needed_type != found_type {
                return Err(RelationGraphError::DifferentArgumentType(
                    fn_name.clone(),
                    module_arg.name.clone(),
                    found_type,
                    needed_type.clone(),
                ));
            }
        }

        Ok(())
    }

    fn get_call(&self, fn_ref: &FnRef) -> Result<&FnCall, RelationGraphError> {
        match self.fn_calls_map.get(&fn_ref.0) {
            Some(calls) => match calls.get(fn_ref.1) {
                Some(fn_call) => Ok(fn_call),
                None => Err(RelationGraphError::UnknownFunctionRef(
                    fn_ref.0.clone(),
                    fn_ref.1,
                )),
            },
            None => Err(RelationGraphError::UnknownFunction(fn_ref.0.clone())),
        }
    }

    fn get_module(&self, fn_name: &String) -> Result<&Module, RelationGraphError> {
        match self.modules.get(fn_name) {
            Some(m) => Ok(m),
            None => Err(RelationGraphError::UnknownFunction(fn_name.clone())),
        }
    }

    fn get_type(&self, arg: &MathExpression<FnArg>) -> Result<ValueType, RelationGraphError> {
        if let MathExpression::Primitif(prim) = arg {
            Ok(match prim {
                FnArg::FnCall(fn_ref) => {
                    let module = self.get_module(&fn_ref.0)?;
                    module.output.clone()
                }
                FnArg::Constant(constant_value) => match constant_value {
                    ConstantValue::Numeric(_) => ValueType::Numeric,
                    ConstantValue::String(_) => ValueType::String,
                    ConstantValue::Bool(_) => ValueType::Bool,
                },
                FnArg::Graph => ValueType::Graph,
            })
        } else {
            // TODO: Numeric types should also be checked to be compatible or not (STRING + NUMERIC = ERROR)
            Ok(ValueType::Numeric)
        }
    }

    fn get_ref(&self, fn_name: &String, args: &[MathExpression<FnArg>]) -> Option<FnRef> {
        let calls = self.fn_calls_map.get(fn_name)?;

        let i = calls
            .iter()
            .position(|saved_call| saved_call.args == args)?;

        Some((fn_name.clone(), i))
    }
}

impl<'a> IntoIterator for RelationGraph<'a> {
    type Item = FnRef;

    type IntoIter = FnCallIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let mut can_now_exec = Vec::new();
        for (i, rel) in self.relations.iter().enumerate() {
            if rel.dep_left == 0 {
                can_now_exec.push(i);
            }
        }
        FnCallIterator {
            relation_graph: self,
            being_computed: HashSet::new(),
            can_now_exec,
        }
    }
}

#[derive(Debug)]
pub struct AutoFnCallIterator<'a> {
    module_iterator: FnCallIterator<'a>,
}

impl<'a> Iterator for AutoFnCallIterator<'a> {
    type Item = FnRef;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.module_iterator.next() {
            self.module_iterator.finish_function(&next);
            Some(next)
        } else {
            None
        }
    }
}

impl<'a> From<FnCallIterator<'a>> for AutoFnCallIterator<'a> {
    fn from(val: FnCallIterator<'a>) -> Self {
        AutoFnCallIterator {
            module_iterator: val,
        }
    }
}

#[derive(Debug)]
pub struct FnCallIterator<'a> {
    relation_graph: RelationGraph<'a>,
    being_computed: HashSet<FnRef>,
    /// the list of graph id of functions that can now be executed.
    can_now_exec: Vec<usize>,
}

impl<'a> FnCallIterator<'a> {
    /// Checks if the iterator currently has an available function to return.
    pub fn has_next(&self) -> bool {
        !self.can_now_exec.is_empty()
    }

    /// Tells the iterator that the given function is over and that the
    /// other dependencies can be free'ed.
    pub fn finish_function(&mut self, fn_ref: &FnRef) {
        if self.being_computed.remove(fn_ref) {
            self.can_now_exec.append(
                &mut self
                    .relation_graph
                    .try_remove_fn_from_deps(fn_ref)
                    .expect("no errors when removing function"),
            );
        }
    }

    /// Get the next function to execute.
    /// The iterator will suppose that the function is being executed until
    /// the function [`FnCallIterator::finish_function`] is called for the reference.
    pub fn next_function(&mut self) -> Option<FnRef> {
        if let Some(next_graph_id) = self.can_now_exec.pop() {
            let fn_ref = self.relation_graph.relations[next_graph_id].id.clone();
            self.being_computed.insert(fn_ref.clone());
            Some(fn_ref)
        } else {
            None
        }
    }
}

impl<'a> Iterator for FnCallIterator<'a> {
    type Item = FnRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_function()
    }
}
