//! How the player along the foot of a screen grows from its one line, after sokkel-ui-media's
//! `bar-growth`: arithmetic over its own height and width, the heights where things change and the
//! sizes that scale between them.

/// How the bar grows once it is dragged taller than its one line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MediaBarLayout {
    /// What plays, the transport and the volume in a top row over a waveform as wide as the bar,
    /// the cover across both rows.
    #[default]
    Rows,
    /// The large now-playing panel: the cover flush and as tall as the bar, the words and the
    /// waveform over the transport.
    Panel,
    /// A stage centered on the bar, the words over a wide waveform with the transport under it,
    /// between the cover and the volume.
    Stage,
}

/// The one-line bar's height, the least the bar is dragged to.
pub const LINE: f32 = 64.0;
/// The bar's own width from which it lays out on one row, the `@2xl` container size.
pub const WIDE: f32 = 672.0;
/// The fullscreen view's width under which it is laid out for a phone.
pub const PHONE: f32 = 640.0;
/// The height from which the transport takes its larger size.
pub const TALL: f32 = 260.0;
/// The height at which the growth is complete; past it only the room for the words grows.
const GROWN: f32 = 300.0;
const STAGE_CONTROLS: f32 = 36.0;
const STAGE_LEAST: f32 = 360.0;
const STAGE_SIDE: f32 = 176.0;
const ROWS_VOLUME_ROOM: f32 = 600.0;
const PANEL_VOLUME_ROOM: f32 = 480.0;
const PANEL_LEAST: f32 = 360.0;

/// The height from which each layout leaves the one line and grows.
pub fn grows_from(layout: MediaBarLayout) -> f32 {
    match layout {
        MediaBarLayout::Rows | MediaBarLayout::Stage => 96.0,
        MediaBarLayout::Panel => 104.0,
    }
}

/// Whether the layout has left the one line at this height.
pub fn grows(layout: MediaBarLayout, height: f32) -> bool {
    height >= grows_from(layout)
}

/// How far `value` has come from `from` to `to`, from 0 to 1.
pub fn ramp(value: f32, from: f32, to: f32) -> f32 {
    ((value - from) / (to - from)).clamp(0.0, 1.0)
}

pub fn blend(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

/// The whole number `at` of the way from `from` to `to`.
pub fn mix(from: f32, to: f32, at: f32) -> f32 {
    // as JavaScript's Math.round, halves up
    (blend(from, to, at) + 0.5).floor()
}

pub const GLOW_FROM: f32 = 88.0;
pub const GLOW_FULL: f32 = 220.0;
/// The most the visualizer shows behind the words.
pub const GLOW_MOST: f32 = 0.3;

/// How strongly the visualizer shows behind the words at this height.
pub fn glow(height: f32) -> f32 {
    GLOW_MOST * ramp(height, GLOW_FROM, GLOW_FULL).sqrt()
}

/// How much the words say in the room they have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Line,
    Pair,
    Liner,
    Poster,
}

impl Tier {
    /// The room the tier needs, in pixels.
    pub fn room(self) -> f32 {
        match self {
            Self::Line => 22.0,
            Self::Pair => 44.0,
            Self::Liner => 74.0,
            Self::Poster => 120.0,
        }
    }
}

/// The most the words say in `room` pixels, never more than `most`.
pub fn tier_for(room: f32, most: Tier) -> Tier {
    let fits = [Tier::Poster, Tier::Liner, Tier::Pair, Tier::Line]
        .into_iter()
        .find(|tier| room >= tier.room())
        .unwrap_or(Tier::Line);
    fits.min(most)
}

/// What `rows` is at one height and width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowsGrowth {
    pub pad: f32,
    pub wave: f32,
    pub gap: f32,
    pub head: f32,
    pub tier: Tier,
    pub cover: f32,
    pub corner: bool,
    pub compact_volume: bool,
}

