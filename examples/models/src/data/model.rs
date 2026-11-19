//! What the Models app keeps, as the web's `data/model.ts` has it: the models on its disk with
//! their files, the conversations had with them, the downloads under way, the runner and the models
//! it holds in memory, the side-by-side comparisons, the runner's log and the preferences.

use serde::{Deserialize, Serialize};
use sokkel_egui::Reaction;

/// How a model's weights are written on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Packed,
    Unified,
    Sharded,
}

pub const FORMATS: [Format; 3] = [Format::Packed, Format::Unified, Format::Sharded];

impl Format {
    pub fn key(self) -> &'static str {
        match self {
            Format::Packed => "packed",
            Format::Unified => "unified",
            Format::Sharded => "sharded",
        }
    }

    /// What the format is, said where a format is chosen or explained.
    pub fn note(self) -> &'static str {
        match self {
            Format::Packed => "One file of rounded weights the runner reads as it is",
            Format::Unified => "Rounded weights laid out for memory the processor and graphics share",
            Format::Sharded => "The full weights, split into shards of a few gigabytes",
        }
    }

    pub fn from_key(key: &str) -> Option<Format> {
        FORMATS.into_iter().find(|one| one.key() == key)
    }
}

/// Where a model came from: pulled from a public hub, or taken in from a runner's own store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Hub,
    Runner,
    Desktop,
}

pub const SOURCE_IDS: [Source; 3] = [Source::Hub, Source::Runner, Source::Desktop];

impl Source {
    pub fn key(self) -> &'static str {
        match self {
            Source::Hub => "hub",
            Source::Runner => "runner",
            Source::Desktop => "desktop",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::Hub => "Public hub",
            Source::Runner => "Local runner",
            Source::Desktop => "Desktop runner",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Source::Hub => "hub.example.org, the public catalog of open weights",
            Source::Runner => "The runner's own store of pulled models",
            Source::Desktop => "A desktop runner's folder of downloads",
        }
    }

    pub fn from_key(key: &str) -> Option<Source> {
        SOURCE_IDS.into_iter().find(|one| one.key() == key)
    }
}

/// What a model can do beyond writing text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Tools,
    Vision,
    Code,
    Reasoning,
}

pub const CAPABILITIES: [Capability; 4] = [
    Capability::Tools,
    Capability::Vision,
    Capability::Code,
    Capability::Reasoning,
];

impl Capability {
    pub fn key(self) -> &'static str {
        match self {
            Capability::Tools => "tools",
            Capability::Vision => "vision",
            Capability::Code => "code",
            Capability::Reasoning => "reasoning",
        }
    }
}

/// What a file of a model is for: the weights the runner loads, a vision projector, the tokenizer,
/// its config or its card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    Weights,
    Projector,
    Tokenizer,
    Config,
    Card,
}

impl FileRole {
    pub fn key(self) -> &'static str {
        match self {
            FileRole::Weights => "weights",
            FileRole::Projector => "projector",
            FileRole::Tokenizer => "tokenizer",
            FileRole::Config => "config",
            FileRole::Card => "card",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelFile {
    pub name: String,
    /// Bytes.
    pub size: f64,
    pub role: FileRole,
}

/// The settings an answer is written with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sampling {
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: f64,
    pub system: String,
}

pub fn default_sampling() -> Sampling {
    Sampling {
        temperature: 0.7,
        top_p: 0.9,
        max_tokens: 1024.0,
        system: "You are a helpful assistant. Answer plainly and say when you are not sure.".to_owned(),
    }
}

/// A change to some of the settings, as `Partial<Sampling>`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplingPatch {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<f64>,
    pub system: Option<String>,
}

impl SamplingPatch {
    pub fn all(settings: &Sampling) -> Self {
        SamplingPatch {
            temperature: Some(settings.temperature),
            top_p: Some(settings.top_p),
            max_tokens: Some(settings.max_tokens),
            system: Some(settings.system.clone()),
        }
    }

    pub fn apply(&self, settings: &Sampling) -> Sampling {
        Sampling {
            temperature: self.temperature.unwrap_or(settings.temperature),
            top_p: self.top_p.unwrap_or(settings.top_p),
            max_tokens: self.max_tokens.unwrap_or(settings.max_tokens),
            system: self.system.clone().unwrap_or_else(|| settings.system.clone()),
        }
    }
}

