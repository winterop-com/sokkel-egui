//! A log's lines, after sokkel-ui's `LogView`.

/// How loud a line is, as sokkel-ui's `LogLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    /// The level as the log writes it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One line of a log, as sokkel-ui's `LogLine`.
#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    /// When it was written, already formatted: `14:02:11`.
    pub at: Option<String>,
    pub level: Option<LogLevel>,
    /// Where it came from: a step, a service, a logger.
    pub source: Option<String>,
    pub text: String,
    /// Extras after the text in muted ink, as `key=value` pairs.
    pub fields: Vec<(String, String)>,
}

impl LogLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            at: None,
            level: None,
            source: None,
            text: text.into(),
            fields: Vec::new(),
        }
    }
}

/// The words a log says.
#[derive(Clone, Debug)]
pub struct LogViewWords {
    /// Said while the log has no lines.
    pub empty: String,
    /// The button that scrolls back to the newest line once the reader has scrolled away from it.
    pub jump: String,
}

impl Default for LogViewWords {
    fn default() -> Self {
        Self {
            empty: "Nothing logged yet.".into(),
            jump: "Jump to newest".into(),
        }
    }
}

/// A log on the terminal ground, as sokkel-ui's `LogView`: time, level, source and text in
/// columns, levels inked, lines numbered, in the mono face 20 to a line. `follow` keeps the newest
/// line in view as lines arrive, until the reader scrolls up, and then offers Jump to newest.
/// Long lines scroll sideways. `framed` false drops its rounded corners, for a log set edge to
/// edge in a panel.
pub struct LogView<'a> {
    id: egui::Id,
    lines: &'a [LogLine],
    follow: bool,
    numbered: bool,
    framed: bool,
    height: f32,
    grow: bool,
    /// As tall as its lines.
    fit: bool,
    words: LogViewWords,
}

