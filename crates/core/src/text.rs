//! Text by role, never by a size and a weight of its own, as sokkel-ui sets it.

use egui::{FontFamily, FontId, RichText};

use crate::Tokens;

/// The roles the tokens define, each a size, a line height, a weight and a tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    PageTitle,
    DetailTitle,
    SectionTitle,
    Head,
    Body,
    Action,
    Label,
    Help,
    Badge,
    Statistic,
    StatisticHero,
    MediaTitle,
    BrandName,
    WelcomeTitle,
    Eyebrow,
    /// A graph node's title, as `text-graph-title`; the node box sets it semibold.
    GraphTitle,
    /// A graph node's code line and an edge's label, as `text-graph-small`.
    GraphSmall,
}

impl TextRole {
    /// Every role, from the page's title down, as sokkel-ui's typography table lists them; the
    /// graph's own two roles are left to the graph.
    pub const ALL: [TextRole; 15] = [
        Self::PageTitle,
        Self::DetailTitle,
        Self::SectionTitle,
        Self::Head,
        Self::Body,
        Self::Action,
        Self::Label,
        Self::Help,
        Self::Badge,
        Self::Statistic,
        Self::StatisticHero,
        Self::MediaTitle,
        Self::BrandName,
        Self::WelcomeTitle,
        Self::Eyebrow,
    ];

    /// The role's name in the tokens.
    pub fn key(self) -> &'static str {
        match self {
            Self::PageTitle => "page-title",
            Self::DetailTitle => "detail-title",
            Self::SectionTitle => "section-title",
            Self::Head => "head",
            Self::Body => "body",
            Self::Action => "action",
            Self::Label => "label",
            Self::Help => "help",
            Self::Badge => "badge",
            Self::Statistic => "statistic",
            Self::StatisticHero => "statistic-hero",
            Self::MediaTitle => "media-title",
            Self::BrandName => "brand-name",
            Self::WelcomeTitle => "welcome-title",
            Self::Eyebrow => "eyebrow",
            Self::GraphTitle => "graph-title",
            Self::GraphSmall => "graph-small",
        }
    }

    /// The role's font: the sans face at its size.
    pub fn font(self, tokens: &Tokens) -> FontId {
        FontId::new(tokens.text(self.key()).size, FontFamily::Proportional)
    }

    /// `text` in this role: its size, line height, tracking, and its weight on the variable face's
    /// `wght` axis, since egui's fonts have no weight of their own.
    pub fn rich(self, tokens: &Tokens, text: impl Into<String>) -> RichText {
        let spec = tokens.text(self.key());
        RichText::new(text)
            .font(FontId::new(spec.size, FontFamily::Proportional))
            .line_height(Some(spec.line_height))
            .extra_letter_spacing(spec.tracking * spec.size)
            .variation("wght", spec.weight)
    }
}