/// One model kept on the disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Its name and rounding, lowercased: `tide-7b-instruct-q4_k_m`.
    pub id: String,
    /// Its name, unique in the library with its rounding: `tide-7b-instruct`.
    pub name: String,
    /// Who published it.
    pub publisher: String,
    pub family: String,
    /// Billions of parameters.
    pub params: f64,
    /// How the weights are rounded: `Q4_K_M`, `8bit`, `BF16`.
    pub quant: String,
    pub format: Format,
    /// Bytes on disk, every file together.
    pub size: f64,
    /// Tokens of context it was trained for.
    pub context: f64,
    pub source: Source,
    /// When it was put in the library, in milliseconds since the epoch.
    pub added: i64,
    /// When it last answered: a seed's may fall between milliseconds.
    pub used: Option<f64>,
    pub capabilities: Vec<Capability>,
    pub tags: Vec<String>,
    pub files: Vec<ModelFile>,
    /// What new conversations with it start with.
    pub defaults: Sampling,
    /// Its card, as markdown.
    pub card: String,
}

impl Model {
    pub fn can(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// What an answer cost and how fast it came.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input: f64,
    pub output: f64,
    /// Milliseconds to the first token.
    pub first_token: f64,
    /// Tokens a second from the first token.
    pub rate: f64,
    /// Milliseconds the whole answer took.
    pub latency: f64,
    /// Why it ended early, if it did: `max tokens`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
}

/// What an attachment is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentKind {
    Image,
    File,
}

/// A file sent with a message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub kind: AttachmentKind,
    /// Bytes.
    pub size: f64,
    /// A line of what it holds: `CSV, 2,184 lines`.
    pub detail: String,
    /// The text of a text file, read by the model.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
    /// An image small enough to keep, as a data address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplyState {
    Streaming,
    Done,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feedback {
    Good,
    Bad,
}

/// One version of an answer: the first, or one asked for again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub text: String,
    /// The model's thinking before it answered, if it thinks.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<String>,
    pub model: String,
    pub state: ReplyState,
    pub usage: Option<Usage>,
    pub time: i64,
    /// What went wrong, when it failed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub problem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub feedback: Option<Feedback>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: String,
    pub text: String,
    pub time: i64,
    pub attachments: Vec<Attachment>,
    /// The emoji given to the question, each with its name and count, the person's own marked.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reactions: Option<Vec<Reaction>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: String,
    pub versions: Vec<Version>,
    /// The version shown, from 0.
    pub at: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
}

impl Message {
    pub fn id(&self) -> &str {
        match self {
            Message::User(one) => &one.id,
            Message::Assistant(one) => &one.id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    /// The model it talks to, by its id in the library.
    pub model: String,
    /// When it was last active.
    pub time: i64,
    pub pinned: bool,
    pub messages: Vec<Message>,
    pub settings: Sampling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState {
    Queued,
    Running,
    Paused,
    Done,
    Failed,
    Cancelled,
}

impl DownloadState {
    pub fn key(self) -> &'static str {
        match self {
            DownloadState::Queued => "queued",
            DownloadState::Running => "running",
            DownloadState::Paused => "paused",
            DownloadState::Done => "done",
            DownloadState::Failed => "failed",
            DownloadState::Cancelled => "cancelled",
        }
    }

    /// Under way, waiting or paused.
    pub fn going(self) -> bool {
        matches!(
            self,
            DownloadState::Running | DownloadState::Queued | DownloadState::Paused
        )
    }

    /// Landed, cancelled or failed.
    pub fn ended(self) -> bool {
        !self.going()
    }
}

/// A model being pulled from a source onto the disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    /// The catalog entry and the variant pulled.
    pub entry: String,
    pub quant: String,
    pub name: String,
    pub publisher: String,
    pub source: Source,
    pub format: Format,
    /// Bytes in all and bytes in so far.
    pub size: f64,
    pub done: f64,
    /// Bytes a second, as last measured.
    pub speed: f64,
    pub state: DownloadState,
    pub started: i64,
    pub ended: Option<i64>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerStatus {
    Up,
    Down,
    Starting,
    Stopping,
}

impl RunnerStatus {
    pub fn key(self) -> &'static str {
        match self {
            RunnerStatus::Up => "up",
            RunnerStatus::Down => "down",
            RunnerStatus::Starting => "starting",
            RunnerStatus::Stopping => "stopping",
        }
    }
}

/// The local runner that loads models and answers with them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Runner {
    pub id: String,
    pub status: RunnerStatus,
    /// When it came up or went down.
    pub since: i64,
    pub version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadState {
    Loading,
    Ready,
}

/// A model the runner holds in memory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    /// The model's id in the library.
    pub id: String,
    /// The model's name.
    pub model: String,
    pub state: LoadState,
    /// How far the load is, from 0 to 1.
    pub progress: f64,
    /// Bytes of memory it takes.
    pub memory: f64,
    /// Tokens of context it was loaded with.
    pub context: f64,
    pub loaded_at: i64,
    pub used_at: i64,
    /// Answers it has written since it was loaded.
    pub answers: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    Runner,
    Loader,
    Chat,
    Pull,
    Library,
}

