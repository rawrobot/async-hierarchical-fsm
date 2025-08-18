//! # Async Hierarchical State Machine
//!
//! A powerful, async-first hierarchical finite state machine implementation in Rust
//! with support for timeouts and context management.
//!
//! ## Features
//!
//! - 🔄 **Async/Await Support**: Built from the ground up for async operations
//! - 🏗️ **Hierarchical States**: Support for superstate delegation and state hierarchies
//! - ⏰ **Dynamic Timeouts**: Context-aware timeout management per state
//! - 🛡️ **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - 🧵 **Thread Safe**: Designed for concurrent environments
//!
//! ## Quick Start
//!
//! ```rust
//! use async_hierarchical_fsm::prelude::*;
//! use async_trait::async_trait;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum State { Off, On }
//!
//! #[derive(Debug, Clone)]
//! enum Event { PowerOn, PowerOff }
//!
//! struct Context { power_level: u8 }
//!
//! struct OffState;
//!
//! #[async_trait]
//! impl Stateful<State, Context, Event> for OffState {
//!     async fn on_enter(&mut self, context: &mut Context) -> Response<State> {
//!         context.power_level = 0;
//!         Response::Handled
//!     }
//!
//!     async fn on_event(&mut self, event: &Event, _context: &mut Context) -> Response<State> {
//!         match event {
//!             Event::PowerOn => Response::Transition(State::On),
//!             _ => Response::Error("Invalid event".to_string()),
//!         }
//!     }
//!
//!     async fn on_exit(&mut self, _context: &mut Context) {}
//! }
//!
//! # async fn example() -> FsmResult<(), State> {
//! let mut fsm = StateMachineBuilder::new(Context { power_level: 0 })
//!     .state(State::Off, OffState)
//!     .build();
//!
//! fsm.init(State::Off).await?;
//! fsm.process_event(&Event::PowerOn).await
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

// Use your original FSM implementation here - don't change it!
mod builder;
mod error;
mod fsm;

pub use async_trait::async_trait;
pub use builder::StateMachineBuilder;
pub use error::{FsmError, FsmResult};
pub use fsm::{Response, StateMachine, Stateful};
pub use std::time::Duration;

#[cfg(feature = "tokio-integration")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-integration")))]
/// Tokio-specific timeout utilities
pub mod tokio_utils {
    //! Tokio utilities for timeout management and async operations

    use crate::{FsmError, StateMachine};
    use std::fmt::Debug;
    use std::hash::Hash;
    use tokio::time::{Duration, timeout};

    /// Process an event with a timeout
    pub async fn process_event_with_timeout<S, CTX, E>(
        fsm: &mut StateMachine<S, CTX, E>,
        event: &E,
        timeout_duration: Duration,
    ) -> Result<(), FsmError<S>>
    where
        S: Hash + Eq + Clone + Send + Debug + 'static,
        E: Debug + Send + 'static,
        CTX: Send + 'static,
    {
        timeout(timeout_duration, fsm.process_event(event))
            .await
            .map_err(|_| FsmError::Timeout)?
    }
}

#[cfg(not(feature = "tokio-integration"))]
/// Stub module when tokio-integration is not enabled
pub mod tokio_utils {
    // Tokio utilities are not available without the `tokio-integration` feature
}

pub mod prelude {
    //! Prelude module for convenient imports

    pub use crate::{
        Duration, FsmError, FsmResult, Response, StateMachine, StateMachineBuilder, Stateful,
        async_trait,
    };

    #[cfg(feature = "tokio-integration")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio-integration")))]
    pub use crate::tokio_utils::*;
}
