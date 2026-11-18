//! The kinds of step a pipeline is built from, as a palette or a picker offers them, after
//! sokkel-ui-graph's `blocks`.

use sokkel_egui::{ChipTone, Icon};

/// A kind of step a pipeline can be built from, as a palette or a picker offers it.
#[derive(Clone, Debug)]
pub struct PipelineBlock {
    /// The block's key, drawn in the code face: the kind a step made from it carries.
    pub kind: String,
    /// A human title; without one, the kind is the title.
    pub name: Option<String>,
    pub description: Option<String>,
    /// The id of the group it is listed under.
    pub group: String,
    pub icon: Option<Icon>,
    /// A short word at the row's end, such as what sort of block it is, in a chip's tone.
    pub badge: Option<BlockBadge>,
    /// Other words a search finds the block by.
    pub keywords: Vec<String>,
}

impl PipelineBlock {
    pub fn new(kind: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            name: None,
            description: None,
            group: group.into(),
            icon: None,
            badge: None,
            keywords: Vec::new(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn badge(mut self, badge: BlockBadge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords.extend(words.into_iter().map(Into::into));
        self
    }

    /// The badge, or `None` for none or an empty one.
    pub fn shown_badge(&self) -> Option<&BlockBadge> {
        self.badge.as_ref().filter(|badge| !badge.label.is_empty())
    }
}

/// A block's badge: its words in a chip's tone, and what pointing at it says.
#[derive(Clone, Debug)]
pub struct BlockBadge {
    pub label: String,
    pub tone: ChipTone,
    pub title: Option<String>,
}

impl BlockBadge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: ChipTone::Neutral,
            title: None,
        }
    }

    pub fn tone(mut self, tone: ChipTone) -> Self {
        self.tone = tone;
        self
    }
}

/// A heading blocks are listed under, in the order given.
#[derive(Clone, Debug)]
pub struct BlockGroup {
    pub id: String,
    pub label: String,
}

impl BlockGroup {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// What a block dragged from a palette carries onto a canvas: its kind. A canvas whose editing
/// takes dropped items answers `GraphEvent::DropItem` when one is let go over it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDragItem(pub String);

/// Whether a block matches what was typed: every word found in its kind, name, description, badge
/// or keywords.
pub fn block_matches(block: &PipelineBlock, query: &str) -> bool {
    let lowered = query.trim().to_lowercase();
    let words: Vec<&str> = lowered.split_whitespace().collect();
    if words.is_empty() {
        return true;
    }
    let mut text = vec![
        block.kind.as_str(),
        block.name.as_deref().unwrap_or_default(),
        block.description.as_deref().unwrap_or_default(),
        block.shown_badge().map_or("", |badge| badge.label.as_str()),
    ];
    text.extend(block.keywords.iter().map(String::as_str));
    let text = text.join(" ").to_lowercase();
    words.iter().all(|word| text.contains(word))
}

/// The blocks that match a query, under their groups in the groups' order; a group with none is
/// left out. Each block is given by its index in `blocks`.
pub fn group_blocks<'a>(
    blocks: &[PipelineBlock],
    groups: &'a [BlockGroup],
    query: &str,
) -> Vec<(&'a BlockGroup, Vec<usize>)> {
    groups
        .iter()
        .map(|group| {
            let inside = blocks
                .iter()
                .enumerate()
                .filter(|(_, block)| block.group == group.id && block_matches(block, query))
                .map(|(k, _)| k)
                .collect::<Vec<_>>();
            (group, inside)
        })
        .filter(|(_, inside)| !inside.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_typed_must_be_found() {
        let block = PipelineBlock::new("transform", "shape").description("Reshapes records with a script.");
        assert!(block_matches(&block, ""));
        assert!(block_matches(&block, "TRANS script"));
        assert!(!block_matches(&block, "trans upload"));
    }

    #[test]
    fn groups_keep_their_order_and_leave_out_the_empty() {
        let blocks = [
            PipelineBlock::new("upload", "write"),
            PipelineBlock::new("fetch", "read"),
            PipelineBlock::new("notify", "write"),
        ];
        let groups = [
            BlockGroup::new("read", "Read"),
            BlockGroup::new("shape", "Shape"),
            BlockGroup::new("write", "Write"),
        ];
        let listed = group_blocks(&blocks, &groups, "");
        let ids: Vec<(&str, Vec<usize>)> = listed.iter().map(|(g, b)| (g.id.as_str(), b.clone())).collect();
        assert_eq!(ids, vec![("read", vec![1]), ("write", vec![0, 2])]);
    }
}