impl<'a> LogView<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, lines: &'a [LogLine]) -> Self {
        Self {
            id: egui::Id::unique(("sokkel-log", id)),
            lines,
            follow: false,
            numbered: true,
            framed: true,
            height: 288.0,
            grow: false,
            fit: false,
            words: LogViewWords::default(),
        }
    }

    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn numbered(mut self, numbered: bool) -> Self {
        self.numbered = numbered;
        self
    }

    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// How tall the log stands; 288 (`h-72`) by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// As tall as its lines (20 each, 8 over and under them), as the web's log with no height of
    /// its own given.
    pub fn fit(mut self, fit: bool) -> Self {
        self.fit = fit;
        self
    }

    /// Grows past its height into the room a stretched dashboard panel has, as the web's `grow`
    /// with `h-72` its basis: 288 alone, as tall as its row beside a taller panel.
    pub fn grow(mut self, grow: bool) -> Self {
        self.grow = grow;
        self
    }

    pub fn words(mut self, words: LogViewWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) {
        use egui::{Align2, Rect, Sense, pos2, vec2};
        let t = crate::tokens(ui.ctx());
        let mono = crate::mono_font(
            ui.ctx(),
            crate::mono_small_size(&t, crate::TextRole::Help.font(&t).size),
            400.0,
        );
        let width = ui.available_width();
        let extra = if self.grow { crate::body_room(ui) } else { 0.0 };
        if extra > 0.0 {
            crate::took_room(ui, extra);
        }
        let tall = if self.fit {
            self.lines.len() as f32 * 20.0 + 16.0
        } else {
            self.height + extra
        };
        let (outer, _) = ui.allocate_exact_size(vec2(width, tall), Sense::hover());
        let radius = if self.framed {
            t.radius("surface")
        } else {
            egui::CornerRadius::ZERO
        };
        ui.painter().rect_filled(outer, radius, t.color("terminal"));
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(outer).id_salt(self.id));
        inside.set_clip_rect(outer.intersect(ui.clip_rect()));
        if self.lines.is_empty() {
            inside.painter().text(
                pos2(outer.left() + 12.0, outer.top() + 8.0 + 10.0),
                Align2::LEFT_CENTER,
                &self.words.empty,
                mono,
                t.color("terminal-muted"),
            );
            return;
        }
        let measure = |text: &str| {
            inside
                .painter()
                .layout_no_wrap(text.to_owned(), mono.clone(), egui::Color32::WHITE)
                .size()
                .x
        };
        // every line keeps the columns any line has, so they stay aligned
        let at_w = self
            .lines
            .iter()
            .filter_map(|l| l.at.as_deref())
            .map(measure)
            .fold(0.0, f32::max);
        let has_level = self.lines.iter().any(|l| l.level.is_some());
        let source_w = self
            .lines
            .iter()
            .filter_map(|l| l.source.as_deref())
            .map(measure)
            .fold(0.0, f32::max);
        let number_w = if self.numbered { 40.0 } else { 12.0 };
        let gap = 12.0;
        let columns = number_w
            + if at_w > 0.0 { at_w + gap } else { 0.0 }
            + if has_level { 56.0 } else { 0.0 }
            + if source_w > 0.0 { source_w + gap } else { 0.0 };
        let texts: Vec<String> = self
            .lines
            .iter()
            .map(|l| {
                if l.fields.is_empty() {
                    l.text.clone()
                } else {
                    let fields: Vec<String> = l.fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
                    format!("{}  {}", l.text, fields.join(" "))
                }
            })
            .collect();
        let widest = texts.iter().map(|s| measure(s)).fold(0.0, f32::max);
        let row = 20.0;
        let content = vec2(
            (columns + widest + 12.0).max(width),
            self.lines.len() as f32 * row + 16.0,
        );
        let jump_key = self.id.with("jump");
        let jump = inside.data_mut(|d| d.remove_temp::<bool>(jump_key)).unwrap_or(false);
        let mut area = egui::ScrollArea::both()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .stick_to_bottom(self.follow);
        if jump {
            area = area.vertical_scroll_offset(f32::MAX);
        }
        let output = area.show(&mut inside, |ui| {
            let (rect, _) = ui.allocate_exact_size(content, Sense::hover());
            let first = ((ui.clip_rect().top() - rect.top() - 8.0) / row).floor().max(0.0) as usize;
            let last = (((ui.clip_rect().bottom() - rect.top()) / row).ceil() as usize).min(self.lines.len());
            let hover = ui.input(|i| i.pointer.hover_pos());
            for index in first..last {
                let line = &self.lines[index];
                let top = rect.top() + 8.0 + index as f32 * row;
                let band = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), row));
                let painter = ui.painter();
                if hover.is_some_and(|p| band.contains(p) && ui.clip_rect().contains(p)) {
                    painter.rect_filled(band, 0.0, t.color("terminal-bar"));
                }
                let middle = band.center().y;
                let mut x = band.left();
                if self.numbered {
                    painter.text(
                        pos2(x + 40.0 - 12.0, middle),
                        Align2::RIGHT_CENTER,
                        (index + 1).to_string(),
                        mono.clone(),
                        t.color("terminal-faint"),
                    );
                }
                x += number_w;
                if at_w > 0.0 {
                    if let Some(at) = &line.at {
                        painter.text(
                            pos2(x, middle),
                            Align2::LEFT_CENTER,
                            at,
                            mono.clone(),
                            t.color("terminal-muted"),
                        );
                    }
                    x += at_w + gap;
                }
                if has_level {
                    if let Some(level) = line.level {
                        let (word, ink) = match level {
                            LogLevel::Debug => ("DEBUG", "terminal-faint"),
                            LogLevel::Info => ("INFO", "terminal-muted"),
                            LogLevel::Warning => ("WARN", "terminal-warning"),
                            LogLevel::Error => ("ERROR", "terminal-critical"),
                        };
                        painter.text(pos2(x, middle), Align2::LEFT_CENTER, word, mono.clone(), t.color(ink));
                    }
                    x += 56.0;
                }
                if source_w > 0.0 {
                    if let Some(source) = &line.source {
                        painter.text(
                            pos2(x, middle),
                            Align2::LEFT_CENTER,
                            source,
                            mono.clone(),
                            t.color("terminal-accent"),
                        );
                    }
                    x += source_w + gap;
                }
                let ink = if line.level == Some(LogLevel::Error) {
                    t.color("terminal-critical")
                } else {
                    t.color("terminal-foreground")
                };
                let text = painter.text(pos2(x, middle), Align2::LEFT_CENTER, &line.text, mono.clone(), ink);
                if !line.fields.is_empty() {
                    let fields: Vec<String> = line.fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
                    painter.text(
                        pos2(text.right() + 8.0, middle),
                        Align2::LEFT_CENTER,
                        fields.join(" "),
                        mono.clone(),
                        t.color("terminal-faint"),
                    );
                }
            }
        });
        // a following log the reader scrolled up from offers the way back
        let room = output.inner_rect.height();
        let away = output.state.clamped_offset().y < content.y - room - 8.0;
        if self.follow && away {
            let galley = inside.painter().layout_no_wrap(
                self.words.jump.clone(),
                crate::TextRole::Help.font(&t),
                t.color("terminal-foreground"),
            );
            let size = vec2(12.0 + 14.0 + 6.0 + galley.size().x + 12.0, 24.0);
            let spot = Rect::from_min_size(outer.right_bottom() - size - vec2(12.0, 12.0), size);
            let response = inside.interact(spot, self.id.with("jump-button"), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.jump));
            let ground = if response.hovered() {
                t.color_at("terminal-muted", 0.3)
            } else {
                t.color("terminal-bar")
            };
            let painter = inside.painter();
            painter.rect_filled(spot, egui::CornerRadius::same(12), ground);
            crate::icons::paint(
                painter,
                pos2(spot.left() + 12.0 + 7.0, spot.center().y),
                crate::Icon::ArrowDown,
                14.0,
                t.color("terminal-foreground"),
            );
            painter.galley(
                pos2(spot.left() + 12.0 + 14.0 + 6.0, spot.center().y - galley.size().y / 2.0),
                galley,
                t.color("terminal-foreground"),
            );
            if response.clicked() {
                inside.data_mut(|d| d.insert_temp(jump_key, true));
                inside.ctx().request_repaint();
            }
        }
    }
}
