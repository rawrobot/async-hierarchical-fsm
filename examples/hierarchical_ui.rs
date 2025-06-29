//! Hierarchical UI state machine example

use async_hierarchical_fsm::prelude::*;
use std::time::Duration;

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
}

#[derive(Debug)]
struct UIContext {
    menu_index: usize,
    brightness: u8,
    volume: u8,
}

impl UIContext {
    fn new() -> Self {
        Self {
            menu_index: 0,
            brightness: 50,
            volume: 30,
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

    async fn on_event(&mut self, event: &UIEvent, _context: &mut UIContext) -> Response<UIState> {
        match event {
            UIEvent::Enter => {
                println!("📱 Opening main menu...");
                Response::Transition(UIState::Menu)
            }
            UIEvent::Home => {
                println!("🏠 Already at home");
                Response::Handled
            }
            _ => Response::Error("Invalid action from home screen".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("⬆️  Leaving home screen");
    }
}

struct MenuState;

#[async_trait]
impl Stateful<UIState, UIContext, UIEvent> for MenuState {
    async fn on_enter(&mut self, context: &mut UIContext) -> Response<UIState> {
        context.menu_index = 0;
        println!("📋 Main menu opened (index: {})", context.menu_index);
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
                println!("⬆️  Menu index: {}", context.menu_index);
                Response::Handled
            }
            UIEvent::Down => {
                context.menu_index += 1;
                println!("⬇️  Menu index: {}", context.menu_index);
                Response::Handled
            }
            UIEvent::Select => {
                println!("⚙️  Opening settings...");
                Response::Transition(UIState::Settings)
            }
            UIEvent::Home => Response::Super, // Delegate to parent (Root)
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut UIContext) {
        println!("⬆️  Leaving menu");
    }
}

// ... rest of the states

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📱 Hierarchical UI State Machine Example\n");

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
    
    // Demo navigation
    let events = vec![
        UIEvent::Enter,    // Root -> Menu
        UIEvent::Select,   // Menu -> Settings  
        UIEvent::Home,     // Settings -> Root (via superstate)
    ];

    for event in events {
        println!("\n📨 Event: {:?}", event);
        ui.process_event(&event).await?;
        println!("📍 Current: {:?}", ui.current_state());
    }

    Ok(())
}
