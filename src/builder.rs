//! Builder pattern implementation for state machines

use crate::{StateMachine, Stateful};
use crate::fsm::SuperstateFn;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Builder for constructing state machines
pub struct StateMachineBuilder<S, CTX, E>
where
    S: Hash + Eq + Clone + Send + Debug + 'static,
    E: Debug + Send + 'static,
    CTX: Send + 'static,
{
    context: CTX,
    states: HashMap<S, Box<dyn Stateful<S, CTX, E> + Send + Sync>>,
    superstate_fn: Option<SuperstateFn<S>>,
}

impl<S, CTX, E> StateMachineBuilder<S, CTX, E>
where
    S: Hash + Eq + Clone + Send + Debug + 'static,
    E: Debug + Send + 'static,
    CTX: Send + 'static,
{
    /// Create a new builder with the given context
    pub fn new(context: CTX) -> Self {
        Self {
            context,
            states: HashMap::new(),
            superstate_fn: None,
        }
    }

    /// Add a state to the state machine
    pub fn state<T>(mut self, state_id: S, state_impl: T) -> Self
    where
        T: Stateful<S, CTX, E> + 'static,
    {
        self.states.insert(state_id, Box::new(state_impl));
        self
    }

    /// Set the superstate function for hierarchical behavior
    pub fn superstate_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&S) -> Option<S> + Send + Sync + 'static,
    {
        self.superstate_fn = Some(Box::new(func));
        self
    }

    /// Build the state machine
    pub fn build(self) -> StateMachine<S, CTX, E> {
        StateMachine::new(self.context, self.states, self.superstate_fn)
    }
}