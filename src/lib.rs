//! # Async Hierarchical State Machine
//!
//! A powerful, async-first hierarchical finite state machine implementation in Rust 
//! with support for timeouts, context management, and PlantUML diagram generation.
//!
//! ## Features
//!
//! - 🔄 **Async/Await Support**: Built from the ground up for async operations
//! - 🏗️ **Hierarchical States**: Support for superstate delegation and state hierarchies
//! - ⏰ **Dynamic Timeouts**: Context-aware timeout management per state
//! - 📊 **PlantUML Export**: Automatic state diagram generation (debug builds only)
//! - 🛡️ **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - 🧵 **Thread Safe**: Designed for concurrent environments
//!
//! ## Quick Start
//!
//! ```rust
//! use async_hierarchical_fsm::*;
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
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut fsm = StateMachineBuilder::new(Context { power_level: 0 })
//!     .state(State::Off, OffState)
//!     .build();
//! 
//! fsm.init(State::Off).await?;
//! fsm.process_event(&Event::PowerOn).await?;
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub use async_trait::async_trait;

// Use your original FSM implementation here - don't change it!
mod error;
mod builder;
mod fsm;

#[cfg(all(feature = "plantuml", debug_assertions))]
mod plantuml;

pub use error::{Error, Result};
pub use builder::StateMachineBuilder;
pub use fsm::{StateMachine, Stateful, Response};

// Include your original fsm.rs content here exactly as you had it
// I won't regenerate it since you want to keep your working version

#[cfg(feature = "tokio-integration")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-integration")))]
pub use tokio::time::Duration;

#[cfg(not(feature = "tokio-integration"))]
pub use std::time::Duration;

pub mod prelude {
    //! Prelude module for convenient imports
    pub use crate::{StateMachine, StateMachineBuilder, Stateful, Response, Error, Result};
    pub use async_trait::async_trait;
    pub use std::time::Duration;
}
