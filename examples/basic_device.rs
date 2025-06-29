//! Basic device state machine example

// Import from your library
use async_hierarchical_fsm::{
    StateMachine, StateMachineBuilder, Stateful, Response, 
    async_trait, Duration
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DeviceState {
    Off,
    Standby,
    Active,
    Error,
}

#[derive(Debug, Clone)]
enum DeviceEvent {
    PowerOn,
    PowerOff,
    Activate,
    Deactivate,
    ErrorOccurred,
    Reset,
}

#[derive(Debug)]
struct DeviceContext {
    power_level: u8,
    error_count: u32,
}

impl DeviceContext {
    fn new() -> Self {
        Self {
            power_level: 0,
            error_count: 0,
        }
    }
}

struct OffState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for OffState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 0;
        println!("📴 Device powered off (power: {}%)", context.power_level);
        Response::Handled
    }

    async fn on_event(&mut self, event: &DeviceEvent, _context: &mut DeviceContext) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOn => {
                println!("🔌 Powering on device...");
                Response::Transition(DeviceState::Standby)
            }
            _ => Response::Error("Device is off - only PowerOn allowed".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {
        println!("⬆️  Leaving off state");
    }
}

// ... implement other states

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Device State Machine Example\n");

    // Build the state machine using your library
    let mut device = StateMachineBuilder::new(DeviceContext::new())
        .state(DeviceState::Off, OffState)
        .state(DeviceState::Standby, StandbyState)
        .state(DeviceState::Active, ActiveState)
        .state(DeviceState::Error, ErrorState)
        .build();

    // Initialize and run
    device.init(DeviceState::Off).await?;
    
    // ... rest of example

    Ok(())
}
