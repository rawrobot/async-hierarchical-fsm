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
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        _context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOn => Response::Transition(DeviceState::Standby),
            _ => Response::Error("Device is off".to_string()),
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {}
}

struct StandbyState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for StandbyState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 25;
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        _context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOff => Response::Transition(DeviceState::Off),
            DeviceEvent::Activate => Response::Transition(DeviceState::Active),
            DeviceEvent::ErrorOccurred => Response::Transition(DeviceState::Error),
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {}

    async fn get_timeout(&self, _context: &DeviceContext) -> Option<Duration> {
        Some(Duration::from_secs(60)) // Auto-shutdown after 1 minute
    }
}

struct ActiveState;

#[async_trait]
impl Stateful<DeviceState, DeviceContext, DeviceEvent> for ActiveState {
    async fn on_enter(&mut self, context: &mut DeviceContext) -> Response<DeviceState> {
        context.power_level = 100;
        Response::Handled
    }

    async fn on_event(
        &mut self,
        event: &DeviceEvent,
        _context: &mut DeviceContext,
    ) -> Response<DeviceState> {
        match event {
            DeviceEvent::PowerOff => Response::Transition(DeviceState::Off),
            DeviceEvent::Deactivate => Response::Transition(DeviceState::Standby),
            DeviceEvent::ErrorOccurred => Response::Transition(DeviceState::Error),
            _ => Response::Handled,
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {}

    async fn get_timeout(&self, context: &DeviceContext) -> Option<Duration> {
        // Shorter timeout if many errors occurred
        if context.error_count > 3 {
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
        context.power_level = 10; // Minimal power

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
                    Response::Transition(DeviceState::Standby)
                } else {
                    Response::Transition(DeviceState::Off) // Too many errors, shut down
                }
            }
            DeviceEvent::PowerOff => Response::Transition(DeviceState::Off),
            _ => Response::Handled, // Ignore other events in error state
        }
    }

    async fn on_exit(&mut self, _context: &mut DeviceContext) {}

    async fn get_timeout(&self, _context: &DeviceContext) -> Option<Duration> {
        Some(Duration::from_secs(5)) // Quick timeout in error state
    }
}

fn create_device_fsm() -> StateMachine<DeviceState, DeviceContext, DeviceEvent> {
    let context = DeviceContext::new();

    StateMachineBuilder::new(context)
        .state(DeviceState::Off, OffState)
        .state(DeviceState::Standby, StandbyState)
        .state(DeviceState::Active, ActiveState)
        .state(DeviceState::Error, ErrorState)
        .build()
}

#[tokio::test]
async fn test_device_lifecycle() {
    let mut device = create_device_fsm();

    // Start in Off state
    device.init(DeviceState::Off).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Off));
    assert_eq!(device.context().power_level, 0);

    // Power on -> Standby
    device.process_event(&DeviceEvent::PowerOn).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Standby));
    assert_eq!(device.context().power_level, 25);

    // Activate -> Active
    device.process_event(&DeviceEvent::Activate).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Active));
    assert_eq!(device.context().power_level, 100);

    // Error occurs -> Error
    device
        .process_event(&DeviceEvent::ErrorOccurred)
        .await
        .unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Error));
    assert_eq!(device.context().error_count, 1);
    assert_eq!(device.context().power_level, 10);

    // Reset -> Standby (since error_count < 5)
    device.process_event(&DeviceEvent::Reset).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Standby));
    assert_eq!(device.context().power_level, 25);

    // Power off -> Off
    device.process_event(&DeviceEvent::PowerOff).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Off));
    assert_eq!(device.context().power_level, 0);
}

#[tokio::test]
async fn test_error_recovery_limit() {
    let mut device = create_device_fsm();
    device.init(DeviceState::Standby).await.unwrap();

    // Cause multiple errors
    for i in 1..=5 {
        device
            .process_event(&DeviceEvent::ErrorOccurred)
            .await
            .unwrap();
        assert_eq!(device.current_state(), Some(DeviceState::Error));
        assert_eq!(device.context().error_count, i);

        if i < 5 {
            // Should reset to Standby
            device.process_event(&DeviceEvent::Reset).await.unwrap();
            assert_eq!(device.current_state(), Some(DeviceState::Standby));
        }
    }

    // After 5 errors, reset should go to Off instead of Standby
    device.process_event(&DeviceEvent::Reset).await.unwrap();
    assert_eq!(device.current_state(), Some(DeviceState::Off));
}

