//! Tidevann's buckets, simulated: three projects' files as the jobs leave them, the same every run.

use sokkel_egui::{FileKind, Icon, TreeNode, format_size};

/// A file or a folder, by its path from the bucket down.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Its path, such as `harbor-exports/nightly-export/2026-11-13/orders.csv`.
    pub id: String,
    pub name: String,
    pub kind: FileKind,
    pub bytes: u64,
    /// When it last changed, as written.
    pub modified: String,
    pub owner: &'static str,
    pub children: Vec<Entry>,
    /// The contents of a file that can be read.
    pub text: Option<String>,
}

impl Entry {
    fn folder(parent: &str, name: &str, owner: &'static str, children: Vec<Entry>) -> Self {
        let id = join(parent, name);
        let modified = children.iter().map(|c| c.modified.clone()).max().unwrap_or_default();
        Self {
            id,
            name: name.into(),
            kind: FileKind::Folder,
            bytes: children.iter().map(|c| c.bytes).sum(),
            modified,
            owner,
            children,
            text: None,
        }
    }

    fn file(name: &str, owner: &'static str, modified: &str, text: Option<String>, bytes: u64) -> Self {
        Self {
            id: name.into(),
            name: name.into(),
            kind: FileKind::of_name(name),
            bytes: text.as_ref().map(|t| t.len() as u64).unwrap_or(bytes),
            modified: modified.into(),
            owner,
            children: Vec::new(),
            text,
        }
    }

    /// The entry with its path, and its children's, under `parent`.
    fn under(mut self, parent: &str) -> Self {
        self.id = join(parent, &self.name);
        let id = self.id.clone();
        self.children = self.children.into_iter().map(|c| c.under(&id)).collect();
        self
    }

    /// The files under it, folders counted by what they hold.
    pub fn file_count(&self) -> usize {
        if self.kind == FileKind::Folder {
            self.children.iter().map(Entry::file_count).sum()
        } else {
            1
        }
    }

    /// The entry as a tree node: a folder with its count, a file with its size.
    pub fn node(&self) -> TreeNode {
        let node = TreeNode::new(&self.id, &self.name);
        if self.kind == FileKind::Folder {
            node.icon(Icon::Folder)
                .open_icon(Icon::FolderOpen)
                .meta(self.file_count().to_string())
                .children(self.children.iter().map(Entry::node).collect())
        } else {
            node.icon(self.kind.icon()).meta(format_size(self.bytes))
        }
    }
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.into()
    } else {
        format!("{parent}/{name}")
    }
}

/// The entry at a path, anywhere in the buckets.
pub fn find<'a>(buckets: &'a [Entry], id: &str) -> Option<&'a Entry> {
    for entry in buckets {
        if entry.id == id {
            return Some(entry);
        }
        if id.starts_with(&format!("{}/", entry.id))
            && let Some(found) = find(&entry.children, id)
        {
            return Some(found);
        }
    }
    None
}

fn orders(day: u32) -> String {
    let mut text = String::from("order,customer,site,items,total\n");
    for n in 0..18u32 {
        let id = 40_000 + day * 100 + n;
        let site = ["Bergen", "Lisbon", "Osaka", "Quito"][(n % 4) as usize];
        let items = 1 + (n * 7 + day) % 9;
        let total = items * 1250 + (n * 37 % 100);
        text.push_str(&format!(
            "{id},c-{:04},{site},{items},{}.{:02}\n",
            100 + n * 13 % 97,
            total / 100,
            total % 100
        ));
    }
    text
}

fn summary(day: u32) -> String {
    format!(
        "{{\n  \"job\": \"nightly-export\",\n  \"run\": \"run-11{day:02}-0200\",\n  \"state\": \"succeeded\",\n  \"rows\": 18,\n  \"steps\": [\"fetch\", \"transform\", \"validate\", \"upload\", \"notify\"],\n  \"seconds\": {}\n}}\n",
        2400 + day * 13
    )
}

