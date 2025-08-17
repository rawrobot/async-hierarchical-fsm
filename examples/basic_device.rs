//! Basic device state machine example
//!
//! This example demonstrates a simple device with four states:
//! - Off: Device is powered down
//! - Standby: Device is on but not active
//! - Active: Device is fully operational
//! - Error: Device encountered an error and needs recovery
//!
//! The example shows:
//! - Basic state transitions
//! - Context management (power level, error count)
//! - Dynamic timeouts based on context
//! - Error handling and recovery
//! - PlantUML diagram export (debug builds only)

use async_hierarchical_fsm::{
    Duration, Response, StateMachine, StateMachineBuilder, Stateful, async_trait,
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
    Timeout,
}

#[derive(Debug)]
struct DeviceContext {
    power_level: u8,
    error_count: u32,
    uptime_seconds: u64,
}

impl DeviceContext {
    fn new() -> Self {
        Self {
            power_level: 0,
            error_count: 0,
            uptime_seconds: 0,
        }
    }

    fn increment_uptime(&mut self) {
        self.uptime_seconds += 1;
    }
}

struct OffState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for OffState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 0;
        context.uptime_seconds = 0;
        println!("📴 Device powered off (power: {}%)", context.power_level);
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        _context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOn => {
                println!("🔌 Powering on device...");
                Response::Transition(DeviceState::Standby)
            }
            _ => Response::Error("Device is off - only PowerOn is allowed".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {
        println!("⚡ Leaving off state");
    }
}

struct StandbyState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for StandbyState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 25;
        println!("⏸️  Device in standby mode (power: {}%)", context.power_level);
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOff => {
                println!("🔌 Powering off device...");
                Response::Transition(DeviceState::Off)
            }
            DeviceEvent::Activate => {
                println!("🚀 Activating device...");
                Response::Transition(DeviceState::Active)
            }
            DeviceEvent::ErrorOccurred => {
                println!("❌ Error occurred in standby!");
                Response::Transition(DeviceState::Error)
            }
            DeviceEvent::Timeout => {
                println!("⏰ Standby timeout - powering off to save energy");
                Response::Transition(DeviceState::Off)
            }
            _ => {
                context.increment_uptime();
                Response::Handled
            }
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {
        println!("📤 Leaving standby state");
    }

    async fn get_timeout(&self, _context: &DeviceContext) -> Option<Duration> {
        Some(Duration::from_secs(60)) // Auto-shutdown after 1 minute of inactivity
    }
}

struct ActiveState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for ActiveState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 100;
        println!("🟢 Device fully active (power: {}%)", context.power_level);
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOff => {
                println!("🔌 Emergency shutdown from active state");
                Response::Transition(DeviceState::Off)
            }
            DeviceEvent::Deactivate => {
                println!("⏸️  Deactivating device...");
                Response::Transition(DeviceState::Standby)
            }
            DeviceEvent::ErrorOccurred => {
                println!("❌ Critical error in active state!");
                Response::Transition(DeviceState::Error)
            }
            DeviceEvent::Timeout => {
                println!("⏰ Active timeout - returning to standby");
                Response::Transition(DeviceState::Standby)
            }
            _ => {
                context.increment_uptime();
                println!("🔄 Processing in active state (uptime: {}s)", context.uptime_seconds);
                Response::Handled
            }
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {
        println!("📤 Leaving active state");
    }

    async fn get_timeout(&self, context: &DeviceContext) -> Option<Duration> {
        // Shorter timeout if many errors occurred
        if context.error_count > 3 {
            println!("⚠️  Reduced timeout due to error history");
            Some(Duration::from_secs(10))
        } else {
            Some(Duration::from_secs(30))
        }
    }
}

