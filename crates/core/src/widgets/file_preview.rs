//! What a file holds, after sokkel-ui's `FilePreview` and the kinds of its `FileList`.

use egui::{FontId, Frame, Id, Margin, RichText, ScrollArea, Stroke, Ui};

use crate::{Icon, TextRole, icons, tokens};

use super::{Cell, Column, DataTable, TableState, TableVariant};

/// What kind of thing a file is, as sokkel-ui's `FileKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    Folder,
    Image,
    Table,
    Json,
    Text,
    Code,
    Pdf,
    Archive,
    Other,
}

impl FileKind {
    /// The kind a file's name says, by its extension.
    pub fn of_name(name: &str) -> Self {
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Self::Image,
            "csv" | "tsv" => Self::Table,
            "json" => Self::Json,
            "txt" | "md" | "log" => Self::Text,
            "rs" | "ts" | "tsx" | "js" | "py" | "sql" | "toml" | "yaml" | "yml" | "sh" => Self::Code,
            "pdf" => Self::Pdf,
            "zip" | "gz" | "tar" | "tgz" | "zst" => Self::Archive,
            _ => Self::Other,
        }
    }

    /// Its icon, as sokkel-ui's `FileTypeIcon`.
    pub fn icon(self) -> Icon {
        match self {
            Self::Folder => Icon::Folder,
            Self::Image => Icon::FileImage,
            Self::Table => Icon::FileSpreadsheet,
            Self::Json => Icon::FileBraces,
            Self::Text | Self::Pdf => Icon::FileText,
            Self::Code => Icon::FileCode,
            Self::Archive => Icon::FileArchive,
            Self::Other => Icon::File,
        }
    }

    /// The color token its icon is drawn in.
    pub fn tone(self) -> &'static str {
        match self {
            Self::Folder => "kind-blue-ink",
            Self::Image => "kind-pink-ink",
            Self::Table => "kind-green-ink",
            Self::Json => "kind-violet-ink",
            Self::Code => "kind-teal-ink",
            Self::Pdf => "critical-ink",
            Self::Archive => "warning-ink",
            Self::Text | Self::Other => "muted-foreground",
        }
    }
}

/// A byte count in the largest unit that keeps it at one or more, as sokkel-ui's `formatSize`:
/// `812 B`, `14.2 KB`, `3.1 MB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{} {}", value.round(), UNITS[unit])
    }
}

/// The cells of a CSV text, as sokkel-ui's `parseCsv`: commas split, double quotes hold commas and
/// doubled quotes; it stops after `limit` rows.
pub fn parse_csv(text: &str, limit: usize) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' && chars.peek() == Some(&'"') {
                cell.push('"');
                chars.next();
            } else if c == '"' {
                quoted = false;
            } else {
                cell.push(c);
            }
        } else if c == '"' {
            quoted = true;
        } else if c == ',' {
            row.push(std::mem::take(&mut cell));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut cell));
            rows.push(std::mem::take(&mut row));
            if rows.len() >= limit {
                return rows;
            }
        } else {
            cell.push(c);
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// The words a preview says.
#[derive(Clone, Debug)]
pub struct FilePreviewWords {
    /// Under a table: the rows shown of the rows there are.
    pub rows: fn(usize, usize) -> String,
    /// Under a table cut off at its most rows.
    pub first_rows: fn(usize) -> String,
    /// Said for a kind with no preview.
    pub none: String,
}

impl Default for FilePreviewWords {
    fn default() -> Self {
        Self {
            rows: |shown, total| {
                if shown == total {
                    format!("{total} rows")
                } else {
                    format!("First {shown} of {total} rows")
                }
            },
            first_rows: |shown| format!("First {shown} rows"),
            none: "No preview for this kind of file.".into(),
        }
    }
}

/// What a file holds, drawn for its kind, as sokkel-ui's `FilePreview`: a CSV as a small table of
/// its first rows, JSON, text and code as their lines numbered under the file's name, up to 448
/// tall and scrolling past it. Any other kind shows its icon and says it has no preview.
pub struct FilePreview<'a> {
    id: Id,
    name: &'a str,
    kind: FileKind,
    text: Option<&'a str>,
    max_rows: usize,
    words: FilePreviewWords,
}

