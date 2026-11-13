//! Every story, in the sidebar's order: a path whose last segment names the story, a line on what
//! it shows, and how to draw it.

use eframe::egui::{Stroke, Ui};
use sokkel_ui_egui::{TextRole, tokens};

mod charts;
mod display;
mod foundations;
mod frame;
mod icon_list;
mod inputs;
mod maps;
mod overlays;
mod page;
mod primitives;
mod table;
mod themes;

pub struct Story {
    pub path: &'static str,
    pub doc: &'static str,
    pub show: fn(&mut State, &mut Ui),
}

/// What the stories' controls hold between frames.
pub struct State {
    pub job: String,
    pub schedule: String,
    pub empty: String,
    pub notify: bool,
    pub retry: bool,
    pub paused: bool,
    pub icon_search: String,
    pub icon_size: f32,
    pub tab: usize,
    pub period: usize,
    pub map: sokkel_ui_egui_maps::MapView,
    pub place: String,
    pub runs: sokkel_ui_egui::TableState,
    pub jobs: sokkel_ui_egui::TableState,
    pub dialog_open: bool,
    pub service: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            job: "nightly-export".into(),
            schedule: "every day at 25:00".into(),
            empty: String::new(),
            notify: true,
            retry: false,
            paused: false,
            icon_search: String::new(),
            icon_size: 20.0,
            tab: 0,
            period: 1,
            map: sokkel_ui_egui_maps::MapView::new(60.4, 11.5, 5.0),
            place: "runs".into(),
            runs: sokkel_ui_egui::TableState::sorted(5, false),
            jobs: sokkel_ui_egui::TableState::default(),
            dialog_open: true,
            service: 1,
        }
    }
}

