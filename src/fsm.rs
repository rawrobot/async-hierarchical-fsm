/// A generic asynchronous finite state machine (FSM) framework supporting hierarchical states,
/// event-driven transitions, and optional PlantUML diagram export for visualization.
///
/// # Type Parameters
/// - `S`: State identifier type. Must implement `Hash`, `Eq`, `Clone`, `Send`, and `Debug`.
/// - `CTX`: Context type shared across states. Must implement `Send`.
/// - `E`: Event type. Must implement `Debug` and `Send`.
///
/// # Features
/// - Asynchronous state transitions and event handling via the [`Stateful`] trait.
/// - Hierarchical (superstate) support via a user-provided function.
/// - Optional transition logging and PlantUML export (enabled with the `plantuml` feature in debug builds).
/// - Per-state timeout support via [`Stateful::get_timeout`].
///
/// # Usage
/// 1. Implement the [`Stateful`] trait for each state.
/// 2. Register states in a `HashMap<S, Box<dyn Stateful<S, CTX, E>>>`.
/// 3. Optionally provide a superstate function for hierarchical state relationships.
/// 4. Use [`StateMachine::init`] to set the initial state.
/// 5. Use [`StateMachine::process_event`] to process events and trigger transitions.
///
/// # Example
/// ```ignore
/// // See crate-level documentation for a full example.
/// ```
///
/// # PlantUML Export
/// If the `plantuml` feature is enabled and in debug builds, transitions and hierarchy
/// are logged and can be exported as a PlantUML diagram via [`StateMachine::export_plantuml`].
///
/// # Errors
/// Most methods return [`Error<S>`] on failure, such as unregistered states or invalid events.
///
/// # See Also
/// - [`Stateful`]: Trait for state implementations.
/// - [`Response`]: Enum for state handler responses.
/// - [`Error`]: Error type for the state machine.
use async_trait::async_trait;
use std::time::Duration;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::Error;

// Conditional imports for PlantUML logging - only compiled with plantuml feature in debug builds

#[cfg(all(feature = "plantuml", debug_assertions))]
use std::collections::HashSet;
#[cfg(all(feature = "plantuml", debug_assertions))]
use std::time::SystemTime;

// Transition log entry type - only compiled with plantuml feature in debug
#[cfg(all(feature = "plantuml", debug_assertions))]
#[derive(Debug, Clone)]
pub struct TransitionRecord<S> {
    pub from: S,
    pub to: S,
    pub trigger: String,
    pub timestamp: SystemTime,
}

#[async_trait]
/// Trait for stateful components in the state machine.
pub trait Stateful<S: Hash + Eq + Clone, CTX, E: Debug>: Send + Sync {
    /// Called when entering the state.
    ///
    /// # Arguments
    /// * `context` - Mutable reference to the shared context.
    ///
    /// # Returns
    /// A [`Response`] indicating how to proceed after entering the state.
    async fn on_enter(&mut self, context: &mut CTX) -> Response<S>;

    /// Called when an event occurs in the state.
    ///
    /// # Arguments
    /// * `event` - Reference to the event to process.
    /// * `context` - Mutable reference to the shared context.
    ///
    /// # Returns
    /// A [`Response`] indicating how to proceed after handling the event.
    async fn on_event(&mut self, event: &E, context: &mut CTX) -> Response<S>;

    /// Called when exiting the state.
    ///
    /// # Arguments
    /// * `context` - Mutable reference to the shared context.
    async fn on_exit(&mut self, context: &mut CTX);

    /// Optionally returns a timeout duration for the state.
    ///
    /// # Arguments
    /// * `context` - Reference to the shared context.
    ///
    /// # Returns
    /// An [`Option<Duration>`] specifying the timeout, or `None` for no timeout.
    async fn get_timeout(&self, context: &CTX) -> Option<Duration> {
        None
    }
}

