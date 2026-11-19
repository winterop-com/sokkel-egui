//! A storage browser for Tidevann's buckets, an app of sokkel-egui's own: the buckets' folders
//! as a tree, a folder's files listed beside it, a file's contents previewed. It uses the library's
//! public parts only.

pub mod files;

use std::collections::HashSet;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountMenu, Appearance, Cell, Column, DataTable, DescriptionList, DesktopWindow, Empty, FileKind, FilePreview,
    Gap, Icon, Page, PageHeader, Rail, RailEntry, RailGroup, SearchField, Section, Shell, ShellPart, SplitPane, Stack,
    StatusLine, StatusTone, TableState, TableVariant, TopBar, Tree, TreeNode, format_size, install, tree_ancestors,
};

use files::{Entry, buckets, find};

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The storage browser: the buckets, the branches open, the entry chosen and the search.
pub struct StorageApp {
    pub appearance: Appearance,
    pub desktop: bool,
    pub buckets: Vec<Entry>,
    pub nodes: Vec<TreeNode>,
    pub open: HashSet<String>,
    /// The entry chosen in the tree or the listing, by its path.
    pub chosen: Option<String>,
    pub search: String,
    pub table: TableState,
}

impl StorageApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app
    }

    /// The browser with harbor-exports open and its newest export chosen.
    pub fn on(appearance: Appearance) -> Self {
        let buckets = buckets();
        let nodes = buckets.iter().map(Entry::node).collect();
        let mut app = Self {
            appearance,
            desktop: false,
            buckets,
            nodes,
            open: HashSet::new(),
            chosen: None,
            search: String::new(),
            table: TableState::default(),
        };
        app.choose("harbor-exports/nightly-export/2026-11-17");
        app
    }

    /// Choose an entry by its path, opening the folders above it.
    pub fn choose(&mut self, id: &str) {
        if let Some(above) = tree_ancestors(&self.nodes, id) {
            self.open.extend(above);
        }
        self.chosen = Some(id.to_owned());
        self.table = TableState::default();
    }

    /// The entry chosen.
    pub fn entry(&self) -> Option<&Entry> {
        self.chosen.as_deref().and_then(|id| find(&self.buckets, id))
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let groups = vec![RailGroup::new(
            None,
            [RailEntry::new("files", "Files", Icon::FolderTree).description("Every bucket's files")],
        )];
        let mut place = "files".to_owned();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let trail: Vec<String> = std::iter::once("Storage".to_owned())
            .chain(
                self.chosen
                    .iter()
                    .flat_map(|id| id.split('/').map(str::to_owned).collect::<Vec<_>>()),
            )
            .collect();
        let trail: Vec<&str> = trail.iter().map(String::as_str).collect();
        let files: usize = self.buckets.iter().map(Entry::file_count).sum();
        let bytes: u64 = self.buckets.iter().map(|b| b.bytes).sum();
        let mut account = None;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Storage", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada").place("tidevann.example.org").show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&trail).show(ui, |_| {});
            }
            ShellPart::Page => Page::show(ui, |ui| self.page(ui)),
            ShellPart::Foot => {
                ui.add(
                    StatusLine::new("Three buckets in tidevann.example.org")
                        .tone(StatusTone::Quiet)
                        .item("files", files)
                        .item("stored", format_size(bytes)),
                );
            }
        });
    }

    fn page(&mut self, ui: &mut Ui) {
        PageHeader::new("Files")
            .count(format!("{} buckets", self.buckets.len()))
            .show(ui, |_| {});
        let mut chosen = None;
        let nodes = &self.nodes;
        let open = &mut self.open;
        let selected = &mut self.chosen;
        let search = &mut self.search;
        let entry = selected.as_deref().and_then(|id| find(&self.buckets, id)).cloned();
        let table = &mut self.table;
        SplitPane::new("storage-split")
            .default_share(0.3)
            .min_share(0.2)
            .max_share(0.5)
            .show(
                ui,
                |ui| {
                    ui.vertical(|ui| {
                        SearchField::new("storage-search", search)
                            .placeholder("Search")
                            .keys("/")
                            .show(ui);
                        egui::ScrollArea::vertical().id_salt("storage-tree").show(ui, |ui| {
                            Tree::new("storage-tree", nodes, selected, open).query(search).show(ui);
                        });
                    });
                },
                |ui| {
                    egui::ScrollArea::vertical().id_salt("storage-side").show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(16, 0))
                            .show(ui, |ui| match &entry {
                                Some(entry) if entry.kind == FileKind::Folder => {
                                    chosen = folder(ui, entry, table);
                                }
                                Some(entry) => file(ui, entry),
                                None => {
                                    Empty::new("Pick a file")
                                        .icon(Icon::FolderOpen)
                                        .description("A folder's files or a file's contents stand here.")
                                        .show(ui, |_| {});
                                }
                            });
                    });
                },
            );
        if let Some(id) = chosen {
            self.choose(&id);
        }
    }
}

/// A folder's entries as a list; the one pressed is chosen.
fn folder(ui: &mut Ui, entry: &Entry, table: &mut TableState) -> Option<String> {
    let columns = [
        Column::new("Name", 220.0).fill(),
        Column::new("Size", 96.0).numeric(),
        Column::new("Modified", 140.0),
        Column::new("Owner", 96.0),
    ];
    let rows: Vec<Vec<Cell>> = entry
        .children
        .iter()
        .map(|c| {
            vec![
                Cell::Text(c.name.clone()),
                Cell::Text(format_size(c.bytes)),
                Cell::Text(c.modified.clone()),
                Cell::Person(c.owner.into()),
            ]
        })
        .collect();
    let mut chosen = None;
    Section::new(&entry.name).flush(true).show(ui, |ui| {
        DataTable::new("storage-folder", &columns, &rows, table)
            .variant(TableVariant::List)
            .footer(false)
            .show(ui);
        if let Some(row) = table.opened.take() {
            chosen = entry.children.get(row).map(|c| c.id.clone());
        }
    });
    chosen
}

/// A file's facts and what it holds.
fn file(ui: &mut Ui, entry: &Entry) {
    Stack::new(Gap::Section).show(ui, |ui| {
        Section::new(&entry.name).show(ui, |ui| {
            DescriptionList::new()
                .stacked()
                .text("Size", format_size(entry.bytes))
                .text("Modified", &entry.modified)
                .text("Owner", entry.owner)
                .text("Path", &entry.id)
                .show(ui);
        });
        let mut preview = FilePreview::new(("storage-preview", &entry.id), &entry.name, entry.kind);
        if let Some(text) = &entry.text {
            preview = preview.text(text);
        }
        preview.show(ui);
    });
}

impl eframe::App for StorageApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
