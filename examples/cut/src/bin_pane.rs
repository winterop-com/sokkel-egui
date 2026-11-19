//! The media bin, as the web's `editor/Bin.tsx`: the pictures and sounds to cut from, each dragged
//! onto a track of its kind, placed at the playhead when pressed or at the end of its track from its
//! menu, and a title added at the playhead.

use eframe::egui::{self, Ui, vec2};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, DragData, Icon, MenuItem, PaneHeader, PaneHeaderVariant, Waveform, group_label,
    tokens,
};
use sokkel_egui_media::{MediaCard, MediaGrid, MediaShape};

use crate::editor::{bars_of, media_drag};
use crate::media::{SOURCES, Source, SourceKind, draw_source, poster_time};
use crate::model::length_text;

/// What the bin was asked this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct BinAsked {
    /// A source to place at the playhead.
    pub add: Option<&'static Source>,
    /// A source to place at the end of its track.
    pub append: Option<&'static Source>,
    /// A title to add at the playhead.
    pub title: bool,
}

/// A source's card: its still or its sound's bars under its length, its name and where it was
/// recorded, carried onto the timeline when dragged.
fn source_card(ui: &mut Ui, source: &'static Source, menu: &[MenuItem], asked: &mut BinAsked) {
    let drag = media_drag(source);
    let primary = tokens(ui.ctx()).color("primary");
    let mut card = MediaCard::new(("bin", source.id), source.name)
        .shape(MediaShape::Wide)
        .subtitle(source.place)
        .title_lines(1)
        .badge(length_text(source.duration))
        .menu(menu)
        .drag(DragData {
            kind: "media".into(),
            label: drag.label.into(),
            duration: Some(drag.duration),
            value: drag.source.into(),
        });
    card = match source.kind {
        SourceKind::Video => card.cover(move |painter, rect| draw_source(painter, rect, source, poster_time(source))),
        SourceKind::Audio => {
            let bars: Vec<f32> = bars_of(source).into_iter().map(|b| b as f32).collect();
            card.cover(move |painter, rect| {
                Waveform::new(&bars)
                    .bar_width(2.0)
                    .bar_gap(1.0)
                    .fade_edges(false)
                    .color(primary)
                    .paint(painter, rect.shrink2(vec2(8.0, 0.0)));
            })
        }
    };
    let out = card.show(ui);
    if out.selected || out.menu == Some(0) {
        asked.add = Some(source);
    }
    if out.menu == Some(1) {
        asked.append = Some(source);
    }
}

pub fn bin_pane(ui: &mut Ui) -> BinAsked {
    let mut asked = BinAsked::default();
    PaneHeader::new()
        .title("Media")
        .variant(PaneHeaderVariant::Panel)
        .show(ui, |ui| {
            asked.title = ui
                .add(
                    Button::new("Title")
                        .icon(Icon::Type)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Xs),
                )
                .clicked();
        });
    let menu = [
        MenuItem::action("Add at the playhead").icon(Icon::Plus),
        MenuItem::action("Add at the end").icon(Icon::ArrowRightToLine),
    ];
    let pictures: Vec<&'static Source> = SOURCES.iter().filter(|s| s.kind == SourceKind::Video).collect();
    let sounds: Vec<&'static Source> = SOURCES.iter().filter(|s| s.kind == SourceKind::Audio).collect();
    egui::ScrollArea::vertical()
        .id_salt("cut-bin")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 24.0;
                for (title, sources) in [("Pictures", &pictures), ("Sound", &sounds)] {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        group_label(ui, title);
                        MediaGrid::new(MediaShape::Wide)
                            .min(120.0)
                            .show(ui, sources.len(), |ui, index| {
                                source_card(ui, sources[index], &menu, &mut asked);
                            });
                    });
                }
            });
        });
    asked
}
