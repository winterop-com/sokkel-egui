//! The primitives, each drawn from the tokens alone, after sokkel-ui's components of the same name.

mod activity;
mod api_link;
mod arrange;
mod board;
mod button;
mod calendar;
mod chat;
mod chip;
mod chip_bar;
mod choice;
mod clip_timeline;
mod code_editor;
mod commands;
mod comment_thread;
mod consequences;
mod contents;
mod dashboard;
mod datetime;
mod diff_view;
mod display;
mod document_tabs;
mod dot;
mod download_button;
mod edit_preview;
mod empty;
mod fields;
mod file_preview;
mod filters;
mod form_fields;
mod frame;
mod info;
mod inline_code;
mod inline_edit;
mod item;
mod item_card;
mod json_schema;
mod json_tree;
mod layout;
mod list_row;
mod log_view;
mod markdown;
mod media_controls;
mod media_slider;
mod menu;
mod meta_line;
mod mini_month;
mod not_found;
mod number_field;
mod overlay;
mod page;
mod pane_header;
mod panel_menu;
mod panes;
mod person;
mod problem;
mod progress;
mod prompt;
mod run_timeline;
pub(crate) mod schema_form;
mod search;
mod search_results;
mod secret_field;
mod section;
mod server_menu;
mod settings;
mod sheet;
mod show_more;
mod sign_in;
mod sign_in_art;
mod slider_field;
/// The visualizer's levels, shapes and constants, after sokkel-ui's `sound.ts`, for an app that
/// feeds it live data; kept under their module so their plain names stay out of the crate root.
pub mod sound_visualizer;
mod speak_button;
mod star_button;
mod stat;
mod stat_panel;
mod switch;
mod table;
mod table_edit;
mod text_field;
mod time;
mod time_range;
mod toast;
mod tree;
mod tree_select;
mod uptime;
mod variables;
mod video_player;
mod waveform;
mod week;
mod windowed_pane;