impl<'a> FilePreview<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, name: &'a str, kind: FileKind) -> Self {
        Self {
            id: Id::unique(id),
            name,
            kind,
            text: None,
            max_rows: 12,
            words: FilePreviewWords::default(),
        }
    }

    /// The contents of a table, JSON, text or code file.
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    /// How many rows of a CSV the table shows.
    pub fn max_rows(mut self, rows: usize) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    pub fn words(mut self, words: FilePreviewWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        match (self.kind, self.text) {
            (FileKind::Table, Some(text)) => self.table(ui, text),
            (FileKind::Json | FileKind::Text | FileKind::Code, Some(text)) => self.code(ui, text),
            _ => self.none(ui),
        }
    }

    fn table(&self, ui: &mut Ui, text: &str) {
        let t = tokens(ui.ctx());
        let parsed = parse_csv(text, self.max_rows + 2);
        let total = text.lines().filter(|l| !l.is_empty()).count().saturating_sub(1);
        let mut parsed = parsed.into_iter();
        let head = parsed.next().unwrap_or_default();
        let body: Vec<Vec<String>> = parsed.take(self.max_rows).collect();
        let number = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-'));
        let columns: Vec<Column> = head
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let column = Column::new(name.clone(), 112.0).fill();
                let cells = body.iter().filter_map(|r| r.get(i)).filter(|c| !c.is_empty());
                if cells.clone().count() > 0 && cells.clone().all(|c| number(c)) {
                    column.numeric()
                } else {
                    column
                }
            })
            .collect();
        let rows: Vec<Vec<Cell>> = body
            .iter()
            .map(|r| {
                (0..head.len())
                    .map(|i| Cell::Text(r.get(i).cloned().unwrap_or_default()))
                    .collect()
            })
            .collect();
        let mut state: TableState = ui.data_mut(|d| d.get_temp(self.id).unwrap_or_default());
        DataTable::new(self.id.with("table"), &columns, &rows, &mut state)
            .variant(TableVariant::List)
            .footer(false)
            .show(ui);
        ui.data_mut(|d| d.insert_temp(self.id, state));
        let foot = if total > rows.len() {
            (self.words.first_rows)(rows.len())
        } else {
            (self.words.rows)(rows.len(), total)
        };
        ui.add_space(8.0);
        ui.label(
            RichText::new(foot)
                .font(FontId::monospace(TextRole::Help.font(&t).size))
                .color(t.color("muted-foreground")),
        );
    }

    fn code(&self, ui: &mut Ui, text: &str) {
        let t = tokens(ui.ctx());
        let mono = FontId::monospace(TextRole::Help.font(&t).size);
        Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(t.radius("surface"))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                // the file's name over its lines
                let (head, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::hover());
                let painter = ui.painter();
                icons::paint(
                    painter,
                    egui::pos2(head.left() + 12.0 + 7.0, head.center().y),
                    self.kind.icon(),
                    14.0,
                    t.color(self.kind.tone()),
                );
                painter.text(
                    egui::pos2(head.left() + 12.0 + 14.0 + 8.0, head.center().y),
                    egui::Align2::LEFT_CENTER,
                    self.name,
                    TextRole::Help.font(&t),
                    t.color("muted-foreground"),
                );
                painter.hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                // the numbers and the lines as two blocks side by side
                let count = text.lines().count().max(1);
                let digits = count.to_string().len();
                let numbers: Vec<String> = (1..=count).map(|n| format!("{n:>digits$}")).collect();
                let lines: Vec<&str> = text.lines().collect();
                let line_height = TextRole::Help.font(&t).size * 1.6;
                ScrollArea::both()
                    .id_salt(self.id.with("lines"))
                    .max_height(448.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        Frame::new().inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 16.0;
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(numbers.join("\n"))
                                            .font(mono.clone())
                                            .line_height(Some(line_height))
                                            .color(t.color("muted-foreground")),
                                    )
                                    .extend(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(lines.join("\n"))
                                            .font(mono.clone())
                                            .line_height(Some(line_height))
                                            .color(t.color("foreground")),
                                    )
                                    .extend(),
                                );
                            });
                        });
                    });
            });
    }

    fn none(&self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 120.0), egui::Sense::hover());
        let painter = ui.painter();
        // a dashed edge, as sokkel-ui's `border-dashed`
        let stroke = Stroke::new(1.0, t.color("border-strong"));
        let r = rect.shrink(0.5);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ];
        for pair in corners.windows(2) {
            painter.extend(egui::Shape::dashed_line(pair, stroke, 4.0, 3.0));
        }
        let center = rect.center();
        icons::paint(
            painter,
            center - egui::vec2(0.0, 14.0),
            self.kind.icon(),
            32.0,
            t.color(self.kind.tone()),
        );
        painter.text(
            center + egui::vec2(0.0, 22.0),
            egui::Align2::CENTER_CENTER,
            &self.words.none,
            TextRole::Body.font(&t),
            t.color("muted-foreground"),
        );
    }
}
