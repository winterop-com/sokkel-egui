//! Every story, in the sidebar's order: a path whose last segment names the story, a line on what
//! it shows, and how to draw it.

use eframe::egui::{Stroke, Ui};
use sokkel_egui::{TextRole, tokens};

mod chart_kinds;
mod charts;
mod chat;
mod dhis2;
mod display;
mod fhir;
mod foundations;
mod frame;
mod icon_list;
mod inputs;
mod listening;
mod maps;
mod media;
mod menus;
mod overlays;
mod page;
mod parts;
mod players;
mod primitives;
mod runner;
pub mod settings;
mod spec;
mod table;
mod themes;
mod watching;

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
    pub cards: sokkel_egui::TableState,
    pub hovered: sokkel_egui::TableState,
    pub palette: usize,
    pub chip_tags: Vec<String>,
    pub chip_filter: Option<String>,
    pub chip_on: Vec<String>,
    pub chip_channel: String,
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
            cards: {
                let mut cards = sokkel_egui::TableState::sorted(0, true);
                cards.grid = true;
                cards.selected.insert(1);
                cards
            },
            hovered: sokkel_egui::TableState::default(),
            palette: 1,
            chip_tags: ["nightly", "harbor", "export", "critical-path"]
                .map(String::from)
                .to_vec(),
            chip_filter: Some("harbor".into()),
            chip_on: ["harbor", "larkspur"].map(String::from).to_vec(),
            chip_channel: "nightly".into(),
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
        path: "Foundations/Text size",
        doc: "Each text role's size over its line height at each text size; the default answers from the md width up, and a phone keeps large, the theme's own sizes.",
        show: foundations::text_size,
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
        doc: "A short value as a chip: each variant, an icon before or after the words, the six kind colors, and drawn as a link.",
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
        path: "Primitives/Inputs/Field issues",
        doc: "Fields holding issues in their tones, the worst edging the control and marked at the field's end, its list opened from the mark: a warning on a yes or no, an error and a warning on a text, a warning on a number.",
        show: primitives::field_issues,
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
        path: "Primitives/Inputs/Select swatches",
        doc: "A select whose options each show a palette's colors as a row of dots before the name.",
        show: overlays::select_swatches,
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
        doc: "Every tone, the chart palette's hues and the kind colors, the code face and icons, a long value cut short, and the control size.",
        show: primitives::chip,
    },
    Story {
        path: "Patterns/Chip actions",
        doc: "Chips a press acts on: buttons with remove buttons, links, toggles on and off in every tone and hue, and a menu's trigger with its menu open.",
        show: primitives::chip_actions,
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
        doc: "Facts as terms and values: side by side in two columns with a missing value and a wide fact, in a line, numeric values with their figures at one width, terms as wide as the longest, small, and stacked with values changed in place.",
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
        path: "Patterns/Data table runs",
        doc: "Cells written in runs of their own inks and faces: a formula with its joining word faint, and a rule with its operator in the code face.",
        show: table::data_table_runs,
    },
    Story {
        path: "Patterns/Data table cards",
        doc: "The rows as cards in a grid, each at least 240 wide: the bar's toggle between the table and the grid, the sort as a menu, a row's actions at its card's corner, and a chosen card ringed with every card's selection box at its other corner while any row is chosen; then a table whose row actions show only on the row pointed at.",
        show: table::data_table_cards,
    },
    Story {
        path: "Patterns/Data table tree",
        doc: "Rows with children: runs under their jobs under their projects, each level indented 20 with the chevron that opens it, harbor open; a branch's box takes every row under it and is mixed while some are chosen. The arrows move along the rows and Home, End, PageUp and PageDown reach along them; Right opens a branch, Left closes it or moves to its parent. Then each person's tokens, loaded as their row first opens, the control a spinner while they come and named for its person (Show the tokens of ada); a load that failed says so under its branch with Try again.",
        show: table::data_table_tree,
    },
    Story {
        path: "Patterns/Data table tree stacked",
        doc: "A tree stacked as cards, as a listed list and as a compact list: each row's control after its indent, 20 a level; listed, every line starts where its title does; sync-inventory's runs refused, said under it with Try again in the two lists (the web's cards leave the line out); run_4119 chosen, its box ticked before its title, its card ringed in the chosen paint and its listed rows on the chosen ground.",
        show: table::data_table_tree_stacked,
    },
    Story {
        path: "Patterns/Data table loading and failed",
        doc: "The table's own loading and failed looks: placeholder rows under the heads while the first rows come, the bar keeping its search and counting nothing; three after the last row while more come; and a failed read in place of the rows, headed by what the server answered, its diagnostic and problem under it, Try again showing the placeholder rows while it reads.",
        show: table::data_table_loading,
    },
    Story {
        path: "Patterns/Item card",
        doc: "The card for one thing: a picture over its words, an icon before its title, a title cut short; a code, a status that wraps, three lines of description and facts in columns; a chosen card with its menu; and a card whose code is its title.",
        show: table::item_card,
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
        path: "Patterns/Save bar",
        doc: "The bar at a page's foot while edits are unsaved: how many, Discard and Save; then the same while the save is on its way.",
        show: parts::save_bar,
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
        path: "Patterns/Side panel",
        doc: "The panel at the page's end: its tabs in the head, 48 clear for the shell's toggle. When the labels do not fit and every tab has an icon, each tab stands as its icon; without icons the labels scroll.",
        show: frame::side_panel_story,
    },
    Story {
        path: "Patterns/Status line",
        doc: "The foot strip: a note at the start and facts at the end in at most 70 percent of the line, each fact whole but the last, which alone gives way.",
        show: frame::status_line_story,
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
        path: "Patterns/Page header editable",
        doc: "A page's head whose title and description are edited where they stand, the count beside the title.",
        show: page::page_header_editable,
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
        doc: "Key figures in a row, each a stat ruled from the next; plain stats (the first large) with a dot, a unit, a change and a trend; and tiles, one emphasized, two of them controls.",
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
        path: "Charts/Bars from rows",
        doc: "Bars drawn from rows and a spec: runs by job split by state and stacked, an average lying down, projects side by side in a set order, and states as shares of 100% in colors of their own.",
        show: chart_kinds::bars,
    },
    Story {
        path: "Charts/Lines and areas from rows",
        doc: "Lines and areas drawn from rows: an average per project, a sum filled under its line, states stacked as areas, and rows counted where no y is given.",
        show: chart_kinds::lines,
    },
    Story {
        path: "Charts/Pie and donut",
        doc: "Each category's share of the whole: a pie in a palette of three, and a ring with its total in the middle. Hover a slice for its figure and share.",
        show: chart_kinds::slices,
    },
    Story {
        path: "Charts/Heatmap",
        doc: "A cell per job and weekday shaded by its runs, its scale beside it; and a scale stepped through three tokens.",
        show: chart_kinds::heatmap,
    },
    Story {
        path: "Charts/Scatter",
        doc: "A point per run, rows moved against duration: fitted to the points, then by project and sized by retries; and a panel filtered to nothing.",
        show: chart_kinds::scatter,
    },
    Story {
        path: "Maps/Map",
        doc: "A map with sites and routes in the chart colors. Drag to pan, scroll to zoom; with no tiles (in tests) it draws a graticule.",
        show: maps::map,
    },
    Story {
        path: "Maps/Choropleth",
        doc: "The regions shaded by their figure on a sequential scale, two with no figure hatched; the legend with its breaks, captions and no-data swatch, and the chosen region's card.",
        show: chart_kinds::choropleth,
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
        path: "Media/Chapter list",
        doc: "The chapters of a book or an episode: each start under its title in the code face, the length at the end, the chapter playing marked with the time left in it or a line along its foot; a press seeks to a chapter.",
        show: listening::chapter_list,
    },
    Story {
        path: "Media/Transcript",
        doc: "An episode's words as they are said, beside its chapters over the player: the line said inked and kept in the middle, the speaker written where the voice changes, each line under its time; a press on a line, a chapter or the bar seeks them all. The bar steps through chapters.",
        show: listening::transcript,
    },
    Story {
        path: "Media/Sleep timer",
        doc: "A moon at the player's end that sets when playback stops: off, after minutes with the time left counting beside it, or at the end of the chapter or the episode.",
        show: listening::sleep_timer,
    },
    Story {
        path: "Media/Speak button",
        doc: "A button that reads a text aloud with audio the app fetches on the first press: waiting, loading, playing and failed, as an icon and with its words.",
        show: listening::speak_button,
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
    Story {
        path: "Media/Video player",
        doc: "A video's player: the picture the app paints at sixteen by nine on the terminal ground, a cue of the captions over it, and the controls along its foot: the position, the title, back, play and forward, the volume, the captions, the speed, the quality, picture in picture and full screen.",
        show: watching::video_player,
    },
    Story {
        path: "Media/Up next",
        doc: "What plays next, over the dimmed picture as a video ends: its artwork, its title and channel, the countdown to playing it by itself, Play now and Cancel.",
        show: watching::up_next,
    },
    Story {
        path: "Media/Video stats",
        doc: "How a video is playing, over its picture: the size, the bitrate, the link, the buffer and the frames dropped, the app's own facts after them, and its verdict in the title's place.",
        show: watching::video_stats,
    },
    Story {
        path: "Media/Channel row",
        doc: "The channel under a video: its picture and name over how many follow it, Follow turning to Following, and like, dislike, Share, Save, Download and More at the line's end, folding into More as the room runs out; large, it heads the channel's page.",
        show: watching::channel_row,
    },
    Story {
        path: "Media/Watch layout",
        doc: "A watch page: the player, what is under it, and what plays next beside both; in theater the player spans both columns and what plays next starts under it.",
        show: watching::watch_layout,
    },
    Story {
        path: "Dhis2/Expression editor",
        doc: "DHIS2's expression languages in the code editor: an indicator's numerator with a finding underlined and listed on the field's issue mark, and a program rule's condition with its program functions and variables.",
        show: dhis2::expression_editor,
    },
    Story {
        path: "Fhir/FHIRPath and CQL",
        doc: "The health-data standards' languages in the code editor: a FHIRPath expression over the whole server with its bound names and a date, and a CQL library in a pane with its declarations, a retrieve and a block comment.",
        show: fhir::languages,
    },
    Story {
        path: "Patterns/Diff view",
        doc: "Two versions of a text as one list of lines, after sokkel-ui's DiffView: a library's earlier version against the editor with the far unchanged lines folded and the counts at its top end, the same with a title and every line, and two texts the same.",
        show: fhir::diff_view,
    },
    Story {
        path: "Patterns/Rail nested places",
        doc: "A rail place holding places of its own, as a workbench's resource types under Resources: open under its chevron while one of them is shown, each with its count, the parent set apart; collapsed, the parent marked.",
        show: fhir::rail_nested,
    },
    Story {
        path: "Patterns/Show more",
        doc: "A block of text held to a few lines, with Show more under it once it runs past them: a description under a video in a well, with its facts over it.",
        show: watching::show_more,
    },
    Story {
        path: "Patterns/Comment thread",
        doc: "The comments under a video: the composer, then each comment with its author, time and words, pinned or by the creator, like, dislike, react and Reply, its reactions, its menu, and its replies folded behind a count.",
        show: watching::comment_thread,
    },
    Story {
        path: "Patterns/Rail collapsed",
        doc: "A rail collapsed to its icons, as a video's page asks: each count a round badge at its icon's corner, a channel's color as a dot in its icon's place; beside it the same rail open.",
        show: watching::rail_collapsed,
    },
    Story {
        path: "Primitives/Chat/Bubble",
        doc: "A message's words in a bubble, after sokkel-ui's Bubble: the person's question at the row's end on the secondary ground with its reactions and the add button under it, then the default and muted grounds.",
        show: chat::bubble,
    },
    Story {
        path: "Patterns/Usage line",
        doc: "What an answer cost and how fast it came, in one quiet line under it: the tokens in and out, the time to the first token, the rate and how long it took, the figures in the text's ink and their units muted; still coming, and stopped at its limit.",
        show: chat::usage_line,
    },
    Story {
        path: "Patterns/Assistant reply",
        doc: "The assistant's side of a turn in a card: the model's mark and name, the answer in markdown with its code colored, its usage line, and the foot with the versions, Copy, Try again and the feedback; streaming with nothing written yet, and failed.",
        show: chat::assistant_reply,
    },
    Story {
        path: "Patterns/Answer column",
        doc: "One model's answer beside the others' in a comparison: the model over its rounding and size, the state as a chip, Copy, Stop, Run again and Remove, the thinking folded behind a line counting its words, the answer, and the usage line at its foot.",
        show: chat::answer_column,
    },
    Story {
        path: "Patterns/Composer",
        doc: "Where a message to an assistant is written, after sokkel-design's composer: inline as a pill with a model's select, opened into the box by a long question while an answer streams (Stop in Send's place), stacked with files waiting to go, and shut.",
        show: chat::composer,
    },
    Story {
        path: "Patterns/Conversation history",
        doc: "The conversations beside a chat, newest first under Pinned, Today and Earlier, each with its time (the hour today, the day this year, the date before), the current one marked; each row's menu pins, renames in place and deletes.",
        show: chat::conversation_history,
    },
    Story {
        path: "Patterns/Chat",
        doc: "A conversation with a model beside the conversations: the header, the question and its reactions, the reply with its code and usage, and the inline composer at the foot, the messages scrolled to their end.",
        show: chat::chat,
    },
    Story {
        path: "Primitives/Display/Progress",
        doc: "How far something has come, after sokkel-ui's Progress: its label and its share on one line over the track, quiet for a line of facts, small in a table's cell, never 0% once begun nor 100% before the end.",
        show: runner::progress,
    },
    Story {
        path: "Patterns/Check list",
        doc: "A health report, after sokkel-ui's CheckList: each check's verdict as a dot, what it found and a hint when it needs attention; a check with checks under it wears the worst and starts open when it is not ok.",
        show: runner::check_list,
    },
    Story {
        path: "Patterns/Stat state",
        doc: "A stat tile whose value is a state's chip, as the runner's State tile beside its numbers.",
        show: runner::stat_chip,
    },
    Story {
        path: "Primitives/Inputs/Select quiet",
        doc: "The small select, the body's words in a control-sm trigger, and the quiet one, a ghost trigger in the action's words, as a composer's model.",
        show: runner::select_quiet,
    },
    Story {
        path: "Patterns/Data table cells",
        doc: "A table's quiet badge in the code face and a cell the app draws itself (a pull's button), as the models' library and discover tables have them.",
        show: runner::table_cells,
    },
    Story {
        path: "Charts/Treemap",
        doc: "Rows sized by a figure in nested rectangles, by source, then format, then model, squarified, after the web's treemap.",
        show: runner::treemap,
    },
    Story {
        path: "Patterns/API link",
        doc: "The request behind a screen: the methods in their inks, the path as a link with a copy that takes the whole address, the whole chip opening the address, and a long path cut to its room.",
        show: spec::api_link,
    },
    Story {
        path: "Patterns/Unfoldable",
        doc: "Headings that open what is under them: shut with a summary, open, a long summary wrapping, facts at the row's end, held open while forced, and a static row lined up with the rest, all against the column's edge.",
        show: spec::unfoldable,
    },
    Story {
        path: "Patterns/Issue count",
        doc: "A form's issues beside its verb: errors, warnings, both behind a rule, counts with nothing to open as a status, and a list open above the face, each issue naming its field.",
        show: spec::issue_count,
    },
    Story {
        path: "Patterns/Server menu",
        doc: "The server an app talks to: up, down and checking, a state of the app's own, a quiet face well and wrong, a face that opens the app's own pane, and the menu open on degraded with the checks behind it.",
        show: spec::server_menu,
    },
    Story {
        path: "Patterns/Sortable grid",
        doc: "Cards the reader puts in order, as many across as fit: each card the grip, or moved by a grip of its own. A dragged card's cell stays at 40 percent where it lands, a copy following the pointer; the arrows move a focused card.",
        show: spec::sortable_grid,
    },
    Story {
        path: "Patterns/Data table editing",
        doc: "An editable table's active cell in each cellFocus: a line along its foot (the default), a ring inside its edge, the cell washed with a quiet ring, and a quiet foot line with its row washed; the counts edit as numbers.",
        show: spec::table_editing,
    },
    Story {
        path: "Patterns/Data table bands",
        doc: "Counts entered by row and column, as a tally is: columns under band heads with a rule where each band starts and ends, rows in groups whose heads sum each count column, the counts edited in place as numbers.",
        show: spec::table_bands,
    },
    Story {
        path: "Patterns/Data table entry grid",
        doc: "The entry grid is the table with editing: a list grouped with each group's sums in its head, the counts under band heads, each count edited in place and kept on its own (the arrows move the active cell; Enter, F2 or a character opens it; Tab saves and moves on; Escape drops). Below its stackBelow each row stacks into a field per count, named by its band and its own name.",
        show: spec::table_entry_grid,
    },
    Story {
        path: "Primitives/Inputs/Secret field",
        doc: "A field for a secret: not set, masked as it is typed with Show; set, sealed with when it was set and Replace; open for a new secret with Cancel; refused; and read only.",
        show: spec::secret_field,
    },
    Story {
        path: "Patterns/Prompt dialog",
        doc: "A dialog that asks for one value: a new project's name, refused by the app's check as it is typed, after taking the value failed and said why in the status slot, the value kept.",
        show: spec::prompt_dialog,
    },
    Story {
        path: "Patterns/Prompt dialog secret",
        doc: "A prompt for a secret, masked with its reveal: empty, so Submit is shut and says why.",
        show: spec::prompt_secret,
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