pub use activity::{ActivityItem, ActivityList, ActivityResponse, ActivityTone, ActivityWords, BarList, BarListItem};
pub use api_link::{ApiLink, ApiLinkWords};
pub use arrange::{
    ALL_SPANS, ArrangeItem, DashboardArrange, DashboardArrangeResponse, DashboardArrangeWords, DashboardEdit,
    DashboardEditActions, DashboardEditOutcome, DashboardEditWords, DashboardLayoutItem, DashboardWidget,
    arranged_index, move_item, panel_tools,
};
pub use board::{BOARD_COLUMN, Board, BoardCard, BoardColumn, BoardMove, CardResponse};
pub use button::{Button, ButtonSize, ButtonVariant, IconButton};
pub use calendar::{Calendar, CalendarWords, RangeCalendar, WeekStart, calendar_forget};
pub use chat::{
    AnswerColumn, AnswerColumnResponse, AnswerColumnWords, AnswerState, AssistantReply, AssistantReplyResponse,
    AssistantReplyWords, AssistantState, AttachmentKind, Bubble, BubbleVariant, Composer, ComposerAttachment,
    ComposerLayout, ComposerResponse, ComposerWords, Conversation, ConversationEntry, ConversationGroup,
    ConversationHistory, ConversationHistoryResponse, ConversationHistoryWords, ReactionChips, ReactionChipsWords,
    ReplyFeedback, UsageFigures, UsageLine, UsageLineWords, conversation_group, conversation_time, usage_count,
    usage_speed, usage_time, word_count,
};
pub use chip::{Chip, ChipHue, ChipMenuAlign, ChipResponse, ChipSize, ChipTone, ChipWords, TagChips};
pub use chip_bar::{ChipBar, ChipBarWords};
pub use choice::{
    Checkbox, Choice, ChoiceList, Segmented, SegmentedVariant, SwatchGroup, Tabs, TabsVariant, checkbox_list,
};
pub use clip_timeline::{
    CLIP_TIMELINE_FIT_PPS, CLIP_TIMELINE_FIT_ROOM, CLIP_TIMELINE_ZOOM_MAX, CLIP_TIMELINE_ZOOM_MIN, ClipChange,
    ClipDrop, ClipTimeline, ClipTimelineResponse, ClipTimelineWords, ClipTrackKind, DragData, KindColor, TimelineClip,
    TimelineTrack, TrackChange, clip_fit_zoom, clip_tick_step, format_timecode, place_clip, snap_time, zoom_step,
};
pub use code_editor::regex;
pub use code_editor::{
    ActiveLine, CodeCompletion, CodeEditor, CodeEditorResponse, CodeEditorWords, CodeLanguage, CodeMark, CodeProblem,
    CodeSize, CodeTag, CodeToken, CodeVariant, LARGE_DOCUMENT, MarkTone,
};
pub use commands::{AppearanceCommandWords, appearance_commands, place_commands, place_of, run_appearance_command};
pub use comment_thread::{
    CommentComposer, CommentComposerResponse, CommentComposerWords, CommentThread, CommentThreadResponse,
    CommentThreadWords, QUICK_REACTIONS, ThreadComment,
};
pub use consequences::{Consequence, ConsequenceList, ConsequenceListWords};
pub use contents::{ContentsEntry, TableOfContents, TableOfContentsWords};
pub use dashboard::{
    DashboardGrid, DashboardPanel, GridCells, PanelSpan, body_room, cell_height, report_natural_height, took_room,
};
pub use datetime::{DateField, DateFieldWords, DateTimeField, DateTimeFieldResponse, DateTimeFieldWords, Moment};
pub use diff_view::{DiffKind, DiffLine, DiffRow, DiffView, DiffViewWords, diff_counts, diff_lines, fold_diff};
pub use display::{Alert, Badge, BadgeVariant, Banner, BannerTone, Card, Separator, Skeleton, TextLink, Unfoldable};
pub use document_tabs::{DocumentTab, DocumentTabs, DocumentTabsResponse, DocumentTabsWords};
pub use dot::Dot;
pub use download_button::{
    DownloadAsked, DownloadButton, DownloadButtonResponse, DownloadButtonWords, DownloadState, ProgressRing,
    download_share, format_percent,
};
pub use edit_preview::{EditPreview, EditPreviewView};
pub use empty::Empty;
pub(crate) use fields::search_field;
pub use fields::{
    DueState, DueText, Field, FieldWords, FormBar, QuickAdd, SaveBar, SaveBarResponse, SaveBarWords, SearchField,
    SwitchField, due_state, field, form_bar, pane_toolbar, toolbar,
};
pub use file_preview::{FileKind, FilePreview, FilePreviewWords, format_size, parse_csv};
pub use filters::{
    Filter, FilterBar, FilterBarWords, FilterChip, FilterField, FilterOperator, FilterType, FilterValue, ListViewState,
    SavedView, SavedViews, SavedViewsAsked, SavedViewsWords, filter_rows, matches_filter, operator_word, same_filters,
};
pub use form_fields::{
    ComputedField, ComputedFieldWords, IssueCount, IssueCountWords, IssueEntry, PeriodPicker, PeriodPickerWords,
    RepeatedGroup, RepeatedGroupWords, RepeatedOutcome, TimeField, TimeFieldWords, YesNoField, YesNoFieldWords,
    read_time,
};
pub use frame::{
    AppBar, DesktopWindow, Rail, RailBody, RailDrop, RailEntry, RailGroup, RailWords, Shell, ShellPart, ShellWords,
    StatusLine, StatusTone, TopBar, desktop_viewport, is_phone,
};
pub use info::info_tip;
pub use inline_code::{InlineCode, InlineCodeTone};
pub use inline_edit::{InlineEdit, InlineEditLook, InlineEditWords};
pub use item::{Item, ItemDescription, ItemSize, ItemVariant, item_group};
pub use item_card::{CardFact, ItemCard, ItemCardWords, SortableGrid, SortableHandle, SortableOutcome, SortableWords};
pub use json_schema::json_schema_2020_12;
pub use json_tree::{JsonTree, JsonTreeWords};
pub use layout::{AsideWidth, Columns, Gap, Split, SplitPane, Stack};
pub use list_row::{ListRow, ListRows, RowAside, RowMark};
pub use log_view::{LogLevel, LogLine, LogView, LogViewWords};
pub use markdown::{Markdown, MarkdownResponse, MarkdownScale, MarkdownWords, heading_id};
pub use media_controls::{
    LiveMark, MEDIA_RATES, PlayButton, PlayWords, Position, PositionResponse, RateMenu, SEEK_LARGE_STEP, SEEK_STEP,
    TransportButton, TransportGlyph, TransportSize, VolumeControl, VolumeResponse, VolumeWords, format_rate,
    format_remaining, format_time, paint_pause, paint_play, paint_skip, volume_icon,
};
pub use media_slider::{MediaRange, MediaSlider, MediaSliderResponse, MediaTone};
pub use menu::{CommandGroup, CommandPalette, DropdownMenu, MenuItem, PaletteOutcome, PaletteWords, palette_shortcut};
pub use meta_line::{MetaLine, MetaLineResponse};
pub use mini_month::{MiniMonth, MiniMonthResponse, MiniMonthWords};
pub use not_found::{NotFound, NotFoundWords};
pub use number_field::{NumberField, NumberFieldWords, read_number, write_number};
pub use overlay::{
    ConfirmAnswer, ConfirmDialog, Dialog, DialogShell, Select, ShellAnswer, UnsavedChangesWords, tooltip,
    tooltip_shown, unsaved_changes,
};
pub use page::{Page, PageHeader, PageHeaderResponse, PageState, PageStateTone, PageStateWords};
pub use pane_header::{PaneHeader, PaneHeaderVariant, PaneTitle, section_title_weight};
pub use panel_menu::{PanelAsked, PanelMenu, PanelWindow, PanelWords, body_drawn, panel_away, quiet_tools, windowed};
pub use panes::{
    Inspector, InspectorWords, SidePanel, SubNav, group_label, group_label_galley, group_label_text, inspector_section,
    panel_section, panel_section_actions,
};
pub use person::{AccountAction, AccountMenu, AccountWords, AvatarSize, PersonAvatar, initials, kind_of};
pub use problem::{Problem, ProblemTone, ProblemWords};
pub use progress::{Check, CheckList, CheckStatus, Progress, check_tree, worst_status};
pub use prompt::{PromptAnswer, PromptCheck, PromptDialog, PromptWords};
pub use run_timeline::{
    RunTimeline, RunTimelineResponse, RunTimelineWords, TimelineLane, TimelineRun, format_shift, timeline_ticks,
};
pub use schema_form::{
    IssueSaid, IssueTone, MapEntry, MapRow, SchemaForm, SchemaFormResponse, SchemaFormWords, SchemaIssue,
    SchemaIssueKind, SchemaPath, ServerIssue, ShownIssue, blank_item, initial_value, object_of, pointer, read_scalar,
    repeated_keys, rows_of, say_issue, scalar_union, schema_issues, unset_choice, without_deferred,
};
pub use search::{Search, SearchPreview, SearchResponse, SearchWords};
pub use search_results::{
    KIND_FACET, SearchChosen, SearchFacet, SearchFacetValue, SearchGroup, SearchResult, SearchResults,
    SearchResultsResponse, SearchResultsWords, toggle_chosen,
};
pub use secret_field::{SecretField, SecretFieldWords};
pub use section::{DescriptionLayout, DescriptionList, PersonRow, Section, Timeline, TimelineEvent};
pub use server_menu::{ServerCheck, ServerMenu, ServerMenuResponse, ServerMenuWords, ServerOwnState, ServerState};
pub use settings::{
    SettingRow, SettingsDialog, SettingsOutcome, SettingsSection, SettingsWords, ThemeWords, setting_row,
    setting_row_below, setting_row_found_by, theme_settings, theme_settings_listed, theme_settings_of,
};
pub use sheet::{DetailSheet, DetailSheetWords};
pub use show_more::{ShowMore, ShowMoreWords};
pub use sign_in::{SignIn, SignInFact, SignInValues, SignInWords};
pub use sign_in_art::{FlowEdge, FlowLines, FlowNode, FlowNodes, LevelBars, flow_graph, level_heights, seeded};
pub use slider_field::{Slider, SliderField};
pub use sound_visualizer::{
    SOUND_RAMPS, SoundAlign, SoundKind, SoundRampBy, SoundRange, SoundVisualizer, sound_middle, sound_ramp,
};
pub use speak_button::{SpeakButton, SpeakButtonWords, SpeakPress, SpeakState, paint_square, speak_press};
pub use star_button::{StarButton, StarButtonWords};
pub use stat::{KpiRow, Stat, StatTiles, sparkline};
pub use stat_panel::{StatColorMode, StatPanel, StatWords, Threshold, format_compact, format_value, threshold_color};
pub use switch::Switch;
pub use table::{
    Aggregate, Cell, CellEdit, CellFocus, CellRun, Column, DataTable, ReadOutcome, RowActions, RowCard, StackLayout,
    StackPlace, TableOutput, TableState, TableVariant, TableWords, Validate, sum_aggregate,
};
pub use table_edit::{EditWords, Editor};
pub use text_field::{Input, TextField};
pub use time::{relative_time, relative_time_short};
pub use toast::{Toast, ToastKind, Toaster, ToasterWords, close_toast, toast_acted, toasts};
pub use tree::{
    CanMove, Tree, TreeBand, TreeDropPart, TreeNode, TreeOrder, TreeResponse, TreeRow, TreeWords, drop_part,
    move_tree_node, tree_ancestors, tree_matches, tree_node, visible_rows,
};
pub use tree_select::{TreeSelect, TreeSelectWords};
pub use uptime::{UptimeBar, UptimeBarWords, UptimeDay, format_uptime, overall_uptime, uptime_color};
pub use video_player::{VIDEO_SKIP, VideoPlayer, VideoPlayerResponse, VideoPlayerWords, VideoQuality};
pub use waveform::{AudioScrubber, AudioScrubberResponse, Waveform, static_levels};
pub use week::{CalendarEvent, PlacedEvent, WeekGrid, WeekGridResponse, layout_day};
pub use windowed_pane::{WindowedAside, WindowedPane, WindowedPaneWhere, WindowedPaneWords, windowed_pane_button};

