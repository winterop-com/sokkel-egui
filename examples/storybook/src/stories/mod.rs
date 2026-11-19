//! Every story, in the sidebar's order: a path whose last segment names the story, a line on what
//! it shows, and how to draw it.

use eframe::egui::{Stroke, Ui};
use sokkel_egui::{TextRole, tokens};

mod charts;
mod display;
mod foundations;
mod frame;
mod icon_list;
mod inputs;
mod maps;
mod media;
mod menus;
mod overlays;
mod page;
mod parts;
mod players;
mod primitives;
pub mod settings;
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
    pub map: sokkel_egui_maps::MapView,
    pub place: String,
    pub runs: sokkel_egui::TableState,
    pub jobs: sokkel_egui::TableState,
    pub dialog_open: bool,
    pub service: usize,
    pub settings_open: bool,
    pub settings_section: String,
    pub settings_appearance: sokkel_egui::Appearance,
    pub last_command: String,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_highlight: usize,
    pub choice: String,
    pub starts: Option<sokkel_egui::Moment>,
    pub region: String,
    pub update: String,
    pub tree_open: std::collections::HashSet<String>,
    pub tree_chosen: Option<String>,
    pub tree_query: String,
    pub arranged: Vec<sokkel_egui::DashboardLayoutItem>,
    pub arrange_edit: sokkel_egui::DashboardEdit,
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
            map: sokkel_egui_maps::MapView::new(60.4, 11.5, 5.0),
            place: "runs".into(),
            runs: sokkel_egui::TableState::sorted(5, false),
            jobs: sokkel_egui::TableState::default(),
            dialog_open: true,
            service: 1,
            settings_open: true,
            settings_section: "appearance".into(),
            settings_appearance: sokkel_egui::Appearance {
                scheme: sokkel_egui::Scheme::Light,
                ..Default::default()
            },
            last_command: String::new(),
            palette_open: true,
            palette_query: "run".into(),
            palette_highlight: 0,
            choice: "some".into(),
            starts: Some(sokkel_egui::Moment::new(
                sokkel_egui::Day::parse("2026-11-17").expect("a day"),
                2,
                0,
            )),
            region: "Europe".into(),
            update: "We are looking into it.".into(),
            tree_open: ["harbor".to_owned(), "harbor/nightly-export".to_owned()]
                .into_iter()
                .collect(),
            tree_chosen: Some("harbor/nightly-export/orders.csv".into()),
            tree_query: String::new(),
            arranged: parts::arranged_default(),
            arrange_edit: sokkel_egui::DashboardEdit::default(),
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
        path: "Primitives/Actions/Icon button",
        doc: "A button that is an icon alone, its label what a screen reader hears and the pointer is told.",
        show: parts::icon_button,
    },
    Story {
        path: "Primitives/Actions/Star button",
        doc: "A favorite toggle on the icon button: an outlined star that fills in the accent once starred, its label saying what a press does. Off and starred at each size, quiet as a row's action and outlined beside a header's buttons.",
        show: media::star_button,
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
        doc: "A 16px box with its label beside it: checked, not, partly chosen, disabled, and as a field (the whole row its label, as CheckboxField).",
        show: inputs::checkbox,
    },
    Story {
        path: "Primitives/Inputs/Choice list",
        doc: "One answer out of a few, each a row with a radio and a line under it.",
        show: parts::choice_list,
    },
    Story {
        path: "Primitives/Inputs/Date and time field",
        doc: "A date and a time typed as one, or picked on a calendar beside the day's times; a moment before the earliest said under it.",
        show: parts::date_time_field,
    },
    Story {
        path: "Primitives/Inputs/Field",
        doc: "A labeled text field with help under it, and the same field holding an error.",
        show: primitives::field,
    },
    Story {
        path: "Primitives/Inputs/Text area",
        doc: "A field of several lines, growing with its text.",
        show: parts::text_area,
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
        doc: "On, off and disabled, each beside the words it switches; and as SwitchField, a form's row with the switch at its right edge, a help mark that says what it does, and shut.",
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
        path: "Primitives/Overlays/Toast",
        doc: "Short notes at the window's corner: the newest in front, two peeking above it; pointed at, they stand apart and stop counting down. Notify adds one that goes after five seconds; each rises in, sinks out, and is swiped away down or right.",
        show: overlays::toasts,
    },
    Story {
        path: "Primitives/Menus/Dropdown menu",
        doc: "A menu under its trigger, open with the story: the account menu, with a label, icons, shortcuts, a separator and a destructive action.",
        show: menus::dropdown,
    },
    Story {
        path: "Primitives/Navigation/Tabs",
        doc: "A tab list on a muted track with the chosen tab raised, or with a line under it.",
        show: inputs::tabs,
    },
    Story {
        path: "Patterns/Calendar",
        doc: "A week of jobs' runs, the days side by side and the hours running down; runs at the same time split their day, a skipped one is faded, the time now a line across today.",
        show: parts::week_grid,
    },
    Story {
        path: "Patterns/Chip",
        doc: "The tones a chip says a state in, and a chosen chip's stronger wash and edge.",
        show: primitives::chip,
    },
    Story {
        path: "Patterns/Chip bar",
        doc: "A row of chips with one chosen, scrolling sideways where it runs out of room.",
        show: parts::chip_bar,
    },
    Story {
        path: "Patterns/Command palette",
        doc: "The palette over the page, open with the story and narrowed by \"run\": the arrows move the highlight, Enter runs it.",
        show: menus::palette,
    },
    Story {
        path: "Patterns/Activity list",
        doc: "What happened, newest first: the unread on a wash with a dot, a way to act on one, the day's heading over its items.",
        show: parts::activity_list,
    },
    Story {
        path: "Patterns/Bar list",
        doc: "Labeled bars of shares, the chosen one in the primary color, each value as a percent and the counts behind it.",
        show: parts::bar_list,
    },
    Story {
        path: "Patterns/Banner",
        doc: "A line across the page about how things stand, in its tone.",
        show: parts::banner,
    },
    Story {
        path: "Patterns/Description list",
        doc: "Terms over their values, each value words or a control changed in place.",
        show: parts::description_list,
    },
    Story {
        path: "Patterns/Download button",
        doc: "A button that downloads something for keeping: idle, n of m or a share with a cancel and a pause, paused, and kept; a kept copy removed in two presses, a failed try's reason on hover, and the icon alone when compact.",
        show: media::download_button,
    },
    Story {
        path: "Patterns/Data table",
        doc: "The one table: forty runs sorted by rows, with check boxes and the selection bar; a list; and an empty table. Press a head to sort, drag its edge to resize, press a row to open it.",
        show: table::data_table,
    },
    Story {
        path: "Patterns/Files",
        doc: "A file's contents drawn for its kind: a CSV as a table of its first rows, text and code as their lines numbered under the name, and a kind with no preview saying so.",
        show: parts::files,
    },
    Story {
        path: "Patterns/Form bar",
        doc: "A form's actions at its end, a note at its start.",
        show: parts::form_bar_story,
    },
    Story {
        path: "Patterns/App bar",
        doc: "The frame's top bar alone, for a screen without the shell: the mark, the trail and the actions.",
        show: parts::app_bar,
    },
    Story {
        path: "Patterns/Settings dialog",
        doc: "The settings every app has, as one dialog over the page, open with the story: its sections, the appearance rows every app shares, and the footer that resets a section and closes.",
        show: settings::settings_dialog,
    },
    Story {
        path: "Patterns/Shell",
        doc: "The frame every app has: the rail of its places, the top bar with the trail, the page, and the status line across the bottom. Press a place to go there.",
        show: frame::shell,
    },
    Story {
        path: "Patterns/Rail drops",
        doc: "A rail whose places take drops: drag a song's row onto a playlist, the row ringed while the song is over it, and the drop said once let go.",
        show: media::rail_drops,
    },
    Story {
        path: "Patterns/Log view",
        doc: "A service's log on the terminal ground: lines numbered, time, level and source in columns, levels inked, the fields after the text; following, it keeps the newest line in view.",
        show: parts::log_view,
    },
    Story {
        path: "Patterns/Markdown",
        doc: "A note written in markdown at a message's scale: a heading, paragraphs with bold, italic and code, and both kinds of list.",
        show: parts::markdown,
    },
    Story {
        path: "Patterns/Windowed pane",
        doc: "A pane whose content opens near full screen in a window, a reference beside it there; the pane keeps its own copy.",
        show: parts::windowed_pane,
    },
    Story {
        path: "Patterns/Inline code",
        doc: "A one-line value in the code face on a faint wash, in each tone, bare, small, and cut short where it runs past its room.",
        show: parts::inline_code,
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
        path: "Patterns/Section",
        doc: "A titled part of a page with its meta, description and actions, and people on its lines.",
        show: parts::sections,
    },
    Story {
        path: "Patterns/Segmented",
        doc: "One choice among a few options, all in view, in each of its four looks.",
        show: inputs::segmented,
    },
    Story {
        path: "Patterns/Split pane",
        doc: "Two panes side by side, the rule between them dragged to share the room.",
        show: parts::split_pane,
    },
    Story {
        path: "Patterns/Stat",
        doc: "Key figures in a row, each a stat ruled from the next.",
        show: parts::stats,
    },
    Story {
        path: "Patterns/Tag chips",
        doc: "Tags in the code face, wrapping where they run out of room.",
        show: parts::tag_chips,
    },
    Story {
        path: "Patterns/Timeline",
        doc: "Events one under another, newest first: dots in their states' colors, or icons.",
        show: parts::timeline,
    },
    Story {
        path: "Patterns/Tree",
        doc: "Folders that open and shut, one file chosen; a search keeps the rows that match and the folders above them.",
        show: parts::tree,
    },
    Story {
        path: "Patterns/Uptime bar",
        doc: "A service's last ninety days as a bar a day, the day pointed at said under it.",
        show: parts::uptime_bar,
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
        path: "Charts/Arranged dashboard",
        doc: "A dashboard the reader arranges: Edit puts a grip, a size menu and remove in each panel's head and the widgets over the grid; a panel dragged by its grip lands where the line shows, a widget dragged or pressed adds a panel. Cancel throws the edits away, Done keeps them.",
        show: parts::arranged,
    },
    Story {
        path: "Charts/Dashboard panels",
        doc: "The figures a dashboard reads at a glance: stat tiles inked by their thresholds with their trend behind, a gauge on an arc with its thresholds as a band, and bars lying down by job.",
        show: parts::dashboard_panels,
    },
    Story {
        path: "Charts/Dashboard",
        doc: "Panels in a dashboard's grid: a time series of latency beside a panel holding an uptime bar.",
        show: parts::dashboard,
    },
    Story {
        path: "Maps/Map",
        doc: "A map with sites and routes in the chart colors. Drag to pan, scroll to zoom; with no tiles (in tests) it draws a graticule.",
        show: maps::map,
    },
    Story {
        path: "Media/Track list",
        doc: "Songs or episodes: the number or a cover, the title over a second line, a quiet album column, the length in the code face, and the row's own actions. The current row marked, a speaker while it plays; a head over each disc of a set.",
        show: players::track_list,
    },
    Story {
        path: "Media/Reorderable track list",
        doc: "A playlist the listener orders: a row's grip drags it, the rest moving aside to show where it lands, and Escape puts it back; with the grip focused the arrows move it a place.",
        show: players::reorderable_track_list,
    },
    Story {
        path: "Media/Draggable track list",
        doc: "An album whose rows drag as a whole onto a target elsewhere, the title on a small card following the pointer; a press still plays the row.",
        show: players::draggable_track_list,
    },
    Story {
        path: "Media/Audio bar",
        doc: "The player along the foot of a screen as one line: what plays at the left, the transport over the position in the middle, the speed, the volume and the app's own controls at the right; a station, a book with skip buttons, and the two rows a narrow bar folds into.",
        show: players::audio_bar,
    },
    Story {
        path: "Media/Audio bar layouts",
        doc: "The bar dragged taller, in each of its layouts: rows, panel and stage, the waveform as the scrubber and the visualizer rising faintly behind the words.",
        show: players::audio_bar_layouts,
    },
    Story {
        path: "Media/Fullscreen player",
        doc: "The player's fullscreen view, a dark room in every theme: what plays at full size with a star, the lyrics shown in the visualizer's place from the corner's button, the waveform and the transport.",
        show: players::fullscreen_player,
    },
    Story {
        path: "Media/Lyrics",
        doc: "Timed lyrics follow the position, the line sung inked in the middle and the rest faint; a press on a line seeks there. Untimed lyrics read as a page, verse by verse.",
        show: players::lyrics,
    },
    Story {
        path: "Media/Queue",
        doc: "What plays next, in the order it plays: dragged into another order or taken out, and shuffle and repeat in the bar rearranging it with the song playing kept.",
        show: players::queue,
    },
    Story {
        path: "Media/Sound visualizer",
        doc: "A picture of what is playing, from levels the app hands it each frame: a spectrum, centered bars, an area and a wave; along a ramp of tokens, mirrored or colored by level; a ring with a cover in its middle, at rest, and faint behind a title.",
        show: media::sound_visualizer,
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
