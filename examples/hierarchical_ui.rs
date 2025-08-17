//! Hierarchical UI state machine example
//!
//! This example demonstrates a hierarchical state machine for a UI system:
//! - Root: Main screen/home
//! - Menu: Main navigation menu (child of Root)
//! - Settings: Settings menu (child of Root)
//! - Display: Display settings (child of Settings)
//! - Audio: Audio settings (child of Settings)
//!
//! The hierarchy allows for:
//! - Event delegation to parent states
//! - Shared behavior across state families
//! - Natural navigation patterns
//!
//! Run with: cargo run --example hierarchical_ui --features tokio-integration

use async_hierarchical_fsm::prelude::*;
use tokio::time::{sleep, Duration};

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
    Enter,
    Back,
    Up,
    Down,
    Select,
    Home,
    Quit,
}

#[derive(Debug)]
struct UIContext {
    menu_index: usize,
    brightness: u8,
    volume: u8,
    should_quit: bool,
}

impl UIContext {
    fn new() -> Self {
        Self {
            menu_index: 0,
            brightness: 50,
            volume: 30,
            should_quit: false,
        }
    }
}

struct RootState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for RootState {
    async fn on_enter(&mut self, _context: &mut UIContext) -> Response<UIState> {
        println!("🏠 Welcome to the main screen");
        Response::Handled
    }

    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Enter => {
                println!("📱 Opening main menu...");
                Response::Transition(UIState::Menu)
            }
            UIEvent::Home => {
                println!("🏠 Already at home");
                Response::Handled
            }
            UIEvent::Quit => {
                println!("👋 Goodbye!");
                context.should_quit = true;
                Response::Handled
            }
            _ => Response::Error("Invalid action from home screen".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("📤 Leaving home screen");
    }
}

struct MenuState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for MenuState {
    async fn on_enter(&mut self, context: &mut UIContext) -> Response<UIState> {
        context.menu_index = 0;
        println!("📋 Main menu opened");
        Response::Handled
    }

    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Back => {
                println!("🔙 Going back to home...");
                Response::Transition(UIState::Root)
            }
            UIEvent::Up => {
                if context.menu_index > 0 {
                    context.menu_index -= 1;
                }
                Response::Handled
            }
            UIEvent::Down => {
                context.menu_index = (context.menu_index + 1).min(0); // Only one menu item
                Response::Handled
            }
            UIEvent::Enter | UIEvent::Select => {
                println!("⚙️  Opening settings...");
                Response::Transition(UIState::Settings)
            }
            UIEvent::Home => Response::Super, // Delegate to parent (Root)
            UIEvent::Quit => Response::Super, // Delegate to parent (Root)
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("📤 Leaving main menu");
    }
}

struct SettingsState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for SettingsState {
    async fn on_enter(&mut self, context: &mut UIContext) -> Response<UIState> {
        context.menu_index = 0;
        println!("⚙️  Settings menu opened");
        Response::Handled
    }

    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Back => {
                println!("🔙 Going back to main menu...");
                Response::Transition(UIState::Menu)
            }
            UIEvent::Up => {
                if context.menu_index > 0 {
                    context.menu_index -= 1;
                    println!("⬆️  Settings index: {}", context.menu_index);
                }
                Response::Handled
            }
            UIEvent::Down => {
                if context.menu_index < 1 {
                    context.menu_index += 1;
                    println!("⬇️  Settings index: {}", context.menu_index);
                }
                Response::Handled
            }
            UIEvent::Enter | UIEvent::Select => {
                match context.menu_index {
                    0 => {
                        println!("🖥️  Opening display settings...");
                        Response::Transition(UIState::Display)
                    }
                    1 => {
                        println!("🔊 Opening audio settings...");
                        Response::Transition(UIState::Audio)
                    }
                    _ => Response::Handled,
                }
            }
            UIEvent::Home => Response::Super, // Delegate to parent (Root)
            UIEvent::Quit => Response::Super, // Delegate to parent (Root)
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("📤 Leaving settings");
    }
}