use egui::{Color32, CornerRadius, Painter, Rect, Response, Stroke, StrokeKind, Ui};

use crate::Tokens;

/// How much a disabled control keeps, as `opacity-50` does.
pub(crate) const DISABLED: f32 = 0.5;

/// The keyboard focus outline: 2px in `ring`, 2px off the control, as `focus-visible:outline-2
/// outline-offset-2` draws it.
pub(crate) fn focus_ring(painter: &Painter, tokens: &Tokens, rect: Rect, radius: CornerRadius, response: &Response) {
    if response.has_focus() {
        let outer = CornerRadius::from(radius.nw.saturating_add(2));
        painter.rect_stroke(
            rect.expand(2.0),
            outer,
            Stroke::new(2.0, tokens.color("ring")),
            StrokeKind::Outside,
        );
    }
}

/// The keyboard focus outline drawn inside a control, for one at the window's edge whose outline
/// outside would be cut off (the top bar's buttons on a phone, the spec's `phone+focus`):
/// `focus-ring-width` in `ring`, `focus-ring-offset` inside the control's edge.
pub(crate) fn focus_ring_inset(
    painter: &Painter,
    tokens: &Tokens,
    rect: Rect,
    radius: CornerRadius,
    response: &Response,
) {
    if response.has_focus() {
        let width = tokens.length("focus-ring-width");
        let inset = tokens.length("focus-ring-offset") + width;
        let inner = CornerRadius::from(radius.nw.saturating_sub(inset as u8));
        painter.rect_stroke(
            rect.shrink(inset),
            inner,
            Stroke::new(width, tokens.color("ring")),
            StrokeKind::Outside,
        );
    }
}

