//! Loops a new edge would close, after sokkel-ui-graph's `cycles`.

use std::collections::{HashMap, VecDeque};

/// A step as a loop is told from: its code and the codes of the steps it needs. A
/// `PipelineStep` and a `RunStep` are one.
pub trait StepNeeds {
    fn code(&self) -> &str;
    fn needs(&self) -> &[String];
}

impl<S: AsRef<str>> StepNeeds for (S, Vec<String>) {
    fn code(&self) -> &str {
        self.0.as_ref()
    }

    fn needs(&self) -> &[String] {
        &self.1
    }
}

/// The loop an edge from `source` to `target` would close: the codes from `target` along the steps
/// that lead on from it to `source`, the shortest such way, which the new edge then joins back to
/// `target`. `[source]` alone when the edge would join a step to itself, and `None` when it closes
/// none.
pub fn cycle_path<S: StepNeeds>(steps: &[S], source: &str, target: &str) -> Option<Vec<String>> {
    if source == target {
        return Some(vec![source.to_owned()]);
    }
    // what comes after each step, in the order the steps are given
    let mut after: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in steps {
        for need in step.needs() {
            after.entry(need.as_str()).or_default().push(step.code());
        }
    }
    let mut from: HashMap<&str, Option<&str>> = HashMap::from([(target, None)]);
    let mut queue = VecDeque::from([target]);
    while let Some(at) = queue.pop_front() {
        if at == source {
            let mut path = Vec::new();
            let mut back = Some(at);
            while let Some(code) = back {
                path.push(code.to_owned());
                back = from.get(code).copied().flatten();
            }
            path.reverse();
            return Some(path);
        }
        for &next in after.get(at).into_iter().flatten() {
            if from.contains_key(next) {
                continue;
            }
            from.insert(next, Some(at));
            queue.push_back(next);
        }
    }
    None
}

/// Whether an edge from `source` to `target` would close a loop: whether `target` already leads to
/// `source`.
pub fn would_cycle<S: StepNeeds>(steps: &[S], source: &str, target: &str) -> bool {
    cycle_path(steps, source, target).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> Vec<(&'static str, Vec<String>)> {
        vec![
            ("fetch", vec![]),
            ("transform", vec!["fetch".into()]),
            ("validate", vec!["transform".into()]),
            ("upload", vec!["validate".into()]),
        ]
    }

    #[test]
    fn an_edge_back_up_a_chain_closes_the_loop_through_it() {
        assert_eq!(
            cycle_path(&chain(), "upload", "transform"),
            Some(vec!["transform".into(), "validate".into(), "upload".into()])
        );
    }

    #[test]
    fn an_edge_down_a_chain_closes_none() {
        assert_eq!(cycle_path(&chain(), "fetch", "upload"), None);
        assert!(!would_cycle(&chain(), "transform", "upload"));
    }

    #[test]
    fn an_edge_to_itself_is_a_loop_of_one() {
        assert_eq!(cycle_path(&chain(), "fetch", "fetch"), Some(vec!["fetch".into()]));
    }
}
