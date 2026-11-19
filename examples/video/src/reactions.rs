//! A comment's reactions as sokkel-ui's `ReactionBar` toggles them: yours added, or taken back
//! when it is already yours; a reaction no one gives any more is dropped, and a new one is added at
//! the end under the emoji as its id. Carried here until the core has `toggleReaction`.

use crate::data::CommentReaction;

/// The reactions with yours to `emoji`, named `name`, added or taken back.
pub fn toggle_reaction(reactions: &[CommentReaction], emoji: &str, name: &str) -> Vec<CommentReaction> {
    let Some(at) = reactions.iter().position(|one| one.id == emoji) else {
        let mut next = reactions.to_vec();
        next.push(CommentReaction {
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
                CommentReaction {
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
