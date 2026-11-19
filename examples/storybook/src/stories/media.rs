//! The parts a media library's pages use: a star for favorites, a download kept on the device, a
//! picture of what is playing, and the rail's places songs are dropped on.

use eframe::egui::{self, Id, Sense, Ui, vec2};
use sokkel_egui::{
    ButtonSize, ButtonVariant, DownloadButton, DownloadState, DragData, Icon, Rail, RailDrop, RailEntry, RailGroup,
    SoundAlign, SoundKind, SoundRampBy, SoundVisualizer, StarButton, TextRole, sound_middle, sound_ramp, tokens,
};

use super::{State, section};

/// A bool the story holds between frames, in egui's memory.
fn held(ui: &Ui, key: &str, from: bool) -> bool {
    ui.data(|d| d.get_temp(Id::unique(("media-story", key))))
        .unwrap_or(from)
}

fn hold(ui: &Ui, key: &str, value: bool) {
    ui.data_mut(|d| d.insert_temp(Id::unique(("media-story", key)), value));
}

fn caption(ui: &mut Ui, words: &str) {
    let t = tokens(ui.ctx());
    ui.label(TextRole::Help.rich(&t, words).color(t.color("muted-foreground")));
}

pub fn star_button(_: &mut State, ui: &mut Ui) {
    for (title, outline) in [
        ("Ghost, as a row's action", false),
        ("Outline, beside a header's buttons", true),
    ] {
        section(ui, title);
        ui.horizontal(|ui| {
            for (size, label) in [("xs", "icon-xs"), ("sm", "icon-sm"), ("md", "icon")] {
                ui.vertical(|ui| {
                    caption(ui, label);
                    ui.horizontal(|ui| {
                        for from in [false, true] {
                            let key = format!("star-{outline}-{size}-{from}");
                            let mut starred = held(ui, &key, from);
                            let star = StarButton::new(&mut starred);
                            let star = if outline { star.outline() } else { star };
                            let star = match size {
                                "xs" => star.xs(),
                                "md" => star.md(),
                                _ => star,
                            };
                            ui.add(star);
                            hold(ui, &key, starred);
                        }
                    });
                });
                ui.add_space(24.0);
            }
        });
    }
}

pub fn download_button(_: &mut State, ui: &mut Ui) {
    section(ui, "States");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(24.0, 12.0);
        let specimen = |ui: &mut Ui, label: &str, add: &dyn Fn(&mut Ui)| {
            ui.vertical(|ui| {
                caption(ui, label);
                add(ui);
            });
        };
        specimen(ui, "idle", &|ui| {
            let _ = DownloadButton::new("idle", DownloadState::Idle).show(ui);
        });
        specimen(ui, "running", &|ui| {
            let _ = DownloadButton::new("running", DownloadState::Running)
                .parts(4, 12)
                .cancellable()
                .show(ui);
        });
        specimen(ui, "running, a share", &|ui| {
            let _ = DownloadButton::new("share", DownloadState::Running)
                .value(0.62)
                .cancellable()
                .show(ui);
        });
        specimen(ui, "running, pausable", &|ui| {
            let _ = DownloadButton::new("pausable", DownloadState::Running)
                .value(0.62)
                .pausable()
                .cancellable()
                .show(ui);
        });
        specimen(ui, "paused", &|ui| {
            let _ = DownloadButton::new("paused", DownloadState::Paused)
                .value(0.62)
                .resumable()
                .cancellable()
                .show(ui);
        });
        specimen(ui, "kept", &|ui| {
            let _ = DownloadButton::new("kept", DownloadState::Kept).show(ui);
        });
    });
    section(ui, "Sizes");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 12.0);
        for (size, label) in [
            (ButtonSize::Sm, "sm"),
            (ButtonSize::Default, "default"),
            (ButtonSize::Lg, "lg"),
        ] {
            ui.vertical(|ui| {
                caption(ui, label);
                ui.horizontal(|ui| {
                    let _ = DownloadButton::new(("idle", label), DownloadState::Idle)
                        .size(size)
                        .show(ui);
                    let _ = DownloadButton::new(("running", label), DownloadState::Running)
                        .size(size)
                        .parts(7, 12)
                        .cancellable()
                        .show(ui);
                    let _ = DownloadButton::new(("kept", label), DownloadState::Kept)
                        .size(size)
                        .show(ui);
                });
            });
            ui.add_space(16.0);
        }
    });
    section(ui, "Removing");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(24.0, 12.0);
        ui.vertical(|ui| {
            caption(ui, "kept, removable");
            let _ = DownloadButton::new("removable", DownloadState::Kept)
                .kept_detail("On this device, 48.2 MB")
                .removable()
                .show(ui);
        });
        ui.vertical(|ui| {
            caption(ui, "armed");
            let _ = DownloadButton::new("armed", DownloadState::Kept)
                .removable()
                .armed(true)
                .show(ui);
        });
        ui.vertical(|ui| {
            caption(ui, "failed");
            let _ = DownloadButton::new("failed", DownloadState::Idle)
                .error("The server stopped answering.")
                .show(ui);
        });
        ui.vertical(|ui| {
            caption(ui, "compact");
            ui.horizontal(|ui| {
                let _ = DownloadButton::new("compact-idle", DownloadState::Idle)
                    .compact(true)
                    .show(ui);
                let _ = DownloadButton::new("compact-kept", DownloadState::Kept)
                    .compact(true)
                    .removable()
                    .show(ui);
            });
        });
    });
    section(ui, "On an album's header");
    ui.horizontal(|ui| {
        let _ = DownloadButton::new("album", DownloadState::Idle)
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .compact_on_phone()
            .show(ui);
        let mut starred = held(ui, "album-star", false);
        ui.add(StarButton::new(&mut starred).outline());
        hold(ui, "album-star", starred);
    });
}