impl LogSource {
    pub fn key(self) -> &'static str {
        match self {
            LogSource::Runner => "runner",
            LogSource::Loader => "loader",
            LogSource::Chat => "chat",
            LogSource::Pull => "pull",
            LogSource::Library => "library",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogEvent {
    pub id: String,
    pub time: i64,
    pub level: Level,
    pub source: LogSource,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnState {
    Waiting,
    Streaming,
    Done,
    Stopped,
    Failed,
}

/// One model's answer in a comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompareColumn {
    pub model: String,
    pub state: ColumnState,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<String>,
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub problem: Option<String>,
}

/// One prompt sent to several models side by side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub id: String,
    pub prompt: String,
    pub time: i64,
    pub temperature: f64,
    pub max_tokens: f64,
    pub columns: Vec<CompareColumn>,
}

/// The person's preferences, kept by the backend as one row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// Where the library keeps its models.
    pub root: String,
    /// The model a new conversation starts with, by its id in the library.
    pub default_model: String,
    /// Megabytes a second all downloads may take together; 0 for no limit.
    pub bandwidth: f64,
    /// How many downloads run at once; the rest wait their turn.
    pub parallel: f64,
    /// Minutes a model stays loaded after its last answer; 0 keeps it until unloaded.
    pub keep_alive: f64,
    /// Gigabytes of memory the runner may fill with models.
    pub memory: f64,
    /// Start the runner when the app opens.
    pub start_runner: bool,
    /// Load a model when a chat needs it and it is not loaded.
    pub auto_load: bool,
    /// Ask before deleting a model from the disk.
    pub confirm_delete: bool,
}

pub fn default_prefs() -> Prefs {
    Prefs {
        id: "prefs".to_owned(),
        root: "/Volumes/models".to_owned(),
        default_model: "tide-7b-instruct-q4_k_m".to_owned(),
        bandwidth: 0.0,
        parallel: 2.0,
        keep_alive: 10.0,
        memory: 24.0,
        start_runner: true,
        auto_load: true,
        confirm_delete: true,
    }
}

/// A change to some of the preferences, as `Partial<Omit<Prefs, 'id'>>`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_runner: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_load: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_delete: Option<bool>,
}

impl PrefsPatch {
    pub fn apply(&self, prefs: &mut Prefs) {
        if let Some(value) = &self.root {
            prefs.root = value.clone();
        }
        if let Some(value) = &self.default_model {
            prefs.default_model = value.clone();
        }
        if let Some(value) = self.bandwidth {
            prefs.bandwidth = value;
        }
        if let Some(value) = self.parallel {
            prefs.parallel = value;
        }
        if let Some(value) = self.keep_alive {
            prefs.keep_alive = value;
        }
        if let Some(value) = self.memory {
            prefs.memory = value;
        }
        if let Some(value) = self.start_runner {
            prefs.start_runner = value;
        }
        if let Some(value) = self.auto_load {
            prefs.auto_load = value;
        }
        if let Some(value) = self.confirm_delete {
            prefs.confirm_delete = value;
        }
    }
}