/// A paint's ground, as sokkel-design resolves it for a part in a state; the token `or` names
/// where a theme lacks the paint (the paint check fails a paint egui reads that is missing).
pub(crate) fn ground(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.ground)
        .unwrap_or_else(|| tokens.color(or))
}

/// A paint's edge, as `ground`.
pub(crate) fn edge(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.edge)
        .unwrap_or_else(|| tokens.color(or))
}

/// A paint's ink, as `ground`.
pub(crate) fn ink(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.ink)
        .unwrap_or_else(|| tokens.color(or))
}

/// The paint keys egui's parts read, for the paint check.
pub const PAINTS_READ: &[&str] = &[
    "dropdown-menu.base.rest.popup",
    "dropdown-menu.base.highlighted.item",
    "dialog.base.rest.box",
    "dialog.base.rest.backdrop",
    "dialog.base.rest.footer",
    "tooltip.base.rest.box",
    "select.base.highlighted.row",
    "text-field.base.rest.control",
    "text-field.base.focus.control",
    "text-field.base.invalid.control",
    "text-field.base.invalid+focus.control",
    "badge.default.rest.container",
    "badge.secondary.rest.container",
    "badge.destructive.rest.container",
    "badge.outline.rest.container",
    "badge.ghost.rest.container",
    "badge.ghost.hover.container",
    "badge.link.rest.container",
    "badge.link.hover.container",
    "badge.kind-blue.rest.container",
    "badge.kind-green.rest.container",
    "badge.kind-jade.rest.container",
    "badge.kind-pink.rest.container",
    "badge.kind-teal.rest.container",
    "badge.kind-violet.rest.container",
    "chip.neutral.rest.container",
    "chip.neutral.selected.container",
    "chip.good.rest.container",
    "chip.good.selected.container",
    "chip.critical.rest.container",
    "chip.critical.selected.container",
    "chip.warning.rest.container",
    "chip.warning.selected.container",
    "chip.info.rest.container",
    "chip.info.selected.container",
    "chip.outline.rest.container",
    "chip.outline.selected.container",
    "chip.absent.rest.container",
    "chip.hue-1.rest.container",
    "chip.hue-1.selected.container",
    "chip.hue-2.rest.container",
    "chip.hue-2.selected.container",
    "chip.hue-3.rest.container",
    "chip.hue-3.selected.container",
    "chip.hue-4.rest.container",
    "chip.hue-4.selected.container",
    "chip.hue-5.rest.container",
    "chip.hue-5.selected.container",
    "chip.hue-blue.rest.container",
    "chip.hue-blue.selected.container",
    "chip.hue-green.rest.container",
    "chip.hue-green.selected.container",
    "chip.hue-jade.rest.container",
    "chip.hue-jade.selected.container",
    "chip.hue-pink.rest.container",
    "chip.hue-pink.selected.container",
    "chip.hue-teal.rest.container",
    "chip.hue-teal.selected.container",
    "chip.hue-violet.rest.container",
    "chip.hue-violet.selected.container",
    "table.base.selected.row",
    "table.base.opened.row",
    "table.base.selected.card",
    "table.base.opened.card",
    "chip.base.focus.face",
    "chip.base.rest.remove",
    "chip.base.hover.remove",
];

