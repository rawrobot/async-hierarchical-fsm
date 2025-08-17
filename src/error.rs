//! Error types for the state machine

use std::fmt::Debug;
use thiserror::Error;

/// Result type alias for state machine operations
pub type FsmResult<T, S> = std::result::Result<T, FsmError<S>>;

/// Errors that can occur during state machine operations
#[derive(Error, Debug)]
pub enum FsmError<S: Debug> {
    /// State machine has not been initialized
    #[error("State machine not initialized")]
    StateMachineNotInitialized,
    
    /// State returned an error during processing
    #[error("State {0:?} error: {1}")]
    StateInvalid(S, String),
    
    /// Event could not be handled by the current state or its superstates
    #[error("Invalid event in state {0:?}: {1}")]
    InvalidEvent(S, String),
    
    /// Attempted to use a state that wasn't registered with the builder
    #[error("State {0:?} not registered")]
    StateNotRegistered(S),

 /// `On_enter` method can't return Super
    #[error("State {0:?} on_enter cannot return Super")]
    OnEnterSuper(S),

    /// Generic error type for custom errors
    #[error("Custom error: {0}")]
    Custom(String),
   }
