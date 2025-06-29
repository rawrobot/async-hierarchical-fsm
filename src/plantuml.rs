//! PlantUML diagram generation

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

/// Generate PlantUML diagram from transition log
pub fn generate_plantuml<S>(
    transition_log: &HashSet<(S, S)>,
    current_state: &Option<S>,
    superstate_fn: &Option<Box<dyn Fn(&S) -> Option<S> + Send + Sync>>,
) -> String
where
    S: Clone + Debug + Eq + Hash,
{
    let mut plantuml = String::new();
    plantuml.push_str("@startuml\n");
    plantuml.push_str("skinparam state {\n");
    plantuml.push_str("  BackgroundColor<<Current>> YellowGreen\n");
    plantuml.push_str("}\n\n");

    // Add hierarchy relationships first
    if let Some(ref superstate_fn) = superstate_fn {
        let mut states: HashSet<S> = HashSet::new();
        
        // Collect all states from transitions
        for (from, to) in transition_log {
            states.insert(from.clone());
            states.insert(to.clone());
        }

        // Add parent relationships
        for state in &states {
            if let Some(parent) = superstate_fn(state) {
                plantuml.push_str(&format!("{:?} -up-> {:?} : parent\n", state, parent));
            }
        }
        
        if !states.is_empty() {
            plantuml.push('\n');
        }
    }

    // Add transitions
    for (from, to) in transition_log {
        plantuml.push_str(&format!("{:?} --> {:?}\n", from, to));
    }

    // Mark current state
    if let Some(ref current) = current_state {
        plantuml.push_str(&format!("state {:?} <<Current>>\n", current));
    }

    plantuml.push_str("@enduml\n");
    plantuml
}