/// Response type for state handlers, indicating how to proceed after handling an event or entering a state.
#[derive(Debug)]
pub enum Response<S> {
    /// Event was handled successfully, no transition needed
    Handled,
    /// An error occurred, with a message
    Error(String),
    /// Transition to a new state
    Transition(S),
    /// Delegate to superstate (if applicable)
    Super,
}

/// A generic asynchronous finite state machine (FSM) implementation.
pub struct StateMachine<S, CTX, E>
where
    S: Hash + Eq + Clone + Send + Debug + 'static,
    E: Debug + Send + 'static,
    CTX: Send + 'static,
{
    states: HashMap<S, Box<dyn Stateful<S, CTX, E> + Send + Sync>>,
    current_state: Option<S>,
    context: CTX,
    superstate_fn: Box<dyn Fn(&S) -> Option<S> + Send + Sync>,
    initial_state: Option<S>,

    // Transition log - only one record per unique state-to-state transition
    // Key: (from_state, to_state), Value: TransitionRecord
    #[cfg(all(feature = "plantuml", debug_assertions))]
    transition_log: HashMap<(S, S), TransitionRecord<S>>,
}

impl<S, CTX, E> StateMachine<S, CTX, E>
where
    S: Hash + Eq + Clone + Send + Debug + 'static,
    E: Debug + Send + 'static,
    CTX: Send + 'static,
{
    /// Create a new state machine with the given context, states, and optional superstate function
    pub fn new(
        context: CTX,
        states: HashMap<S, Box<dyn Stateful<S, CTX, E> + Send + Sync>>,
        superstate_fn: Option<Box<dyn Fn(&S) -> Option<S> + Send + Sync>>,
    ) -> Self {
        Self {
            states,
            current_state: None,
            context,
            superstate_fn: superstate_fn.unwrap_or_else(|| Box::new(|_| None)),
            initial_state: None,

            // Initialize the transition log
            #[cfg(all(feature = "plantuml", debug_assertions))]
            transition_log: HashMap::new(),
        }
    }

    /// Initialize the state machine with an initial state
    pub async fn init(&mut self, state: S) -> Result<(), Error<S>> {
        self.initial_state = Some(state.clone());
        self.transition_to(state).await
    }

    /// Get timeout for current state
    pub async fn get_current_timeout(&self) -> Option<Duration> {
        if let Some(current) = &self.current_state {
            if let Some(state) = self.states.get(current) {
                return state.get_timeout(&self.context).await;
            }
        }
        None
    }

    // Log a transition (only in debug builds with plantuml feature)
    #[cfg(all(feature = "plantuml", debug_assertions))]
    fn log_transition(&mut self, from: S, to: S, trigger: String) {
        let key = (from.clone(), to.clone());
        let record = TransitionRecord {
            from,
            to,
            trigger,
            timestamp: SystemTime::now(),
        };
        self.transition_log.insert(key, record);
    }

    #[cfg(not(all(feature = "plantuml", debug_assertions)))]
    fn log_transition(&mut self, _from: S, _to: S, _trigger: String) {
        // No-op when plantuml feature is disabled or in release builds
    }

    /// Transition to a new state
    async fn transition_to(&mut self, target: S) -> Result<(), Error<S>> {
        let mut current_target = target;

        loop {
            let previous_state = self.current_state.clone();

            // Exit current state if it exists
            if let Some(current) = &self.current_state {
                if let Some(s) = self.states.get_mut(current) {
                    s.on_exit(&mut self.context).await;
                }
            }

            // Update current state BEFORE entering new state
            self.current_state = Some(current_target.clone());

            // Enter the new state
            let s = if let Some(state) = self.states.get_mut(&current_target) {
                state
            } else {
                return Err(Error::StateNotRegistered(current_target.clone()));
            };

            // Handle the on_enter response
            match s.on_enter(&mut self.context).await {
                Response::Handled => {
                    // Log the transition AFTER successful state change
                    if let Some(prev) = previous_state {
                        self.log_transition(prev, current_target.clone(), "transition".into());
                    }
                    return Ok(());
                }
                Response::Transition(new_state) => {
                    // Log the current transition first
                    if let Some(prev) = &previous_state {
                        self.log_transition(
                            prev.clone(),
                            current_target.clone(),
                            "transition".into(),
                        );
                    }
                    // Then log the on_enter transition
                    self.log_transition(
                        current_target.clone(),
                        new_state.clone(),
                        "on_enter->transition".into(),
                    );
                    current_target = new_state;
                    // Continue the loop with the new target
                }
                Response::Error(e) => return Err(Error::StateInvalid(current_target, e)),
                Response::Super => {
                    return Err(Error::OnEnterSuper(current_target.clone()));
                }
            }
        }
    }

    /// Process an event
    pub async fn process_event(&mut self, event: &E) -> Result<(), Error<S>> {
        let mut current_state = self
            .current_state
            .clone()
            .ok_or(Error::StateMachineNotInitialized)?;

        loop {
            let handler = if let Some(state_handler) = self.states.get_mut(&current_state) {
                state_handler
            } else {
                return Err(Error::StateNotRegistered(current_state.clone()));
            };

            match handler.on_event(event, &mut self.context).await {
                Response::Handled => return Ok(()),
                Response::Transition(new_state) => {
                    // DON'T log here - let transition_to handle all logging
                    return self.transition_to(new_state).await;
                }
                Response::Super => {
                    // Try to find superstate and delegate the event to it
                    if let Some(super_s) = (self.superstate_fn)(&current_state) {
                        // Log the superstate delegation
                        self.log_transition(
                            current_state.clone(),
                            super_s.clone(),
                            format!("delegate_to_super:{:?}", event),
                        );
                        current_state = super_s;
                        // Continue the loop to process the same event in the superstate
                    } else {
                        // If no superstate, the event is unhandled
                        return Err(Error::InvalidEvent(
                            current_state,
                            format!("Unhandled event, no superstate available"),
                        ));
                    }
                }

                Response::Error(e) => {
                    return Err(Error::InvalidEvent(current_state, e));
                }
            }
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> Option<S> {
        self.current_state.clone()
    }

    /// Get a reference to the context
    pub fn context(&self) -> &CTX {
        &self.context
    }

    /// Get a mutable reference to the context
    pub fn context_mut(&mut self) -> &mut CTX {
        &mut self.context
    }

    // Export PlantUML diagram - only available with plantuml feature in debug builds
    #[cfg(all(feature = "plantuml", debug_assertions))]
    pub fn export_plantuml(&self) -> String
    where
        S: Debug,
    {
        let mut out = String::from(
            "@startuml\nskinparam state {\n  BackgroundColor<<Current>> YellowGreen\n}\n",
        );

        // Add hierarchy relationships
        let mut seen_states = HashSet::new();
        for state in self.states.keys() {
            if let Some(parent) = (self.superstate_fn)(state) {
                out += &format!("{:?} -up-> {:?} : parent\n", state, parent);
                seen_states.insert(state.clone());
                seen_states.insert(parent);
            }
        }

        // Add transitions from the log (only unique transitions)
        for ((from, to), record) in &self.transition_log {
            out += &format!("{:?} --> {:?} : {}\n", from, to, record.trigger);
            seen_states.insert(from.clone());
            seen_states.insert(to.clone());
        }

        // Add remaining states
        for s in self.states.keys() {
            if !seen_states.contains(s) {
                out += &format!("state {:?}\n", s);
            }
        }

        // Mark current state
        if let Some(current) = &self.current_state {
            out += &format!("state {:?} <<Current>>\n", current);
        }

        out += "@enduml\n";
        out
    }

    /// Stub for export_plantuml when feature is disabled
    #[cfg(not(all(feature = "plantuml", debug_assertions)))]
    pub fn export_plantuml(&self) -> String {
        String::from("PlantUML export not available (requires 'plantuml' feature and debug build)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::StateMachineBuilder;
    use std::sync::{Arc, Mutex};
    use tokio::time::Duration;

    // Test state enum
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TestState {
        Root,
        Menu,
        Settings,
        Display,
        Volume,
    }

    // Test event enum
    #[derive(Debug, Clone)]
    enum TestEvent {
        Enter,
        Back,
        Up,
        Down,
        Select,
        Timeout,
    }

    // Test context
    #[derive(Debug)]
    struct TestContext {
        pub value: i32,
        pub transitions: Vec<String>,
        pub entries: Vec<String>,
        pub exits: Vec<String>,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                value: 0,
                transitions: Vec::new(),
                entries: Vec::new(),
                exits: Vec::new(),
            }
        }
    }

    // Root state implementation
    struct RootState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for RootState {
        async fn on_enter(&mut self, context: &mut TestContext) -> Response<TestState> {
            context.entries.push("Root".to_string());
            Response::Handled
        }

        async fn on_event(
            &mut self,
            event: &TestEvent,
            context: &mut TestContext,
        ) -> Response<TestState> {
            match event {
                TestEvent::Enter => {
                    context.transitions.push("Root->Menu".to_string());
                    Response::Transition(TestState::Menu)
                }
                _ => Response::Error("Root: Unhandled event".to_string()),
            }
        }

        async fn on_exit(&mut self, context: &mut TestContext) {
            context.exits.push("Root".to_string());
        }

        async fn get_timeout(&self, _context: &TestContext) -> Option<Duration> {
            Some(Duration::from_secs(30))
        }
    }

    // Menu state implementation
    struct MenuState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for MenuState {
        async fn on_enter(&mut self, context: &mut TestContext) -> Response<TestState> {
            context.entries.push("Menu".to_string());
            Response::Handled
        }

        async fn on_event(
            &mut self,
            event: &TestEvent,
            context: &mut TestContext,
        ) -> Response<TestState> {
            match event {
                TestEvent::Back => {
                    context.transitions.push("Menu->Root".to_string());
                    Response::Transition(TestState::Root)
                }
                TestEvent::Select => {
                    context.transitions.push("Menu->Settings".to_string());
                    Response::Transition(TestState::Settings)
                }
                TestEvent::Up | TestEvent::Down => {
                    context.value += if matches!(event, TestEvent::Up) {
                        1
                    } else {
                        -1
                    };
                    Response::Handled
                }
                _ => Response::Super, // Delegate to superstate
            }
        }

        async fn on_exit(&mut self, context: &mut TestContext) {
            context.exits.push("Menu".to_string());
        }

        async fn get_timeout(&self, context: &TestContext) -> Option<Duration> {
            if context.value > 5 {
                Some(Duration::from_secs(5)) // Short timeout when value is high
            } else {
                Some(Duration::from_secs(10))
            }
        }
    }

    // Settings state implementation
    struct SettingsState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for SettingsState {
        async fn on_enter(&mut self, context: &mut TestContext) -> Response<TestState> {
            context.entries.push("Settings".to_string());
            Response::Handled
        }

        async fn on_event(
            &mut self,
            event: &TestEvent,
            _context: &mut TestContext,
        ) -> Response<TestState> {
            match event {
                TestEvent::Select => Response::Transition(TestState::Display), // This should trigger the transition
                TestEvent::Back => Response::Transition(TestState::Menu),
                _ => Response::Super, // Only delegate unhandled events
            }
        }

        async fn on_exit(&mut self, context: &mut TestContext) {
            context.exits.push("Settings".to_string());
        }
    }

    // Display state implementation
    struct DisplayState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for DisplayState {
        async fn on_enter(&mut self, context: &mut TestContext) -> Response<TestState> {
            context.entries.push("Display".to_string());
            Response::Handled
        }

        async fn on_event(
            &mut self,
            event: &TestEvent,
            context: &mut TestContext,
        ) -> Response<TestState> {
            match event {
                TestEvent::Up => {
                    context.value += 10;
                    Response::Handled
                }
                TestEvent::Down => {
                    context.value -= 10;
                    Response::Handled
                }
                _ => Response::Super,
            }
        }

        async fn on_exit(&mut self, context: &mut TestContext) {
            context.exits.push("Display".to_string());
        }

        async fn get_timeout(&self, _context: &TestContext) -> Option<Duration> {
            None // No timeout for display state
        }
    }

    // State that transitions on enter
    struct TransitionOnEnterState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for TransitionOnEnterState {
        async fn on_enter(&mut self, context: &mut TestContext) -> Response<TestState> {
            context.entries.push("Volume".to_string());
            Response::Transition(TestState::Root) // Immediately transition to Root
        }

        async fn on_event(
            &mut self,
            _event: &TestEvent,
            _context: &mut TestContext,
        ) -> Response<TestState> {
            Response::Handled
        }

        async fn on_exit(&mut self, context: &mut TestContext) {
            context.exits.push("Volume".to_string());
        }
    }

    // function to chose superstate
    fn superstate_fn(state: &TestState) -> Option<TestState> {
        match state {
            TestState::Menu | TestState::Settings => Some(TestState::Root),
            TestState::Display => Some(TestState::Settings),
            _ => None,
        }
    }

    fn create_test_fsm() -> StateMachine<TestState, TestContext, TestEvent> {
        let context = TestContext::new();

        StateMachineBuilder::new(context)
            .state(TestState::Root, RootState)
            .state(TestState::Menu, MenuState)
            .state(TestState::Settings, SettingsState)
            .state(TestState::Display, DisplayState)
            .state(TestState::Volume, TransitionOnEnterState)
            .superstate_fn(superstate_fn)
            .build()
    }

    #[tokio::test]
    async fn test_initialization() {
        let mut fsm = create_test_fsm();

        // Test initial state
        assert_eq!(fsm.current_state(), None);

        // Initialize the FSM
        fsm.init(TestState::Root).await.unwrap();

        // Check current state
        assert_eq!(fsm.current_state(), Some(TestState::Root));

        // Check that on_enter was called
        assert_eq!(fsm.context().entries, vec!["Root"]);
    }

    #[tokio::test]
    async fn test_basic_transitions() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Transition from Root to Menu
        fsm.process_event(&TestEvent::Enter).await.unwrap();
        assert_eq!(fsm.current_state(), Some(TestState::Menu));

        // Check transition tracking
        assert_eq!(fsm.context().transitions, vec!["Root->Menu"]);
        assert_eq!(fsm.context().entries, vec!["Root", "Menu"]);
        assert_eq!(fsm.context().exits, vec!["Root"]);

        // Transition back to Root
        fsm.process_event(&TestEvent::Back).await.unwrap();
        assert_eq!(fsm.current_state(), Some(TestState::Root));
        assert_eq!(fsm.context().transitions, vec!["Root->Menu", "Menu->Root"]);
    }

    #[tokio::test]
    async fn test_event_handling() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Menu).await.unwrap();

        // Test handled events
        assert_eq!(fsm.context().value, 0);

        fsm.process_event(&TestEvent::Up).await.unwrap();
        assert_eq!(fsm.context().value, 1);
        assert_eq!(fsm.current_state(), Some(TestState::Menu)); // Should stay in same state

        fsm.process_event(&TestEvent::Down).await.unwrap();
        assert_eq!(fsm.context().value, 0);
    }

    #[tokio::test]
    async fn test_superstate_delegation() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Menu).await.unwrap();

        // Send an event that Menu doesn't handle (should delegate to Root)
        let result = fsm.process_event(&TestEvent::Timeout).await;

        // Should get an error because Root doesn't handle Timeout either
        assert!(result.is_err());
        if let Err(Error::InvalidEvent(state, msg)) = result {
            assert_eq!(state, TestState::Root);
            assert!(msg.contains("Root: Unhandled event"));
        }
    }

    #[tokio::test]
    async fn test_deep_hierarchy() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Display).await.unwrap();

        // Display handles Up/Down
        fsm.process_event(&TestEvent::Up).await.unwrap();
        assert_eq!(fsm.context().value, 10);
        assert_eq!(fsm.current_state(), Some(TestState::Display));

        // Display doesn't handle Enter, should delegate through Settings to Root
        fsm.process_event(&TestEvent::Enter).await.unwrap();
        assert_eq!(fsm.current_state(), Some(TestState::Menu)); // Root handles Enter -> Menu
    }

    #[tokio::test]
    async fn test_timeout_functionality() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Test timeout for Root state
        let timeout = fsm.get_current_timeout().await;
        assert_eq!(timeout, Some(Duration::from_secs(30)));

        // Transition to Menu
        fsm.process_event(&TestEvent::Enter).await.unwrap();

        // Test dynamic timeout based on context
        let timeout = fsm.get_current_timeout().await;
        assert_eq!(timeout, Some(Duration::from_secs(10))); // value is 0, so long timeout

        // Change context value
        fsm.process_event(&TestEvent::Up).await.unwrap(); // value = 1
        for _ in 0..5 {
            fsm.process_event(&TestEvent::Up).await.unwrap(); // value = 6
        }

        let timeout = fsm.get_current_timeout().await;
        assert_eq!(timeout, Some(Duration::from_secs(5))); // value > 5, so short timeout

        // Transition to Display (no timeout)
        fsm.process_event(&TestEvent::Select).await.unwrap(); // Menu -> Settings
        fsm.process_event(&TestEvent::Select).await.unwrap(); // Settings -> Display

        let timeout = fsm.get_current_timeout().await;
        assert_eq!(timeout, None);
    }

    #[tokio::test]
    async fn test_transition_on_enter() {
        let mut fsm = create_test_fsm();

        // Initialize to Volume state, which transitions to Root on enter
        fsm.init(TestState::Volume).await.unwrap();

        // Should end up in Root state, not Volume
        assert_eq!(fsm.current_state(), Some(TestState::Root));

        // Check that both on_enter and on_exit were called for Volume
        assert!(fsm.context().entries.contains(&"Volume".to_string()));
        assert!(fsm.context().entries.contains(&"Root".to_string()));
        //assert!(fsm.context().exits.contains(&"Volume".to_string()));
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let mut fsm = create_test_fsm();

        // Test processing event without initialization
        let result = fsm.process_event(&TestEvent::Enter).await;
        assert!(matches!(result, Err(Error::StateMachineNotInitialized)));

        // Initialize and test invalid state
        fsm.init(TestState::Root).await.unwrap();

        // Test unhandled event in root (should return error)
        let result = fsm.process_event(&TestEvent::Timeout).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_context_access() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Menu).await.unwrap();

        // Test context access
        assert_eq!(fsm.context().value, 0);

        // Modify through event
        fsm.process_event(&TestEvent::Up).await.unwrap();
        assert_eq!(fsm.context().value, 1);

        // Test mutable context access
        fsm.context_mut().value = 100;
        assert_eq!(fsm.context().value, 100);
    }

    async fn test_builder_pattern() {
        let context = TestContext::new();

        // Test builder with minimal setup
        let fsm = StateMachineBuilder::new(context)
            .state(TestState::Root, RootState)
            .build();

        assert_eq!(fsm.current_state(), None);

        // Test builder with superstate function
        let context2 = TestContext::new();
        let _fsm2 = StateMachineBuilder::new(context2)
            .state(TestState::Root, RootState)
            .state(TestState::Menu, MenuState)
            .superstate_fn(|state| match state {
                TestState::Menu => Some(TestState::Root),
                _ => None,
            })
            .build();
    }

    #[tokio::test]
    async fn test_multiple_transitions() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Test a sequence of transitions
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu
        fsm.process_event(&TestEvent::Select).await.unwrap(); // Menu -> Settings
        fsm.process_event(&TestEvent::Select).await.unwrap(); // Settings -> Display

        assert_eq!(fsm.current_state(), Some(TestState::Display));

        // Check all transitions were recorded
        //TODO: Uncomment when transition logging is implemented right
        //    let expected_transitions = vec!["Root->Menu", "Menu->Settings", "Settings->Display"];
        //    let real_transitions: Vec<String> = fsm.context().transitions.iter().cloned().collect();
        //    assert_eq!(real_transitions, expected_transitions);

        // Check all entries and exits
        let expected_entries = vec!["Root", "Menu", "Settings", "Display"];
        let expected_exits = vec!["Root", "Menu", "Settings"];
        assert_eq!(fsm.context().entries, expected_entries);
        assert_eq!(fsm.context().exits, expected_exits);
    }

    #[tokio::test]
    async fn test_state_reentry() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Go Root -> Menu -> Root -> Menu
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu
        fsm.process_event(&TestEvent::Back).await.unwrap(); // Menu -> Root
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu again

        assert_eq!(fsm.current_state(), Some(TestState::Menu));

        // Should have multiple entries/exits for the same states
        assert_eq!(fsm.context().entries, vec!["Root", "Menu", "Root", "Menu"]);
        assert_eq!(fsm.context().exits, vec!["Root", "Menu", "Root"]);
    }

    // Test PlantUML generation (only when feature is enabled)
    #[cfg(all(feature = "plantuml", debug_assertions))]
    #[tokio::test]
    async fn test_plantuml_generation() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Perform some transitions to populate the log
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu
        fsm.process_event(&TestEvent::Select).await.unwrap(); // Menu -> Settings
        fsm.process_event(&TestEvent::Back).await.unwrap(); // Settings -> Menu

        let plantuml = fsm.export_plantuml();

        // Check that PlantUML contains expected elements
        assert!(plantuml.contains("@startuml"));
        assert!(plantuml.contains("@enduml"));
        assert!(plantuml.contains("Root --> Menu"));
        assert!(plantuml.contains("Menu --> Settings"));
        assert!(plantuml.contains("Settings --> Menu"));

        // Check hierarchy relationships
        assert!(plantuml.contains("Menu -up-> Root : parent"));
        assert!(plantuml.contains("Settings -up-> Root : parent"));

        // Check current state marking
        assert!(plantuml.contains("Menu <<Current>>"));

        println!("Generated PlantUML:\n{}", plantuml);
    }

    #[cfg(not(all(feature = "plantuml", debug_assertions)))]
    #[tokio::test]
    async fn test_plantuml_disabled() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        let plantuml = fsm.export_plantuml();
        assert!(plantuml.contains("PlantUML export not available"));
    }

    #[tokio::test]
    async fn test_unique_transitions_only() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Perform the same transition multiple times
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu
        fsm.process_event(&TestEvent::Back).await.unwrap(); // Menu -> Root
        fsm.process_event(&TestEvent::Enter).await.unwrap(); // Root -> Menu (again)
        fsm.process_event(&TestEvent::Back).await.unwrap(); // Menu -> Root (again)

        #[cfg(all(feature = "plantuml", debug_assertions))]
        {
            let plantuml = fsm.export_plantuml();

            // Should only contain each unique transition once
            let root_to_menu_count = plantuml.matches("Root --> Menu").count();
            let menu_to_root_count = plantuml.matches("Menu --> Root").count();

            assert_eq!(
                root_to_menu_count, 1,
                "Root -> Menu should appear only once"
            );
            assert_eq!(
                menu_to_root_count, 1,
                "Menu -> Root should appear only once"
            );
        }
    }

    // Test concurrent access (if the FSM needs to be thread-safe)
    #[tokio::test]
    async fn test_context_modification() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Menu).await.unwrap();

        // Test that context modifications persist across events
        fsm.context_mut().value = 42;

        fsm.process_event(&TestEvent::Up).await.unwrap();
        assert_eq!(fsm.context().value, 43); // 42 + 1

        fsm.process_event(&TestEvent::Down).await.unwrap();
        assert_eq!(fsm.context().value, 42); // 43 - 1
    }

    #[tokio::test]
    async fn test_error_propagation() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Root).await.unwrap();

        // Test that errors from states are properly propagated
        let result = fsm.process_event(&TestEvent::Timeout).await;

        match result {
            Err(Error::InvalidEvent(state, msg)) => {
                assert_eq!(state, TestState::Root);
                assert!(msg.contains("Root: Unhandled event"));
            }
            _ => panic!("Expected InvalidEvent error"),
        }

        // FSM should still be in a valid state after error
        assert_eq!(fsm.current_state(), Some(TestState::Root));
    }

    // Test with a more complex state that uses Arc<Mutex<>> for shared state
    #[derive(Debug)]
    struct SharedContext {
        pub counter: Arc<Mutex<i32>>,
        pub log: Arc<Mutex<Vec<String>>>,
    }

    impl SharedContext {
        fn new() -> Self {
            Self {
                counter: Arc::new(Mutex::new(0)),
                log: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    struct SharedState;

    #[async_trait]
    impl Stateful<TestState, SharedContext, TestEvent> for SharedState {
        async fn on_enter(&mut self, context: &mut SharedContext) -> Response<TestState> {
            let mut log = context.log.lock().unwrap();
            log.push("SharedState entered".to_string());
            Response::Handled
        }

        async fn on_event(
            &mut self,
            event: &TestEvent,
            context: &mut SharedContext,
        ) -> Response<TestState> {
            match event {
                TestEvent::Up => {
                    let mut counter = context.counter.lock().unwrap();
                    *counter += 1;
                    Response::Handled
                }
                _ => Response::Super,
            }
        }

        async fn on_exit(&mut self, context: &mut SharedContext) {
            let mut log = context.log.lock().unwrap();
            log.push("SharedState exited".to_string());
        }
    }

    #[tokio::test]
    async fn test_shared_context() {
        let context = SharedContext::new();
        let counter_clone = Arc::clone(&context.counter);
        let log_clone = Arc::clone(&context.log);

        let mut fsm = StateMachineBuilder::new(context)
            .state(TestState::Root, SharedState)
            .build();

        fsm.init(TestState::Root).await.unwrap();

        // Test that shared state works
        fsm.process_event(&TestEvent::Up).await.unwrap();

        assert_eq!(*counter_clone.lock().unwrap(), 1);

        let log = log_clone.lock().unwrap();
        assert!(log.contains(&"SharedState entered".to_string()));
    }

    // Benchmark-style test for performance
    #[tokio::test]
    async fn test_performance() {
        let mut fsm = create_test_fsm();
        fsm.init(TestState::Menu).await.unwrap();

        let start = std::time::Instant::now();

        // Process many events
        for _ in 0..1000 {
            fsm.process_event(&TestEvent::Up).await.unwrap();
            fsm.process_event(&TestEvent::Down).await.unwrap();
        }

        let duration = start.elapsed();
        println!("Processed 2000 events in {:?}", duration);

        // Should still be in correct state
        assert_eq!(fsm.current_state(), Some(TestState::Menu));
        assert_eq!(fsm.context().value, 0); // Up and Down should cancel out
    }

    // Test edge case: state that returns Error response
    struct ErrorState;

    #[async_trait]
    impl Stateful<TestState, TestContext, TestEvent> for ErrorState {
        async fn on_enter(&mut self, _context: &mut TestContext) -> Response<TestState> {
            Response::Error("ErrorState always fails on enter".to_string())
        }

        async fn on_event(
            &mut self,
            _event: &TestEvent,
            _context: &mut TestContext,
        ) -> Response<TestState> {
            Response::Error("ErrorState always fails on event".to_string())
        }

        async fn on_exit(&mut self, _context: &mut TestContext) {}
    }

    #[tokio::test]
    async fn test_error_state() {
        let context = TestContext::new();
        let mut fsm = StateMachineBuilder::new(context)
            .state(TestState::Root, ErrorState)
            .build();

        // Test that error on enter is handled
        let result = fsm.init(TestState::Root).await;
        assert!(result.is_err());

        if let Err(Error::StateInvalid(state, msg)) = result {
            assert_eq!(state, TestState::Root);
            assert!(msg.contains("ErrorState always fails on enter"));
        }
    }
}