/// A popup's surface (a select's list, a menu, a submenu, the account menu): the popover's
/// ground, the surface's corners, a 1px `border-strong` edge and `elevation-pop`'s shadow.
/// Keys written for a reader, as a key cap or a menu's shortcut says them: the text in `role`,
/// spaced by `spacing`, with "⌘" drawn as Lucide's command glyph, since the text faces have none.
/// Its ink is the painter's, as the galley is drawn.
pub fn keys_galley(
    ui: &egui::Ui,
    tokens: &Tokens,
    role: crate::TextRole,
    keys: &str,
    spacing: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    let style = ui.style();
    let size = role.font(tokens).size;
    let mut add = |rich: egui::RichText| {
        rich.append_to(&mut job, style, egui::FontSelection::Default, egui::Align::Center);
    };
    // the modifier keys' symbols, which the text faces lack, as their icons
    let symbol = |c: char| match c {
        '⌘' => Some(crate::Icon::Command),
        '⇧' => Some(crate::Icon::ArrowBigUp),
        '⌥' => Some(crate::Icon::Option),
        '⌃' => Some(crate::Icon::ChevronUp),
        _ => None,
    };
    let mut run = String::new();
    for c in keys.chars() {
        match symbol(c) {
            Some(icon) => {
                if !run.is_empty() {
                    add(role
                        .rich(tokens, std::mem::take(&mut run))
                        .extra_letter_spacing(spacing));
                }
                add(egui::RichText::new(char::from(icon).to_string()).font(crate::icons::font(size)));
            }
            None => run.push(c),
        }
    }
    if !run.is_empty() {
        add(role.rich(tokens, run).extra_letter_spacing(spacing));
    }
    ui.painter().layout_job(job)
}

