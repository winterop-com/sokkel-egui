//! The journal, as the web's `pages/Daily.tsx`: today's note a press away, a month to jump to any
//! day, and the days written so far.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, DataTable, Day, Empty, Icon, Markdown, MarkdownScale, MiniMonth,
    PageHeader, Section, StackLayout, TableState, TableVariant, Toast, tokens,
};

use crate::all_notes::note_menu;
use crate::app::NotesApp;
use crate::backend::Notice;
use crate::model::{Note, date_of, day, from_template, long_day, word_count};

/// The day column's sort, newest first.
const DAY: usize = 0;

/// Says what the backend did, as a toast.
pub fn notify(ctx: &egui::Context, notice: Notice) {
    let toast = if notice.success {
        Toast::success(notice.title)
    } else {
        Toast::new(notice.title)
    };
    match notice.description {
        Some(description) => toast.description(description).show(ctx),
        None => toast.show(ctx),
    };
}

impl NotesApp {
    pub fn daily(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let now = self.now();
        let today = day(0, now);
        let today_day = date_of(&today).map(|d| Day(d as i32));
        let vault = self.backend.vault();
        let prefs = &self.backend.prefs;
        let mut days: Vec<Note> = vault.notes.iter().filter(|n| n.day.is_some()).cloned().collect();
        days.sort_by(|a, b| b.day.cmp(&a.day));
        let written = days.iter().any(|n| n.day.as_deref() == Some(today.as_str()));
        let folder = match prefs.daily_folder.as_deref() {
            None => "the top of the vault".to_owned(),
            Some(id) => vault.path(Some(id)),
        };
        let marked: Vec<Day> = days
            .iter()
            .filter_map(|n| date_of(n.day.as_deref()?).map(|d| Day(d as i32)))
            .collect();
        let template = from_template(&prefs.template, &today);
        let columns = vec![
            Column::new("Day", 120.0).sortable(true),
            Column::new("Date", 192.0).fill(),
            Column::new("Words", 96.0).numeric().sortable(true),
            Column::new("Links out", 104.0).numeric(),
        ];
        let rows: Vec<Vec<Cell>> = days
            .iter()
            .map(|n| {
                let at = n.day.clone().unwrap_or_else(|| today.clone());
                let words = word_count(&n.body);
                let links = vault.links(&n.id).len();
                vec![
                    Cell::Text(at.clone()),
                    Cell::Text(long_day(&at)),
                    Cell::Number {
                        value: words as f64,
                        text: words.to_string(),
                    },
                    Cell::Number {
                        value: links as f64,
                        text: links.to_string(),
                    },
                ]
            })
            .collect();
        let keys: Vec<String> = days.iter().map(|n| n.id.clone()).collect();

        let mut template_asked = false;
        let mut start = false;
        PageHeader::new("Daily")
            .count(format!("{} days", days.len()))
            .description(format!("A note a day, made from the template in {folder}."))
            .show(ui, |ui| {
                start = ui
                    .add(
                        Button::new(if written {
                            "Open today's note"
                        } else {
                            "Start today's note"
                        })
                        .icon(Icon::CalendarPlus)
                        .size(ButtonSize::Sm),
                    )
                    .clicked();
                template_asked = ui
                    .add(
                        Button::new("Template")
                            .icon(Icon::Settings)
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
            });

        let gap = t.density("section-gap");
        let mut month = self.daily_month.unwrap_or_else(|| today_day.unwrap_or(Day(0)));
        let mut picked = None;
        let mut opened = None;
        let mut action = None;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            ui.allocate_ui_with_layout(egui::vec2(288.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(288.0);
                Section::new("Jump to a day")
                    .description("A day with a note carries a dot; any other starts one.")
                    .show(ui, |ui| {
                        let mut mini = MiniMonth::new("daily-month", &mut month).marked(&marked);
                        if let Some(today) = today_day {
                            mini = mini.today(today).value(Some(today));
                        }
                        picked = mini.show(ui).chosen;
                    });
                Section::new("Template")
                    .description("What a new day's note starts with; {{date}} is the day.")
                    .show(ui, |ui| {
                        Markdown::new(&template).scale(MarkdownScale::Message).show(ui);
                    });
            });
            ui.vertical(|ui| {
                Section::new("Days written").flush(true).show(ui, |ui| {
                    let state = self
                        .tables
                        .entry("daily".to_owned())
                        .or_insert_with(|| TableState::sorted(DAY, false));
                    let out = DataTable::new("daily", &columns, &rows, state)
                        .variant(TableVariant::List)
                        .fits(true)
                        .stack_layout(StackLayout::Compact)
                        .keys(keys.iter())
                        .actions(|_| note_menu())
                        .height(f32::INFINITY)
                        .empty(|ui| {
                            Empty::new("No day written yet. Start today's note.").show(ui, |_| {});
                        })
                        .footer(false)
                        .show(ui);
                    opened = state.opened.take();
                    action = out.action;
                });
            });
        });
        self.daily_month = Some(month);

        let ctx = ui.ctx().clone();
        if start {
            self.open_day(&ctx, &today);
        }
        if template_asked {
            self.go("settings");
        }
        if let Some(day) = picked {
            let (y, m, d) = day.ymd();
            self.open_day(&ctx, &format!("{y}-{m:02}-{d:02}"));
        }
        if let Some(id) = opened.and_then(|row| keys.get(row)) {
            self.open_note(&id.clone());
        }
        if let Some((row, index)) = action
            && let Some(id) = keys.get(row).cloned()
        {
            self.note_action(&id, index);
        }
    }

    /// Opens a day's note, making it from the template first when there is none.
    pub fn open_day(&mut self, ctx: &egui::Context, date: &str) {
        let now = self.now();
        let (id, notice) = self.backend.open_day(date, now);
        if let Some(notice) = notice {
            notify(ctx, notice);
        }
        self.open_note(&id);
    }
}