/// A moment of music as levels, low to high: a strong bass, a body of mids, highs falling away, and
/// the jagged detail between, as sokkel-ui's story fixture.
fn spectrum_levels() -> Vec<f32> {
    let hump = |at: f32, middle: f32, width: f32| (-((at - middle).powi(2)) / width).exp();
    (0..64)
        .map(|index| {
            let i = index as f32;
            let at = i / 63.0;
            let body = 0.56 * hump(at, 0.07, 0.006)
                + 0.42 * hump(at, 0.34, 0.018)
                + 0.32 * hump(at, 0.62, 0.02)
                + 0.2 * hump(at, 0.86, 0.01);
            let detail = 0.12 * (i * 1.9).sin() * (i * 0.37).cos() + 0.06 * (i * 5.3).sin();
            (0.06 + 0.2 * (1.0 - at) + body + detail).clamp(0.0, 1.0)
        })
        .collect()
}

/// Four cycles of a tone with its overtones, as an oscilloscope shows a held note.
fn wave_samples() -> Vec<f32> {
    (0..640)
        .map(|index| {
            let turn = 2.0 * std::f32::consts::PI * index as f32 / 160.0;
            0.52 * turn.sin()
                + 0.2 * (2.0 * turn + 0.6).sin()
                + 0.12 * (5.0 * turn + 1.1).sin()
                + 0.05 * (11.0 * turn + 0.3).sin()
        })
        .collect()
}

/// What a cell of the visualizer story draws: a look, or a ring with a cover in its middle, or a
/// backdrop behind a title.
enum Cell<'a> {
    Look(SoundVisualizer<'a>),
    Covered(SoundVisualizer<'a>),
    Behind(SoundVisualizer<'a>),
}

/// Cells three to a row, each its caption over its picture.
fn cells(ui: &mut Ui, cells: Vec<(&str, Cell<'_>)>) {
    let t = tokens(ui.ctx());
    let gap = 24.0;
    let wide = (ui.available_width() - 2.0 * gap) / 3.0;
    for row in cells.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (title, cell) in row {
                let (rect, _) = ui.allocate_exact_size(vec2(wide, 22.0 + 110.0), Sense::hover());
                let painter = ui.painter();
                painter.text(
                    rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    *title,
                    TextRole::Help.font(&t),
                    t.color("muted-foreground"),
                );
                let room = egui::Rect::from_min_max(rect.left_top() + vec2(0.0, 22.0), rect.right_bottom());
                match cell {
                    Cell::Look(look) => look.paint(painter, room),
                    Cell::Covered(look) => {
                        look.paint(painter, room);
                        let middle = sound_middle(room);
                        painter.circle_filled(middle.center(), middle.width() / 2.0, t.color("kind-violet"));
                        sokkel_egui::icons::paint(painter, middle.center(), Icon::Music, 14.0, t.color("on-fill"));
                    }
                    Cell::Behind(look) => {
                        look.paint(painter, room);
                        let left = room.left() + 12.0;
                        painter.text(
                            egui::pos2(left, room.bottom() - 48.0),
                            egui::Align2::LEFT_BOTTOM,
                            "Track 3 of 9 · Juniper · 2025",
                            TextRole::Help.font(&t),
                            t.color("muted-foreground"),
                        );
                        painter.text(
                            egui::pos2(left, room.bottom() - 24.0),
                            egui::Align2::LEFT_BOTTOM,
                            "Harbor at Dusk",
                            TextRole::DetailTitle.font(&t),
                            t.color("foreground"),
                        );
                        painter.text(
                            egui::pos2(left, room.bottom() - 6.0),
                            egui::Align2::LEFT_BOTTOM,
                            "Grace",
                            TextRole::Body.font(&t),
                            t.color("muted-foreground"),
                        );
                    }
                }
            }
        });
        ui.add_space(12.0);
    }
}