pub fn rows_growth(height: f32, width: f32) -> RowsGrowth {
    let narrow = width < WIDE;
    let pad_at = blend(10.0, 14.0, ramp(height, grows_from(MediaBarLayout::Rows), GROWN));
    let wave_at = blend(18.0, 96.0, ramp(height, grows_from(MediaBarLayout::Rows), TALL));
    let gap = 8.0;
    let room = height - 2.0 * pad_at - wave_at - gap;
    let pad = pad_at.round();
    let wave = wave_at.round();
    let cover = if narrow {
        room.floor().min(72.0)
    } else {
        (height - 2.0 * pad).min((width * 0.25).round())
    };
    let compact_volume = width - 24.0 - cover - 16.0 < ROWS_VOLUME_ROOM;
    let most = if narrow {
        Tier::Pair
    } else if compact_volume {
        Tier::Liner
    } else {
        Tier::Poster
    };
    RowsGrowth {
        pad,
        wave,
        gap,
        head: height - 2.0 * pad - wave - gap,
        tier: tier_for(room, most),
        cover,
        corner: room >= 64.0,
        compact_volume,
    }
}

/// The least height at which a wide `rows` bar draws its cover `cover` pixels across.
pub fn rows_height_for(cover: f32) -> f32 {
    if !cover.is_finite() {
        return grows_from(MediaBarLayout::Rows);
    }
    let mut height = grows_from(MediaBarLayout::Rows).max(cover.ceil());
    while rows_growth(height, f32::INFINITY).cover < cover {
        height += 1.0;
    }
    height
}

/// What `panel` is at one height and width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelGrowth {
    pub wave: f32,
    pub gap: f32,
    pub room: f32,
    pub tier: Tier,
    pub cover: f32,
    pub compact_volume: bool,
}

/// The transport's row under the panel, `spacing-shell-foot`.
pub fn panel_growth(height: f32, width: f32, foot: f32) -> PanelGrowth {
    let narrow = width < WIDE;
    let grown = ramp(height, grows_from(MediaBarLayout::Panel), GROWN);
    let room = height - foot - blend(20.0, 60.0, grown) - blend(4.0, 12.0, grown) - 12.0;
    let beside = room.floor().min(112.0);
    let cover = if narrow {
        if beside >= 44.0 { beside } else { 0.0 }
    } else {
        height.min(width - PANEL_LEAST)
    };
    PanelGrowth {
        wave: mix(20.0, 60.0, grown),
        gap: mix(4.0, 12.0, grown),
        room,
        tier: tier_for(room, if narrow { Tier::Liner } else { Tier::Poster }),
        cover,
        compact_volume: width - cover < PANEL_VOLUME_ROOM,
    }
}

/// What `stage` is at one height and width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageGrowth {
    pub pad: f32,
    pub wave: f32,
    pub gap: f32,
    pub room: f32,
    pub tier: Tier,
    pub side: f32,
    pub cover: f32,
    pub under: f32,
    pub compact_volume: bool,
    pub tall: bool,
}