struct DisplayState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for DisplayState {
    async fn on_enter(&mut self, context: &mut UIContext) -> Response<UIState> {
        println!("🖥️  Display settings (brightness: {}%)", context.brightness);
        println!("   Use ↑/↓ to adjust brightness, Enter to save, Esc to go back");
        Response::Handled
    }

    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Back => {
                println!("🔙 Going back to settings...");
                Response::Transition(UIState::Settings)
            }
            UIEvent::Up => {
                if context.brightness < 100 {
                    context.brightness = (context.brightness + 10).min(100);
                    println!("🔆 Brightness increased to {}%", context.brightness);
                }
                Response::Handled
            }
            UIEvent::Down => {
                if context.brightness > 0 {
                    context.brightness = context.brightness.saturating_sub(10);
                    println!("🔅 Brightness decreased to {}%", context.brightness);
                }
                Response::Handled
            }
            UIEvent::Enter => {
                println!("💾 Brightness saved at {}%", context.brightness);
                Response::Handled
            }
            // Home and other events delegate to superstate (Settings -> Root)
            _ => Response::Super,
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("📤 Leaving display settings");
    }
}

struct AudioState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for AudioState {
    async fn on_enter(&mut self, context: &mut UIContext) -> Response<UIState> {
        println!("🔊 Audio settings (volume: {}%)", context.volume);
        println!("   Use ↑/↓ to adjust volume, Enter to save, Esc to go back");
        Response::Handled
    }

    async fn on_event(&mut self, event: &UIEvent, context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Back => {
                println!("🔙 Going back to settings...");
                Response::Transition(UIState::Settings)
            }
            UIEvent::Up => {
                if context.volume < 100 {
                    context.volume = (context.volume + 10).min(100);
                    println!("🔊 Volume increased to {}%", context.volume);
                }
                Response::Handled
            }
            UIEvent::Down => {
                if context.volume > 0 {
                    context.volume = context.volume.saturating_sub(10);
                    println!("🔉 Volume decreased to {}%", context.volume);
                }
                Response::Handled
            }
            UIEvent::Enter => {
                println!("💾 Volume saved at {}%", context.volume);
                Response::Handled
            }
            // Home and other events delegate to superstate (Settings -> Root)
            _ => Response::Super,
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("📤 Leaving audio settings");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📱 Hierarchical UI State Machine Example");
    println!("=========================================\n");

    let mut ui = StateMachineBuilder::new(UIContext::new())
        .state(UIState::Root, RootState)
        .state(UIState::Menu, MenuState)
        .state(UIState::Settings, SettingsState)
        .state(UIState::Display, DisplayState)
        .state(UIState::Audio, AudioState)
        .superstate_fn(|state| match state {
            UIState::Menu | UIState::Settings => Some(UIState::Root),
            UIState::Display | UIState::Audio => Some(UIState::Settings),
            _ => None,
        })
        .build();

    ui.init(UIState::Root).await?;

    println!("🎯 Demo: Automatic Navigation Through UI Hierarchy\n");

    // Demo navigation sequence
    let events = vec![
        (UIEvent::Enter, "Opening main menu"),
        (UIEvent::Select, "Selecting settings"),
        (UIEvent::Down, "Navigate to audio settings"),
        (UIEvent::Select, "Enter audio settings"),
        (UIEvent::Up, "Increase volume"),
        (UIEvent::Up, "Increase volume again"),
        (UIEvent::Enter, "Save volume setting"),
        (UIEvent::Back, "Back to settings menu"),
        (UIEvent::Up, "Navigate to display settings"),
        (UIEvent::Select, "Enter display settings"),
        (UIEvent::Down, "Decrease brightness"),
        (UIEvent::Down, "Decrease brightness again"),
        (UIEvent::Enter, "Save brightness setting"),
        (UIEvent::Home, "Go home (via superstate delegation)"),
    ];

    for (event, description) in events {
        println!("📨 {}: {:?}", description, event);
        
        match ui.process_event(&event).await {
            Ok(()) => {
                println!("✅ Event processed");
                println!("📍 Current state: {:?}", ui.current_state());
                println!("🔆 Brightness: {}%, 🔊 Volume: {}%", 
                    ui.context().brightness, ui.context().volume);
            }
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }
        
        println!(); // Empty line for readability
        
        // Delay for better visualization
        sleep(Duration::from_millis(1500)).await;
    }

    println!("🎉 Demo completed!");
    println!("Final state: {:?}", ui.current_state());
    println!("Final settings - Brightness: {}%, Volume: {}%", 
        ui.context().brightness, ui.context().volume);

    Ok(())
}