pub fn sound_visualizer(_: &mut State, ui: &mut Ui) {
    let levels = spectrum_levels();
    let samples = wave_samples();
    let heat = sound_ramp("heat").unwrap_or(&[]);
    let cool = sound_ramp("cool").unwrap_or(&[]);
    let lagoon = sound_ramp("lagoon").unwrap_or(&[]);
    section(ui, "The kinds");
    cells(
        ui,
        vec![
            ("Spectrum", Cell::Look(SoundVisualizer::new().levels(&levels))),
            (
                "Centered",
                Cell::Look(SoundVisualizer::new().levels(&levels).align(SoundAlign::Center)),
            ),
            (
                "Area",
                Cell::Look(SoundVisualizer::new().kind(SoundKind::Area).levels(&levels)),
            ),
            (
                "Wave",
                Cell::Look(SoundVisualizer::new().kind(SoundKind::Wave).samples(&samples)),
            ),
            (
                "Ring, a cover in its middle",
                Cell::Covered(SoundVisualizer::new().kind(SoundKind::Ring).levels(&levels)),
            ),
            ("At rest", Cell::Look(SoundVisualizer::new())),
        ],
    );
    section(ui, "Ramps, a mirror and a backdrop");
    cells(
        ui,
        vec![
            (
                "heat, mirrored",
                Cell::Look(SoundVisualizer::new().levels(&levels).ramp(heat).mirror(true)),
            ),
            (
                "cool, by level",
                Cell::Look(
                    SoundVisualizer::new()
                        .levels(&levels)
                        .ramp(cool)
                        .ramp_by(SoundRampBy::Level),
                ),
            ),
            (
                "lagoon, an area",
                Cell::Look(
                    SoundVisualizer::new()
                        .kind(SoundKind::Area)
                        .levels(&levels)
                        .ramp(lagoon),
                ),
            ),
            (
                "cool, a ring",
                Cell::Covered(SoundVisualizer::new().kind(SoundKind::Ring).levels(&levels).ramp(cool)),
            ),
            (
                "primary, a wave",
                Cell::Look(
                    SoundVisualizer::new()
                        .kind(SoundKind::Wave)
                        .samples(&samples)
                        .ramp(sound_ramp("primary").unwrap_or(&[])),
                ),
            ),
            (
                "A backdrop behind a title",
                Cell::Behind(SoundVisualizer::new().levels(&levels).backdrop(true)),
            ),
        ],
    );
}

/// The kind of thing a song's row carries when dragged.
const SONG: &str = "song";

pub fn rail_drops(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let groups = vec![
        RailGroup::new(
            Some("Library"),
            [
                RailEntry::new("albums", "Albums", Icon::DiscAlbum),
                RailEntry::new("artists", "Artists", Icon::MicVocal),
            ],
        ),
        RailGroup::new(
            Some("Playlists"),
            [
                RailEntry::new("deep-focus", "Deep focus", Icon::ListMusic).takes(SONG),
                RailEntry::new("long-drive", "Long drive", Icon::ListMusic).takes(SONG),
                RailEntry::new("morning-run", "Morning run", Icon::ListMusic).takes(SONG),
            ],
        ),
    ];
    let place_id = Id::unique(("media-story", "place"));
    let said_id = Id::unique(("media-story", "dropped"));
    let mut place: String = ui.data(|d| d.get_temp(place_id)).unwrap_or_else(|| "albums".into());
    let mut said: String = ui.data(|d| d.get_temp(said_id)).unwrap_or_default();
    section(ui, "Drag a song onto a playlist");
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(Rail::width(false), 360.0), Sense::hover());
        let mut dropped: Option<RailDrop> = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.add(Rail::new("Tidevann", &groups, &mut place).drops(&mut dropped));
        });
        if let Some(drop) = dropped {
            let list = groups
                .iter()
                .flat_map(|g| &g.entries)
                .find(|e| e.id == drop.entry)
                .map_or(drop.entry.clone(), |e| e.label.clone());
            said = format!("Added {} to {list}", drop.data.label);
        }
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.label(
                TextRole::SectionTitle
                    .rich(&t, "Salt and Cedar")
                    .color(t.color("foreground")),
            );
            ui.add_space(8.0);
            for (number, title) in ["Low Lagoon", "Silver High Water", "Turning Marsh"].iter().enumerate() {
                let id = Id::unique(("media-story", "song", number));
                let data = DragData {
                    kind: SONG.into(),
                    label: (*title).into(),
                    duration: None,
                    value: format!("salt-and-cedar-{}", number + 1),
                };
                ui.dnd_drag_source(id, data, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_width(320.0);
                        sokkel_egui::icons::paint(
                            ui.painter(),
                            ui.cursor().min + vec2(8.0, 10.0),
                            Icon::GripVertical,
                            16.0,
                            t.color("muted-foreground"),
                        );
                        ui.add_space(24.0);
                        ui.label(
                            TextRole::Body
                                .rich(&t, format!("{}  {title}", number + 1))
                                .color(t.color("foreground")),
                        );
                    });
                });
                ui.add_space(4.0);
            }
            ui.add_space(8.0);
            if !said.is_empty() {
                caption(ui, &said);
            }
        });
    });
    ui.data_mut(|d| {
        d.insert_temp(place_id, place);
        d.insert_temp(said_id, said);
    });
}