struct ErrorState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for ErrorState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.error_count += 1;
        context.power_level = 10; // Minimal power in error state
        
        println!(
            "🚨 Device in error state (errors: {}, power: {}%)", 
            context.error_count, 
            context.power_level
        );

        if context.error_count >= 5 {
            println!("💥 Too many errors! Device will shut down on next reset.");
        }

        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::Reset => {
                if context.error_count < 5 {
                    println!("🔄 Resetting device - returning to standby");
                    Response::Transition(DeviceState::Standby)
                } else {
                    println!("🛑 Too many errors - shutting down for safety");
                    Response::Transition(DeviceState::Off)
                }
            }
            DeviceEvent::PowerOff => {
                println!("🔌 Manual shutdown from error state");
                Response::Transition(DeviceState::Off)
            }
            DeviceEvent::Timeout => {
                println!("⏰ Error state timeout - attempting auto-recovery");
                if context.error_count < 3 {
                    Response::Transition(DeviceState::Standby)
                } else {
                    Response::Transition(DeviceState::Off)
                }
            }
            _ => {
                println!("🚫 Ignoring event {:?} in error state", event);
                Response::Handled // Ignore other events in error state
            }
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {
        println!("📤 Leaving error state");
    }

    async fn get_timeout(&self, _context: &DeviceContext) -> Option<Duration> {
        Some(Duration::from_secs(5)) // Quick timeout in error state for auto-recovery
    }
}

fn create_device() -> StateMachine<DeviceState, DeviceContext, DeviceEvent> {
    let context = DeviceContext::new();

    StateMachineBuilder::new(context)
        .state(DeviceState::Off, OffState)
        .state(DeviceState::Standby, StandbyState)
        .state(DeviceState::Active, ActiveState)
        .state(DeviceState::Error, ErrorState)
        .build()
}

async fn simulate_device_operation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Starting device simulation...\n");

    let mut device = create_device();
    
    // Initialize device
    device.init(DeviceState::Off).await?;
    println!("Current state: {:?}\n", device.current_state());

    // Simulate device lifecycle
    let events = vec![
        (DeviceEvent::PowerOn, "Turning device on"),
        (DeviceEvent::Activate, "Activating device"),
        (DeviceEvent::Reset, "Processing some work"), // This will be handled
        (DeviceEvent::ErrorOccurred, "Simulating an error"),
        (DeviceEvent::Reset, "Recovering from error"),
        (DeviceEvent::Activate, "Reactivating device"),
        (DeviceEvent::ErrorOccurred, "Another error occurs"),
        (DeviceEvent::ErrorOccurred, "Yet another error"),
        (DeviceEvent::Reset, "Trying to recover"),
        (DeviceEvent::Deactivate, "Deactivating device"),
        (DeviceEvent::PowerOff, "Shutting down"),
    ];

    for (event, description) in events {
        println!("📋 {}", description);
        
        match device.process_event(&event).await {
            Ok(()) => {
                println!("✅ Event processed successfully");
                println!("📊 State: {:?}", device.current_state());
                println!("🔋 Power: {}%", device.context().power_level);
                println!("❌ Errors: {}", device.context().error_count);
                println!("⏱️  Uptime: {}s", device.context().uptime_seconds);
                
                if let Some(timeout) = device.get_current_timeout().await {
                    println!("⏰ Timeout: {:?}", timeout);
                }
            }
            Err(e) => {
                println!("❌ Error processing event: {:?}", e);
                println!("📊 Staying in state: {:?}", device.current_state());
            }
        }
        
        println!(); // Empty line for readability
        
        // Small delay to make the simulation more realistic
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Demonstrate timeout handling
    println!("🕐 Demonstrating timeout behavior...");
    device.init(DeviceState::Standby).await?;
    
    if let Some(timeout) = device.get_current_timeout().await {
        println!("⏰ Current timeout: {:?}", timeout);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    match simulate_device_operation().await {
        Ok(()) => println!("Device simulation completed successfully!"),
        Err(e) => println!("Error during device simulation: {:?}", e),
    }

    Ok(())
}