pub(crate) fn popup_frame(tokens: &Tokens) -> egui::Frame {
    egui::Frame::new()
        .fill(ground(tokens, "dropdown-menu.base.rest.popup", "popover"))
        .stroke(Stroke::new(
            1.0,
            edge(tokens, "dropdown-menu.base.rest.popup", "border-strong"),
        ))
        .corner_radius(tokens.radius("surface"))
        .shadow(tokens.shadow("elevation-pop"))
}

/// A boxed dialog's surface (a dialog, the settings dialog, the palette): a popup's, wearing
/// `elevation-dialog`, or `elevation-pop` until a theme has the dialog's own.
pub(crate) fn dialog_frame(tokens: &Tokens) -> egui::Frame {
    let shadow = if tokens.shadows("elevation-dialog").is_empty() {
        tokens.shadow("elevation-pop")
    } else {
        tokens.shadow("elevation-dialog")
    };
    popup_frame(tokens).shadow(shadow)
}

/// A control's height on this window: `height`, and at least a finger on a phone, as the web's
/// `max-md:min-h-finger` (buttons, inputs, search fields, a select's trigger).
pub(crate) fn finger_tall(ctx: &egui::Context, tokens: &Tokens, height: f32) -> f32 {
    if is_phone(ctx) {
        height.max(tokens.length("spacing-finger"))
    } else {
        height
    }
}

/// How tall a one-line row of a list of choices (a menu, a select's list) is: a control tall, as
/// the density sets it, and a finger tall on a phone.
pub(crate) fn choice_row_height(ctx: &egui::Context, tokens: &Tokens) -> f32 {
    if is_phone(ctx) {
        tokens.length("spacing-finger")
    } else {
        tokens.density("control")
    }
}

/// A color as a disabled control shows it.
pub(crate) fn dim(color: Color32, enabled: bool) -> Color32 {
    if enabled { color } else { color.gamma_multiply(DISABLED) }
}

/// The busy spinner: three quarters of a ring turning, in `color`; asks for the next frame.
pub(crate) fn spinner(ui: &Ui, painter: &Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let start = ui.input(|i| i.time) as f32 * 6.0;
    let points: Vec<_> = (0..=24)
        .map(|i| {
            let a = start + i as f32 / 24.0 * std::f32::consts::TAU * 0.75;
            center + radius * egui::vec2(a.cos(), a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    ui.ctx().request_repaint();
}
pub use time_range::{RefreshAsked, RefreshControl, RefreshWords, TimeRangePicker, TimeRangeWords, range_label};
pub use variables::{variable_select, variable_toggle};
