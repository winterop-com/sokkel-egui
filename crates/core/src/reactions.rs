//! The reactions under a message or a comment, as sokkel-ui's `ReactionBar` keeps them: each an
//! emoji with its name, how many gave it and whether the viewer did, and `toggle_reaction`, which
//! adds the viewer's or takes it back.

use serde::{Deserialize, Serialize};

/// One reaction: the emoji as its id, its name, how many gave it, and whether the viewer did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    pub id: String,
    pub label: String,
    pub count: i64,
    pub mine: bool,
}

/// The reactions with the viewer's `emoji`, named `name`, added or taken back: a reaction no one
/// gives any more is dropped, and a new one is added at the end under the emoji as its id.
pub fn toggle_reaction(reactions: &[Reaction], emoji: &str, name: &str) -> Vec<Reaction> {
    let Some(at) = reactions.iter().position(|one| one.id == emoji) else {
        let mut next = reactions.to_vec();
        next.push(Reaction {
            id: emoji.to_owned(),
            label: name.to_owned(),
            count: 1,
            mine: true,
        });
        return next;
    };
    let reaction = &reactions[at];
    let mine = !reaction.mine;
    let count = reaction.count + if mine { 1 } else { -1 };
    if count <= 0 {
        return reactions
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != at)
            .map(|(_, one)| one.clone())
            .collect();
    }
    reactions
        .iter()
        .enumerate()
        .map(|(index, one)| {
            if index == at {
                Reaction {
                    count,
                    mine,
                    ..one.clone()
                }
            } else {
                one.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reaction_is_given_and_taken_back() {
        let given = toggle_reaction(&[], "+1", "wave");
        assert_eq!(given.len(), 1);
        assert!(given[0].mine);
        assert!(toggle_reaction(&given, "+1", "wave").is_empty());
    }
}