fn run_log(day: u32) -> String {
    let steps = ["fetch", "transform", "validate", "upload", "notify"];
    let mut text = String::new();
    for (i, step) in steps.iter().enumerate() {
        text.push_str(&format!(
            "2026-11-{day:02}T02:{:02}:{:02}Z info  {step} started\n",
            i * 8,
            (i * 17) % 60
        ));
        text.push_str(&format!(
            "2026-11-{day:02}T02:{:02}:{:02}Z info  {step} done\n",
            i * 8 + 6,
            (i * 29) % 60
        ));
    }
    text
}

fn stock() -> String {
    let mut text = String::from("sku,warehouse,on_hand,reserved\n");
    for n in 0..30u32 {
        let warehouse = ["oslo-1", "porto-2", "kobe-1"][(n % 3) as usize];
        text.push_str(&format!(
            "sku-{:05},{warehouse},{},{}\n",
            1000 + n * 37,
            (n * 53) % 400,
            (n * 11) % 40
        ));
    }
    text
}

/// The three buckets as they stand on Wednesday 18 November 2026.
pub fn buckets() -> Vec<Entry> {
    let days = |owner: &'static str| {
        (12..=17u32)
            .rev()
            .map(|day| {
                let at = format!("2026-11-{day:02} 02:4{}", day % 10);
                Entry::folder(
                    "",
                    &format!("2026-11-{day:02}"),
                    owner,
                    vec![
                        Entry::file("orders.csv", owner, &at, Some(orders(day)), 0),
                        Entry::file("summary.json", owner, &at, Some(summary(day)), 0),
                        Entry::file("run.log", owner, &at, Some(run_log(day)), 0),
                    ],
                )
            })
            .collect::<Vec<_>>()
    };
    let buckets = vec![
        Entry::folder(
            "",
            "harbor-exports",
            "ada",
            vec![
                Entry::folder("", "nightly-export", "ada", days("ada")),
                Entry::file(
                    "README.md",
                    "ada",
                    "2026-10-02 10:14",
                    Some(
                        "# harbor-exports\n\nnightly-export writes a folder a night: the orders as CSV,\na summary of the run, and its log.\n\nKept for 30 days.\n"
                            .into(),
                    ),
                    0,
                ),
            ],
        ),
        Entry::folder(
            "",
            "juniper-inventory",
            "grace",
            vec![
                Entry::folder(
                    "",
                    "sync-inventory",
                    "grace",
                    vec![
                        Entry::file("stock.csv", "grace", "2026-11-18 06:50", Some(stock()), 0),
                        Entry::file(
                            "warehouses.json",
                            "grace",
                            "2026-11-02 09:00",
                            Some(
                                "[\n  { \"id\": \"oslo-1\", \"city\": \"Oslo\" },\n  { \"id\": \"porto-2\", \"city\": \"Porto\" },\n  { \"id\": \"kobe-1\", \"city\": \"Kobe\" }\n]\n"
                                    .into(),
                            ),
                            0,
                        ),
                    ],
                ),
                Entry::folder(
                    "",
                    "photos",
                    "grace",
                    vec![
                        Entry::file("shelf-a.jpg", "grace", "2026-11-09 14:22", None, 2_412_000),
                        Entry::file("shelf-b.jpg", "grace", "2026-11-09 14:23", None, 2_380_500),
                    ],
                ),
            ],
        ),
        Entry::folder(
            "",
            "larkspur-index",
            "linus",
            vec![
                Entry::folder(
                    "",
                    "rebuild-index",
                    "linus",
                    vec![
                        Entry::file("index.tar.gz", "linus", "2026-11-16 04:30", None, 184_200_000),
                        Entry::file(
                            "manifest.json",
                            "linus",
                            "2026-11-16 04:30",
                            Some("{\n  \"documents\": 48213,\n  \"shards\": 6,\n  \"built\": \"2026-11-16T04:30:00Z\"\n}\n".into()),
                            0,
                        ),
                    ],
                ),
                Entry::folder(
                    "",
                    "config",
                    "linus",
                    vec![Entry::file(
                        "schedule.toml",
                        "linus",
                        "2026-09-21 11:05",
                        Some("[rebuild-index]\ndays = [\"mon\", \"thu\"]\nat = \"03:00\"\nminutes = 90\n".into()),
                        0,
                    )],
                ),
            ],
        ),
    ];
    buckets.into_iter().map(|b| b.under("")).collect()
}