pub const STORIES: &[Story] = &[
    Story {
        path: "Foundations/Colors",
        doc: "Every color token of the theme and scheme in the toolbar, as sRGB, grouped by what it paints.",
        show: foundations::colors,
    },
    Story {
        path: "Foundations/Typography",
        doc: "The text roles: a component sets its text by role, never by a size and a weight of its own.",
        show: foundations::typography,
    },
    Story {
        path: "Foundations/Corners",
        doc: "Every corner derives from one radius: controls round by the control radius, surfaces by the surface radius.",
        show: foundations::corners,
    },
    Story {
        path: "Foundations/Density",
        doc: "The layout lengths each density level sets; the toolbar's density picks the one the stories wear.",
        show: foundations::density,
    },
    Story {
        path: "Foundations/Icons",
        doc: "The one icon set: Lucide. These are the icons sokkel-ui's packages import; press one to copy its name.",
        show: foundations::icons,
    },
    Story {
        path: "Foundations/Code colors",
        doc: "The code-* tokens a code block and the editor color their text with.",
        show: foundations::code_colors,
    },
    Story {
        path: "Themes/All themes",
        doc: "Every theme side by side, each tile drawn in its own theme as a scope, in both schemes.",
        show: themes::all_themes,
    },
    Story {
        path: "Themes/Sample",
        doc: "A small page of jobs in the theme and scheme in the toolbar.",
        show: themes::sample,
    },
    Story {
        path: "Primitives/Actions/Button",
        doc: "The variants, the sizes, and the busy and disabled states, with the reason a disabled button gives on hover.",
        show: primitives::button,
    },
    Story {
        path: "Primitives/Display/Alert",
        doc: "A message that stays on the page, plain or destructive, with an icon beside its title and description.",
        show: display::alert,
    },
    Story {
        path: "Primitives/Display/Badge",
        doc: "A short label in the chip corner, in each variant, with and without an icon.",
        show: display::badge,
    },
    Story {
        path: "Primitives/Display/Card",
        doc: "A surface for one subject: titled with a description, or with its title in a head bar and a footer strip.",
        show: display::card,
    },
    Story {
        path: "Primitives/Display/Separator",
        doc: "A 1px rule in the border color, across or down.",
        show: display::separator,
    },
    Story {
        path: "Primitives/Display/Skeleton",
        doc: "Placeholders in the muted ground, pulsing while their content loads.",
        show: display::skeleton,
    },
    Story {
        path: "Primitives/Inputs/Checkbox",
        doc: "A 16px box with its label beside it: checked, not, partly chosen, and disabled.",
        show: inputs::checkbox,
    },
    Story {
        path: "Primitives/Inputs/Field",
        doc: "A labeled text field with help under it, and the same field holding an error.",
        show: primitives::field,
    },
    Story {
        path: "Primitives/Inputs/Input",
        doc: "A one-line text input: plain, with a placeholder, invalid, and disabled.",
        show: inputs::input,
    },
    Story {
        path: "Primitives/Inputs/Select",
        doc: "One choice from a list in a popup under a trigger that looks like an input; the second opens with the story.",
        show: overlays::select,
    },
    Story {
        path: "Primitives/Inputs/Switch",
        doc: "On, off and disabled, each beside the words it switches.",
        show: primitives::switch,
    },
    Story {
        path: "Primitives/Overlays/Dialog",
        doc: "A dialog over the page, open with the story: its title, a description, a field and a switch, and its actions in the footer band.",
        show: overlays::dialog,
    },
    Story {
        path: "Primitives/Overlays/Tooltip",
        doc: "A short line on the foreground ground, under its part after the pointer rests or while the part has focus.",
        show: overlays::tooltips,
    },
    Story {
        path: "Primitives/Navigation/Tabs",
        doc: "A tab list on a muted track with the chosen tab raised, or with a line under it.",
        show: inputs::tabs,
    },
    Story {
        path: "Patterns/Chip",
        doc: "The tones a chip says a state in, and a chosen chip's stronger wash and edge.",
        show: primitives::chip,
    },
    Story {
        path: "Patterns/Data table",
        doc: "The one table: forty runs sorted by rows, with check boxes and the selection bar; a list; and an empty table. Press a head to sort, drag its edge to resize, press a row to open it.",
        show: table::data_table,
    },
    Story {
        path: "Patterns/Shell",
        doc: "The frame every app has: the rail of its places, the top bar with the trail, the page, and the status line across the bottom. Press a place to go there.",
        show: frame::shell,
    },
    Story {
        path: "Patterns/Page header",
        doc: "A page's head: its title with a count or a code, a description, and the page's actions at the end of the row.",
        show: page::page_header,
    },
    Story {
        path: "Patterns/Page state",
        doc: "What a page shows in place of its content: loading, a refused read with its problems, no answer, a part refused, and empty.",
        show: page::page_state,
    },
    Story {
        path: "Patterns/Segmented",
        doc: "One choice among a few options, all in view, in each of its four looks.",
        show: inputs::segmented,
    },
    Story {
        path: "Charts/Line",
        doc: "Lines over days, one per series with a legend; and a line with a wash under it and a shaded band.",
        show: charts::line,
    },
    Story {
        path: "Charts/Bars",
        doc: "Bars over jobs, the series side by side or stacked. Hover a category for its values.",
        show: charts::bars,
    },
    Story {
        path: "Charts/Sparkline",
        doc: "A small line with no axis, in a stat card or beside a row.",
        show: charts::sparkline,
    },
    Story {
        path: "Maps/Map",
        doc: "A map with sites and routes in the chart colors. Drag to pan, scroll to zoom; with no tiles (in tests) it draws a graticule.",
        show: maps::map,
    },
];

/// A titled block of a story, ruled off from the one before it.
pub fn section(ui: &mut Ui, title: &str) {
    let t = tokens(ui.ctx());
    ui.add_space(8.0);
    ui.label(TextRole::SectionTitle.rich(&t, title).color(t.color("foreground")));
    let rect = ui.available_rect_before_wrap();
    ui.painter()
        .hline(rect.x_range(), rect.top() + 4.0, Stroke::new(1.0, t.color("border")));
    ui.add_space(14.0);
}
