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
