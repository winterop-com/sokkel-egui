//! Importing data, as the web's `ImportDialog.tsx` does it: a CSV, TSV or JSON file picked, or rows
//! pasted, read at once; the columns' names and guessed types shown before anything is kept, each
//! one editable; and why it cannot be imported yet.

use crate::backend::{Backend, ImportSource};
use crate::js;
use crate::model::{Column, ColumnType, counted};
use crate::parse::{ParsedTable, columns_of, parse_table};

/// The rows the paste field shows before any are pasted.
pub const SAMPLE: &str =
    "Month,Harbor,Arrivals\n2025-01,Bergen,412\n2025-01,Cork,198\n2025-02,Bergen,437\n2025-02,Cork,224";

/// One column of the table being read: its name and type, and a few of its values.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ColumnLine {
    pub column: Column,
    pub at: usize,
    pub sample: String,
}

/// A file picked: its name and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Picked {
    pub name: String,
    pub content: String,
}

/// The dialog's state, as it starts each time it opens.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportDialog {
    pub source: ImportSource,
    pub file: Option<Picked>,
    pub text: String,
    pub table: Option<ParsedTable>,
    pub columns: Vec<Column>,
    pub name: String,
    pub error: Option<String>,
}

impl Default for ImportDialog {
    fn default() -> Self {
        ImportDialog {
            source: ImportSource::File,
            file: None,
            text: String::new(),
            table: None,
            columns: Vec::new(),
            name: String::new(),
            error: None,
        }
    }
}

/// The name a file's dataset takes: its name without its extension, dashes and underscores as
/// spaces.
pub fn title_of_file(name: &str) -> String {
    let stem = match name.rfind('.') {
        Some(at) if at + 1 < name.len() => &name[..at],
        _ => name,
    };
    let mut out = String::new();
    let mut gap = false;
    for c in stem.chars() {
        if c == '-' || c == '_' {
            gap = true;
        } else {
            if gap {
                out.push(' ');
                gap = false;
            }
            out.push(c);
        }
    }
    if gap {
        out.push(' ');
    }
    out
}

impl ImportDialog {
    /// Reads a text as a table, naming the dataset `title` unless it has a name already.
    pub fn read(&mut self, content: &str, title: &str) {
        if js::trim(content).is_empty() {
            self.table = None;
            self.error = None;
            return;
        }
        match parse_table(content) {
            Err(error) => {
                self.table = None;
                self.error = Some(error);
            }
            Ok((table, _)) => {
                self.error = None;
                self.columns = columns_of(&table);
                self.table = Some(table);
                if js::trim(&self.name).is_empty() {
                    self.name = title.to_owned();
                }
            }
        }
    }

    /// A file picked, or none.
    pub fn pick(&mut self, file: Option<Picked>) {
        self.file = file.clone();
        match file {
            None => {
                self.table = None;
                self.error = None;
            }
            Some(file) => self.read(&file.content, &title_of_file(&file.name)),
        }
    }

    /// Rows pasted.
    pub fn paste(&mut self, text: &str) {
        self.text = text.to_owned();
        self.read(text, "Pasted data");
    }

    /// The source switched: what it holds is read again.
    pub fn set_source(&mut self, source: ImportSource) {
        self.source = source;
        self.table = None;
        self.error = None;
        if source == ImportSource::File && self.file.is_some() {
            let file = self.file.clone();
            self.pick(file);
        }
        if source == ImportSource::Paste && !self.text.is_empty() {
            let text = self.text.clone();
            self.read(&text, "Pasted data");
        }
    }

    /// A column renamed or retyped.
    pub fn set_column(&mut self, at: usize, name: Option<&str>, kind: Option<ColumnType>) {
        if let Some(column) = self.columns.get_mut(at) {
            if let Some(name) = name {
                column.name = js::trim(name).to_owned();
            }
            if let Some(kind) = kind {
                column.kind = kind;
            }
        }
    }

    /// Why the table cannot be imported yet.
    pub fn why(&self) -> Option<String> {
        if let Some(error) = &self.error {
            return Some(error.clone());
        }
        if self.table.is_none() {
            return Some(
                match self.source {
                    ImportSource::File => "Pick a file to read first.",
                    ImportSource::Paste => "Paste rows to read first.",
                }
                .to_owned(),
            );
        }
        if js::trim(&self.name).is_empty() {
            return Some("The dataset needs a name.".to_owned());
        }
        let names: Vec<String> = self
            .columns
            .iter()
            .map(|column| js::trim(&column.name).to_lowercase())
            .collect();
        let clash = names
            .iter()
            .enumerate()
            .any(|(at, one)| one.is_empty() || names.iter().position(|other| other == one) != Some(at));
        clash.then(|| "Every column needs a name of its own.".to_owned())
    }

    /// The columns as the dialog lists them, each with its first three values.
    pub fn lines(&self) -> Vec<ColumnLine> {
        self.columns
            .iter()
            .enumerate()
            .map(|(at, column)| ColumnLine {
                column: column.clone(),
                at,
                sample: self
                    .table
                    .as_ref()
                    .map(|table| {
                        table
                            .rows
                            .iter()
                            .map(|row| row.get(at).map_or("", String::as_str))
                            .filter(|value| !value.is_empty())
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default(),
            })
            .collect()
    }

    /// The dialog's foot: how many rows were read.
    pub fn footer(&self) -> Option<String> {
        self.table
            .as_ref()
            .map(|table| format!("{} read", counted(table.rows.len(), "row")))
    }

    /// Keeps the table as a dataset; none where there is no table.
    pub fn save(&self, backend: &mut Backend) -> Option<String> {
        let table = self.table.as_ref()?;
        let file = self.file.as_ref().map(|file| file.name.as_str());
        Some(backend.import(&self.name, self.source, file, &self.columns, table))
    }
}
