# Async Hierarchical State Machine


A powerful, async-first hierarchical finite state machine implementation in Rust with support for timeouts and context management.

## ✨ Features

- 🔄 **Async/Await Support**: Built from the ground up for async operations
- 🏗️ **Hierarchical States**: Support for superstate delegation and state hierarchies
- ⏰ **Dynamic Timeouts**: Context-aware timeout management per state
- 🛡️ **Type Safety**: Leverages Rust's type system for compile-time guarantees
- 🧵 **Thread Safe**: Designed for concurrent environments
- 🎯 **Zero-Cost Abstractions**: Minimal runtime overhead
- 📝 **Comprehensive Logging**: Built-in transition and event logging

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-hierarchical-fsm = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Example

```rust
use async_hierarchical_fsm::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State { Off, On }

#[derive(Debug, Clone)]
enum Event { PowerOn, PowerOff }

struct Context { power_level: u8 }

struct OffState;

#[async_trait]
impl Stateful<State, Context, Event> for OffState {
    async fn on_enter(&mut self, context: &mut Context) -> Response<State> {
        context.power_level = 0;
        println!("Device powered off");
        Response::Handled
    }

    async fn on_event(&mut self, event: &Event, _context: &mut Context) -> Response<State> {
        match event {
            Event::PowerOn => Response::Transition(State::On),
            _ => Response::Error("Invalid event".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut Context) {
        println!("Leaving off state");
    }
}

struct OnState;

#[async_trait]
impl Stateful<State, Context, Event> for OnState {
    async fn on_enter(&mut self, context: &mut Context) -> Response<State> {
        context.power_level = 100;
        println!("Device powered on");
        Response::Handled
    }

    async fn on_event(&mut self, event: &Event, _context: &mut Context) -> Response<State> {
        match event {
            Event::PowerOff => Response::Transition(State::Off),
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut Context) {
        println!("Leaving on state");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fsm = StateMachineBuilder::new(Context { power_level: 0 })
        .state(State::Off, OffState)
        .state(State::On, OnState)
        .build();

    fsm.init(State::Off).await?;
    fsm.process_event(&Event::PowerOn).await?;
    
    println!("Current state: {:?}", fsm.current_state());
    println!("Power level: {}", fsm.context().power_level);
    
    Ok(())
}
```

## 🏗️ Hierarchical States

The state machine supports hierarchical states with superstate delegation:

```rust
use async_hierarchical_fsm::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum UIState {
    Root,
    Menu,
    Settings,
    Display,
    Audio,
}

#[derive(Debug, Clone)]
enum UIEvent {
    Enter, Back, Up, Down, Select, Home
}

// Menu state that delegates Home events to Root
struct MenuState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for MenuState {
    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Back => Response::Transition(UIState::Root),
            UIEvent::Select => Response::Transition(UIState::Settings),
            UIEvent::Home => Response::Super, // Delegate to parent (Root)
            _ => Response::Handled,
        }
    }
    // ... other methods
}

let mut ui = StateMachineBuilder::new(UIContext::new())
    .state(UIState::Root, RootState)
    .state(UIState::Menu, MenuState)
    .state(UIState::Settings, SettingsState)
    .superstate_fn(|state| match state {
        UIState::Menu | UIState::Settings => Some(UIState::Root),
        UIState::Display | UIState::Audio => Some(UIState::Settings),
        _ => None,
    })
    .build();
```

## ⏰ Timeouts

States can define dynamic timeouts based on context:

```rust
#[async_trait]
impl Stateful<State, Context, Event> for IdleState {
    async fn get_timeout(&self, context: &Context) -> Option<Duration> {
        if context.error_count > 3 {
            Some(Duration::from_secs(5))  // Shorter timeout after errors
        } else {
            Some(Duration::from_secs(30)) // Normal timeout
        }
    }
    
    // ... other methods
}

// Check timeout in your main loop
if let Some(timeout) = fsm.get_current_timeout().await {
    match tokio::time::timeout(timeout, fsm.process_event(&event)).await {
        Ok(result) => result?,
        Err(_) => {
            // Handle timeout
            fsm.process_event(&Event::Timeout).await?;
        }
    }
}
```


### Context Management

```rust
// Access context immutably
let power = fsm.context().power_level;

// Access context mutably
fsm.context_mut().error_count += 1;
```

### Error Handling

```rust
match fsm.process_event(&event).await {
    Ok(()) => println!("Event processed successfully"),
    Err(Error::InvalidEvent(state, msg)) => {
        eprintln!("Invalid event in state {:?}: {}", state, msg);
    }
    Err(Error::StateNotRegistered(state)) => {
        eprintln!("State {:?} not registered", state);
    }
    Err(e) => eprintln!("Other error: {:?}", e),
}
```

### Response Types

States can respond to events in four ways:

```rust
async fn on_event(&mut self, event: &Event, context: &mut Context) -> Response<State> {
    match event {
        Event::Stay => Response::Handled,                    // Stay in current state
        Event::Move => Response::Transition(State::Next),    // Transition to new state
        Event::Delegate => Response::Super,                  // Delegate to superstate
        Event::Invalid => Response::Error("Bad event".into()), // Return error
    }
}
```

## 📦 Feature Flags

- `default`: No additional features
- `plantuml`: Enable PlantUML diagram generation (debug builds only)
- `tokio-integration`: Enable Tokio-specific timeout utilities

```toml
[dependencies]
async-hierarchical-fsm = { version = "0.1", features = ["plantuml", "tokio-integration"] }
```

## 🔧 Examples

The repository includes several comprehensive examples:

- **Basic Device**: Simple on/off device with error handling
- **Hierarchical UI**: Multi-level menu system with superstate delegation
- **Event-Driven**: Complex state machine with timeouts and context management

Run examples:

```bash
cargo run --example basic_device --features "plantuml,tokio-integration"
cargo run --example hierarchical_ui --all-features
```

## 🧪 Testing

```bash
# Run all tests
cargo test --all-features

# Run with coverage
cargo tarpaulin --all-features

# Test specific features
cargo test --no-default-features
cargo test --features plantuml
```

## 🚀 Performance

The state machine is designed for minimal overhead:

- Zero-cost state transitions
- Compile-time state validation
- Minimal heap allocations
- Efficient event dispatching

Benchmarks show sub-microsecond event processing for typical use cases.

## 🛠️ Use Cases

Perfect for:

- **IoT Device Control**: Managing device states with timeouts and error recovery
- **Game State Management**: Handling game modes, menus, and gameplay states
- **Protocol Implementation**: State-driven network protocol handling
- **UI State Management**: Complex user interface state transitions
- **Workflow Engines**: Business process state management
- **Embedded Systems**: Resource-constrained async state management

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create a feature branch
3. Add tests for your changes
4. Ensure all tests pass
5. Submit a pull request

## 📄 License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

- Inspired by UML State Charts and Hierarchical State Machines
- Built with the excellent [async-trait](https://crates.io/crates/async-trait) crate
- Thanks to the Rust async ecosystem and Tokio project

---

**[Documentation](https://docs.rs/async-hierarchical-fsm) | [Crates.io](https://crates.io/crates/async-hierarchical-fsm) | [Repository](https://github.com/yourusername/async-hierarchical-fsm)**