pub fn stage_growth(height: f32, width: f32) -> StageGrowth {
    let narrow = width < WIDE;
    let padded = ramp(height, grows_from(MediaBarLayout::Stage), GLOW_FULL);
    let grown = ramp(height, grows_from(MediaBarLayout::Stage), GROWN);
    let room = height
        - 2.0 * blend(6.0, 16.0, padded)
        - blend(18.0, 56.0, grown)
        - STAGE_CONTROLS
        - 2.0 * blend(2.0, 12.0, grown);
    let pad = mix(6.0, 16.0, padded);
    let wave = mix(18.0, 56.0, grown);
    let gap = mix(2.0, 12.0, grown);
    let most = ((width - 2.0 * pad - 2.0 * 24.0 - STAGE_LEAST) / 2.0).floor();
    let side = if narrow {
        0.0
    } else {
        (height - 2.0 * pad).max(STAGE_SIDE).min(most).max(0.0)
    };
    StageGrowth {
        pad,
        wave,
        gap,
        room,
        tier: tier_for(room, if narrow { Tier::Liner } else { Tier::Poster }),
        side,
        cover: if narrow { 0.0 } else { (height - 2.0 * pad).min(side) },
        under: pad + STAGE_CONTROLS + 2.0 * gap + wave,
        compact_volume: side < STAGE_SIDE,
        tall: height >= TALL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUTS: [MediaBarLayout; 3] = [MediaBarLayout::Rows, MediaBarLayout::Panel, MediaBarLayout::Stage];
    const FOOT: f32 = 46.0;

    fn heights() -> impl Iterator<Item = f32> {
        (64..=360).map(|h| h as f32)
    }

    fn growth(layout: MediaBarLayout, height: f32, width: f32) -> (f32, Tier) {
        match layout {
            MediaBarLayout::Rows => {
                let g = rows_growth(height, width);
                (g.head, g.tier)
            }
            MediaBarLayout::Panel => {
                let g = panel_growth(height, width, FOOT);
                (g.room, g.tier)
            }
            MediaBarLayout::Stage => {
                let g = stage_growth(height, width);
                (g.room, g.tier)
            }
        }
    }

    #[test]
    fn ramp_and_mix() {
        assert_eq!(ramp(10.0, 20.0, 40.0), 0.0);
        assert_eq!(ramp(30.0, 20.0, 40.0), 0.5);
        assert_eq!(ramp(50.0, 20.0, 40.0), 1.0);
        assert_eq!(mix(18.0, 96.0, 0.0), 18.0);
        assert_eq!(mix(18.0, 96.0, 1.0), 96.0);
        assert_eq!(mix(10.0, 20.0, 0.25), 13.0);
    }

    #[test]
    fn the_one_line_holds_until_each_layout_grows() {
        for layout in LAYOUTS {
            assert!(!grows(layout, LINE));
            assert!(!grows(layout, grows_from(layout) - 1.0));
            assert!(grows(layout, grows_from(layout)));
        }
    }

    #[test]
    fn every_layout_leaves_a_line_of_room_and_never_says_less_as_it_grows() {
        for layout in LAYOUTS {
            for width in [390.0, 1200.0] {
                for height in heights().filter(|h| grows(layout, *h)) {
                    let (room, tier) = growth(layout, height, width);
                    assert!(room >= Tier::Line.room(), "{layout:?} {width} {height}");
                    if grows(layout, height - 1.0) {
                        assert!(tier >= growth(layout, height - 1.0, width).1);
                    }
                }
            }
        }
    }

    #[test]
    fn the_visualizer_shows_faintly_from_about_96() {
        assert_eq!(glow(LINE), 0.0);
        assert_eq!(glow(88.0), 0.0);
        assert!(glow(96.0) > 0.05);
        assert_eq!(glow(220.0), GLOW_MOST);
        assert_eq!(glow(360.0), GLOW_MOST);
    }

    #[test]
    fn the_words_say_more_as_their_room_grows() {
        assert_eq!(tier_for(0.0, Tier::Poster), Tier::Line);
        assert_eq!(tier_for(Tier::Pair.room(), Tier::Poster), Tier::Pair);
        assert_eq!(tier_for(Tier::Liner.room() - 1.0, Tier::Poster), Tier::Pair);
        assert_eq!(tier_for(Tier::Liner.room(), Tier::Poster), Tier::Liner);
        assert_eq!(tier_for(Tier::Poster.room(), Tier::Poster), Tier::Poster);
    }

    #[test]
    fn rows_height_for_gives_the_cover_asked() {
        let height = rows_height_for(160.0);
        assert!(rows_growth(height, f32::INFINITY).cover >= 160.0);
        assert!(rows_growth(height - 1.0, f32::INFINITY).cover < 160.0);
    }
}