#[tokio::test]
async fn test_timeout_behavior() {
    let mut device = create_device_fsm();
    device.init(DeviceState::Off).await.unwrap();

    // Off state should have no timeout
    assert_eq!(device.get_current_timeout().await, None);

    // Standby should have 60 second timeout
    device.process_event(&DeviceEvent::PowerOn).await.unwrap();
    assert_eq!(
        device.get_current_timeout().await,
        Some(Duration::from_secs(60))
    );

    // Active should have 30 second timeout initially
    device.process_event(&DeviceEvent::Activate).await.unwrap();
    assert_eq!(
        device.get_current_timeout().await,
        Some(Duration::from_secs(30))
    );

    // After multiple errors, Active timeout should be shorter
    for _ in 0..4 {
        device
            .process_event(&DeviceEvent::ErrorOccurred)
            .await
            .unwrap();
        device.process_event(&DeviceEvent::Reset).await.unwrap();
        device.process_event(&DeviceEvent::Activate).await.unwrap();
    }

    assert_eq!(
        device.get_current_timeout().await,
        Some(Duration::from_secs(10))
    );

    // Error state should have 5 second timeout
    device
        .process_event(&DeviceEvent::ErrorOccurred)
        .await
        .unwrap();
    assert_eq!(
        device.get_current_timeout().await,
        Some(Duration::from_secs(5))
    );
}

#[tokio::test]
async fn test_invalid_transitions() {
    let mut device = create_device_fsm();
    device.init(DeviceState::Off).await.unwrap();

    // Try invalid events in Off state
    let result = device.process_event(&DeviceEvent::Activate).await;
    assert!(result.is_err());
    assert_eq!(device.current_state(), Some(DeviceState::Off)); // Should stay in Off

    let result = device.process_event(&DeviceEvent::Deactivate).await;
    assert!(result.is_err());
    assert_eq!(device.current_state(), Some(DeviceState::Off));
}

#[tokio::test]
async fn test_concurrent_operations() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let device = Arc::new(Mutex::new(create_device_fsm()));
    let device_clone = Arc::clone(&device);

    // Initialize device
    {
        let mut d = device.lock().await;
        d.init(DeviceState::Standby).await.unwrap();
    }

    // Spawn multiple tasks that interact with the device
    let task1 = tokio::spawn(async move {
        let mut d = device_clone.lock().await;
        for _ in 0..10 {
            let _ = d.process_event(&DeviceEvent::Activate).await;
            let _ = d.process_event(&DeviceEvent::Deactivate).await;
        }
    });

    let device_clone2 = Arc::clone(&device);
    let task2 = tokio::spawn(async move {
        let d = device_clone2.lock().await;
        for _ in 0..10 {
            let _ = d.get_current_timeout().await;
            let _ = d.current_state();
        }
    });

    // Wait for tasks to complete
    let _ = tokio::join!(task1, task2);

    // Device should still be in a valid state
    let d = device.lock().await;
    assert!(d.current_state().is_some());
}

// Test with channels for event-driven architecture
#[tokio::test]
async fn test_event_driven_architecture() {
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    let mut device = create_device_fsm();
    device.init(DeviceState::Off).await.unwrap();

    let (event_tx, mut event_rx) = mpsc::channel(32);
    let (state_tx, mut state_rx) = mpsc::channel(32);

    // Spawn event processor
    let processor = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if let Ok(()) = device.process_event(&event).await {
                let current_state = device.current_state();
                if state_tx.send(current_state).await.is_err() {
                    break;
                }
            }
        }
    });

    // Send events
    event_tx.send(DeviceEvent::PowerOn).await.unwrap();
    event_tx.send(DeviceEvent::Activate).await.unwrap();
    event_tx.send(DeviceEvent::ErrorOccurred).await.unwrap();
    event_tx.send(DeviceEvent::Reset).await.unwrap();

    // Collect state changes
    let mut states = Vec::new();
    for _ in 0..4 {
        if let Ok(Some(state)) = timeout(Duration::from_millis(100), state_rx.recv()).await {
            states.push(state);
        }
    }

    // Close channels and wait for processor
    drop(event_tx);
    let _ = processor.await;

    // Verify state sequence
    assert_eq!(
        states,
        vec![
            Some(DeviceState::Standby),
            Some(DeviceState::Active),
            Some(DeviceState::Error),
            Some(DeviceState::Standby),
        ]
    );
}

// Stress test
#[tokio::test]
async fn test_stress() {
    let mut device = create_device_fsm();
    device.init(DeviceState::Off).await.unwrap();

    let events = vec![
        DeviceEvent::PowerOn,
        DeviceEvent::Activate,
        DeviceEvent::Deactivate,
        DeviceEvent::ErrorOccurred,
        DeviceEvent::Reset,
        DeviceEvent::PowerOff,
    ];

    let start = std::time::Instant::now();

    // Process many random events
    for i in 0..1000 {
        let event = &events[i % events.len()];
        let _ = device.process_event(event).await; // Ignore errors for stress test
    }

    let duration = start.elapsed();
    println!("Stress test: processed 1000 events in {:?}", duration);

    // Device should still be operational
    assert!(device.current_state().is_some());
    let _ = device.get_current_timeout().await;
}
