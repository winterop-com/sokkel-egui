//! The simulated model, as the web's `engine/generate.ts` has it: what it answers to a prompt, how
//! many tokens that is, and how fast a given model writes it. Answers are drawn from a few written
//! ones per kind of question, picked by the prompt, the model, the temperature and the attempt, so
//! temperature 0 always answers alike and asking again at a higher one gives another version; top p
//! narrows the draw to the likeliest of them, the first ones written. A system prompt asking to be
//! brief keeps the first paragraph, and max tokens cuts the answer where it runs out.

use serde::Serialize;

use crate::data::model::{Attachment, AttachmentKind, Capability, Model, Sampling};
use crate::js::{has_word, is_space, len16, locale, number_to_string, round, slice16, split_spaces, trim, words};

/// What a model is asked.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub prompt: String,
    pub model: Model,
    pub settings: Sampling,
    pub attachments: Vec<Attachment>,
    /// Tokens of the conversation before this prompt.
    pub history: f64,
    /// 0 for the first answer, then one more each time it is asked again.
    pub attempt: i64,
}

/// An answer planned out: its text, the model's thinking before it, and how fast it comes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Plan {
    pub reasoning: String,
    pub text: String,
    /// Tokens the prompt takes, the system prompt and the history with it.
    pub input: f64,
    /// Milliseconds to the first token once the model is loaded.
    pub prefill: f64,
    /// Tokens a second.
    pub rate: f64,
    /// Why the answer ends early: `max tokens`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A rough token count: about four characters a token, at least one a word.
pub fn tokens(text: &str) -> f64 {
    split_spaces(text)
        .into_iter()
        .filter(|piece| !piece.is_empty())
        .map(|piece| (len16(piece) as f64 / 4.2).ceil().max(1.0))
        .sum()
}

/// The text cut into the pieces it streams in: each word with the space after it.
pub fn pieces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_space = false;
    for c in text.chars() {
        let space = is_space(c);
        if !space && in_space && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        in_space = space;
        current.push(c);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// FNV-1a over a text's UTF-16 units, as the web hashes it.
pub fn hash(text: &str) -> u32 {
    let mut value: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        value ^= u32::from(unit);
        value = value.wrapping_mul(16_777_619);
    }
    value
}

/// How much faster or slower each rounding writes than four bits.
fn quant_pace(quant: &str) -> f64 {
    match quant {
        "Q3_K_S" => 1.1,
        "Q4_K_M" => 1.0,
        "Q5_K_M" => 0.9,
        "Q6_K" => 0.82,
        "Q8_0" => 0.7,
        "F16" | "BF16" => 0.45,
        "4bit" => 1.12,
        "8bit" => 0.76,
        _ => 1.0,
    }
}

/// Tokens a second a model writes on this machine.
pub fn rate_of(model: &Model) -> f64 {
    ((330.0 / model.params.powf(0.9)) * quant_pace(&model.quant)).min(320.0)
}

/// Milliseconds a model takes to read a prompt of `input` tokens.
pub fn prefill_of(model: &Model, input: f64) -> f64 {
    round(60.0 + input * model.params * 0.045)
}

/// The kinds of question the simulated model has answers written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Greeting,
    Cron,
    Translate,
    Summary,
    Python,
    Jq,
    Typescript,
    Speed,
    Failure,
    Ideas,
    Names,
    General,
}

fn any_word(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| has_word(text, word))
}

fn any_part(text: &str, parts: &[&str]) -> bool {
    parts.iter().any(|part| text.contains(part))
}

pub fn kind_of(prompt: &str, model: &Model) -> Kind {
    let text = prompt.to_lowercase();
    let greeting = ["hi", "hello", "hey", "good morning", "good evening"]
        .iter()
        .any(|start| text.starts_with(start) && !text[start.len()..].chars().next().is_some_and(crate::js::is_word));
    if greeting {
        return Kind::Greeting;
    }
    if has_word(&text, "cron")
        || text.contains("schedule")
        || ["day", "weekday", "week", "monday", "hour", "night"]
            .iter()
            .any(|one| text.contains(&format!("every {one}")))
    {
        return Kind::Cron;
    }
    if any_part(
        &text,
        &["translat", "in norwegian", "in french", "in german", "in spanish"],
    ) {
        return Kind::Translate;
    }
    if any_part(&text, &["summar", "tl;dr", "tldr", "release notes", "digest", "recap"]) {
        return Kind::Summary;
    }
    if has_word(&text, "jq") || any_part(&text, &["filter the json", "json filter"]) {
        return Kind::Jq;
    }
    if any_part(&text, &["typescript", "retry", "backoff", "javascript"]) || has_word(&text, "ts") {
        return Kind::Typescript;
    }
    if any_word(
        &text,
        &[
            "code", "function", "python", "script", "regex", "sql", "query", "parse", "parser", "bug", "fix", "test",
        ],
    ) {
        return Kind::Python;
    }
    if any_word(&text, &["name", "names"]) {
        return Kind::Names;
    }
    if any_word(&text, &["ideas", "list", "suggest", "options", "ways"]) {
        return Kind::Ideas;
    }
    if any_word(
        &text,
        &[
            "model", "models", "token", "tokens", "slow", "slower", "fast", "faster", "bigger", "rate", "quantiz",
            "rounding", "memory",
        ],
    ) {
        return Kind::Speed;
    }
    if any_word(
        &text,
        &[
            "fail", "failed", "failure", "error", "broke", "broken", "crash", "why did",
        ],
    ) {
        return Kind::Failure;
    }
    if model.can(Capability::Code) {
        Kind::Python
    } else {
        Kind::General
    }
}

const PYTHON: &str = "```python\nfrom decimal import Decimal, InvalidOperation\n\n\ndef parse_amount(text: str) -> Decimal | None:\n    \"\"\"An amount from a cell, with a decimal comma or point; None for a blank or junk.\"\"\"\n    cleaned = text.strip().replace(\" \", \"\")\n    if cleaned in {\"\", \"n/a\", \"-\"}:\n        return None\n    if \",\" in cleaned and \".\" not in cleaned:\n        cleaned = cleaned.replace(\",\", \".\")\n    try:\n        return Decimal(cleaned)\n    except InvalidOperation:\n        return None\n```";
const GREETING_2: &str =
    "Hi. What should we look at? I can read files you attach, write code, or summarize a long text.";
const CRON_1: &str = "To run `send-digest` every weekday at 06:30:\n\n```cron\n30 6 * * 1-5\n```\n\n- `30 6` is the minute and the hour.\n- `* *` takes every day of every month.\n- `1-5` keeps Monday to Friday.\n\nThe scheduler reads it in UTC unless the job sets a time zone, so in Oslo in winter it fires at 07:30 local time.";
const CRON_2: &str = "Every weekday at half past six is `30 6 * * 1-5`.\n\nIf it should skip public holidays as well, cron alone cannot say so: leave the schedule as it is and let the job's first step, `fetch`, end early on a day listed as a holiday.";
const TRANSLATE_1: &str = "In Norwegian:\n\n> Den nattlige eksporten feilet i `validate` fordi 37 rader hadde beløp med desimalkomma. Vi leser nå beløpene med riktig locale, så kjøringen i natt skal gå gjennom.\n\n\"Locale\" is usually left as it is in Norwegian technical writing.";
const TRANSLATE_2: &str = "Here it is, kept close to the original:\n\n> Eksporten stoppet ved valideringen: 37 av 2 184 rader hadde et beløp som ikke var et tall.";
const SUMMARY_1: &str = "**The week in short**\n\n- `nightly-export` failed twice, both times at `validate`, on amounts written with a decimal comma.\n- `sync-inventory` timed out once while the worker restarted; the retry an hour later succeeded.\n- `rebuild-index` is 18% slower since the index grew past two million rows.\n\nNothing is missing downstream. The one open item is the parse fix for `nightly-export`, which ships tonight.";
const SUMMARY_2: &str = "It comes down to three things:\n\n1. Two failed runs of `nightly-export`, one cause, now fixed.\n2. One timeout in `sync-inventory`, already retried.\n3. A slower `rebuild-index`, which needs a look before it doubles.\n\nAsk grace about the index; she wrote the step.";
const JQ_1: &str = "A jq filter that keeps the failed runs and says which step each stopped at:\n\n```jq\n.runs[]\n| select(.state == \"failed\")\n| {job, step: .at.step, started}\n```\n\nRun it over the API's answer:\n\n```bash\ncurl -s https://tidevann.example.org/api/runs | jq -f failed.jq\n```";
const JQ_2: &str = "This one counts the failures per job instead of listing them:\n\n```jq\n[.runs[] | select(.state == \"failed\")]\n| group_by(.job)\n| map({job: .[0].job, failed: length})\n```";
const TYPESCRIPT_1: &str = "A small TypeScript helper that retries a call with a growing pause:\n\n```ts\nexport async function retry<T>(call: () => Promise<T>, tries = 4, pause = 500): Promise<T> {\n    for (let attempt = 1; ; attempt += 1) {\n        try {\n            return await call()\n        } catch (error) {\n            if (attempt >= tries) throw error\n            await new Promise((done) => setTimeout(done, pause * 2 ** (attempt - 1)))\n        }\n    }\n}\n```\n\nFour tries wait 0.5, 1 and 2 seconds between them, so a call that keeps failing gives up after 3.5 seconds.";
const TYPESCRIPT_2: &str = "Retry with backoff, and give up on errors that will not get better:\n\n```ts\nconst permanent = (error: unknown) => error instanceof Response && error.status < 500\n\nexport async function retry<T>(call: () => Promise<T>, tries = 4): Promise<T> {\n    let wait = 500\n    for (let attempt = 1; attempt < tries; attempt += 1) {\n        try {\n            return await call()\n        } catch (error) {\n            if (permanent(error)) throw error\n            await new Promise((done) => setTimeout(done, wait))\n            wait *= 2\n        }\n    }\n    return call()\n}\n```";
const SPEED_1: &str = "Think of it in two parts.\n\n**What a model reads.** Every token of the prompt, the system prompt and the conversation so far is read before the first word comes out. That is the *time to first token*, and it grows with the length of what is sent and the size of the model.\n\n**What a model writes.** After that it writes one token at a time, and every token passes through all of its weights. A model twice the size moves twice the bytes per token, so it writes about half as fast; rounding the weights to four bits instead of eight halves the bytes again.";
const SPEED_2: &str = "Mostly memory bandwidth. Writing a token means reading every weight once:\n\n| model | weights at 4 bits | tokens a second, roughly |\n| --- | --- | --- |\n| 3B | 1.9 GB | 110 |\n| 7B | 4.4 GB | 55 |\n| 14B | 8.6 GB | 30 |\n| 32B | 19 GB | 14 |\n\nThe usage line under each answer shows the rate this machine actually got.";
const FAILURE_1: &str = "The short answer is that the input changed and the check did not.\n\nThe export reads amounts from a supplier feed. Since October 14 that feed writes them with a decimal comma, `12,50` rather than `12.50`, and `validate` refuses anything that does not parse as a number. Nothing in our code changed; the data did.\n\nThe fix belongs in `transform`, which should parse with the source's locale, not in `validate`, which is right to refuse junk.";
const FAILURE_2: &str = "`nightly-export` stopped at `validate`: **37 of 2,184 rows** had an `amount` that was not a number, mostly decimal commas like `12,50`.\n\n- `transform` now parses amounts with the source's locale.\n- Blank amounts are dropped before `validate`.\n\nThe next run is tonight at 02:00. Questions go to ada.";
const IDEAS_1: &str = "A few ideas:\n\n1. **Name the step in the alert.** A failure that says `validate` gets read faster than one that says \"export failed\".\n2. **Keep the rejected rows.** Write them to `rejected.csv` beside the export, so nobody has to rerun to see them.\n3. **Warn before failing.** Let up to ten bad rows through with a warning, and fail only past that.\n4. **Chart the run times.** A slow creep in `rebuild-index` shows up weeks before it times out.";
const IDEAS_2: &str = "Three that cost little:\n\n- Send the failing step's last ten log lines with every alert.\n- Add a dry run that reads and validates without writing.\n- Give each job an owner, named in its definition, so an alert reaches a person.";
const NAMES_1: &str = "Some names, in the house style of short sea words:\n\n- **eddy** for the retry loop, since it goes round\n- **skerry** for the small checks before a run\n- **sounding** for the health probe\n- **ballast** for the queue that evens out the load";
const NAMES_2: &str = "A few that read well in a log line: **undertow**, **slack-water**, **wrack** and **spindrift**. Short ones are easier to type in a command; I would pick **wrack** for the cleanup job.";
const GREETING_1: [&str; 2] = [
    "Hello. I am **",
    "**, running on this machine with nothing sent anywhere else. Ask me about a job, a log, some code or a text you want written.",
];
const PYTHON_1: [&str; 2] = [
    "Here is a parser for amounts written either way, `12.50` or `12,50`:\n\n",
    "\n\nIt keeps `Decimal` rather than `float`, so `12,50` stays exactly 12.50. A test for the cases that failed last night:\n\n```python\nassert parse_amount(\"12,50\") == Decimal(\"12.50\")\nassert parse_amount(\"n/a\") is None\n```",
];
const PYTHON_2: [&str; 2] = [
    "The trick is to normalize the decimal mark before converting, and to treat blanks as missing rather than zero:\n\n",
    "\n\nCall it on every `amount` cell in `transform`; `validate` then only sees numbers or `None`.",
];
const GENERAL_1: [&str; 2] = [
    "> ",
    "\n\nHere is how I would go about it. Start with what you can check: the latest run, its log, and what changed since it last worked: the code, the data or the machine. Most problems that appear overnight come from the data.\n\nIf you attach the log or the file, I can point at the line.",
];
const GENERAL_2: [&str; 2] = [
    "On \"",
    "\": the plain answer is that it depends on how often it changes.\n\n- If it changes daily, automate it and keep a log.\n- If it changes monthly, a checklist is enough.\n- If it rarely changes, write down why it is the way it is, so the next person does not undo it.",
];
const REASONING: [&str; 2] = [
    "The question: \"",
    "\". Before answering, work out what is actually asked, what I know for certain, and what I would only be guessing.\n\nKeep it to what can be checked. Name the parts in code, give one example, and say plainly where I am not sure.",
];

/// The first line of a prompt, cut to `count` UTF-16 units.
fn first_line(prompt: &str, count: usize) -> String {
    slice16(prompt.split('\n').next().unwrap_or(""), count)
}

/// How many answers are written for a kind of question.
fn written(kind: Kind) -> usize {
    match kind {
        Kind::Greeting
        | Kind::Cron
        | Kind::Translate
        | Kind::Summary
        | Kind::Python
        | Kind::Jq
        | Kind::Typescript
        | Kind::Speed
        | Kind::Failure
        | Kind::Ideas
        | Kind::Names
        | Kind::General => 2,
    }
}

/// The written answer `pick` of a kind, for a prompt and a model.
fn answer_of(kind: Kind, pick: usize, prompt: &str, model: &Model) -> String {
    let joined = |parts: [&str; 2], middle: &str| format!("{}{middle}{}", parts[0], parts[1]);
    match (kind, pick) {
        (Kind::Greeting, 0) => joined(GREETING_1, &model.name),
        (Kind::Greeting, _) => GREETING_2.to_owned(),
        (Kind::Cron, 0) => CRON_1.to_owned(),
        (Kind::Cron, _) => CRON_2.to_owned(),
        (Kind::Translate, 0) => TRANSLATE_1.to_owned(),
        (Kind::Translate, _) => TRANSLATE_2.to_owned(),
        (Kind::Summary, 0) => SUMMARY_1.to_owned(),
        (Kind::Summary, _) => SUMMARY_2.to_owned(),
        (Kind::Python, 0) => joined(PYTHON_1, PYTHON),
        (Kind::Python, _) => joined(PYTHON_2, PYTHON),
        (Kind::Jq, 0) => JQ_1.to_owned(),
        (Kind::Jq, _) => JQ_2.to_owned(),
        (Kind::Typescript, 0) => TYPESCRIPT_1.to_owned(),
        (Kind::Typescript, _) => TYPESCRIPT_2.to_owned(),
        (Kind::Speed, 0) => SPEED_1.to_owned(),
        (Kind::Speed, _) => SPEED_2.to_owned(),
        (Kind::Failure, 0) => FAILURE_1.to_owned(),
        (Kind::Failure, _) => FAILURE_2.to_owned(),
        (Kind::Ideas, 0) => IDEAS_1.to_owned(),
        (Kind::Ideas, _) => IDEAS_2.to_owned(),
        (Kind::Names, 0) => NAMES_1.to_owned(),
        (Kind::Names, _) => NAMES_2.to_owned(),
        (Kind::General, 0) => joined(GENERAL_1, &first_line(prompt, 160)),
        (Kind::General, _) => joined(GENERAL_2, &first_line(prompt, 80)),
    }
}

/// The thinking a model that reasons writes before it answers.
fn reasoning_of(prompt: &str) -> String {
    format!("{}{}{}", REASONING[0], first_line(prompt, 90), REASONING[1])
}

/// The lines of a text that are not blank.
fn lines_of(text: &str) -> Vec<&str> {
    text.split('\n').filter(|line| !trim(line).is_empty()).collect()
}

/// What a set of attachments says before the answer.
fn read_attachments(attachments: &[Attachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = attachments
        .iter()
        .map(|file| {
            if file.kind == AttachmentKind::Image {
                return format!(
                    "- `{}`: an image of {}. I can describe what is in it if this model reads images.",
                    file.name, file.detail
                );
            }
            let Some(text) = &file.text else {
                return format!("- `{}`: {}. I could not read it as text.", file.name, file.detail);
            };
            let rows = lines_of(text);
            let first = rows.first().map(|row| slice16(row, 80)).unwrap_or_default();
            let starting = if first.is_empty() {
                String::new()
            } else {
                format!(", starting `{}`", first.replace('`', "'"))
            };
            format!("- `{}`: {} lines{starting}.", file.name, locale(rows.len() as f64))
        })
        .collect();
    format!("I read what you attached:\n\n{}\n\n", lines.join("\n"))
}

/// Cells of one CSV line, quoted cells kept whole.
pub fn cells(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut found: Vec<String> = Vec::new();
    let mut at = 0;
    loop {
        // A quoted cell, its quotes doubled inside, then a comma or the end; else anything up to a comma.
        let mut end = None;
        if chars.get(at) == Some(&'"') {
            let mut pos = at + 1;
            loop {
                match chars.get(pos) {
                    Some('"') if chars.get(pos + 1) == Some(&'"') => pos += 2,
                    Some('"') | None => break,
                    Some(_) => pos += 1,
                }
            }
            if chars.get(pos) == Some(&'"') {
                let after = pos + 1;
                match chars.get(after) {
                    Some(',') => end = Some(after + 1),
                    None => end = Some(after),
                    _ => {}
                }
            }
        }
        let end = end.unwrap_or_else(|| {
            let mut pos = at;
            while chars.get(pos).is_some_and(|c| *c != ',') {
                pos += 1;
            }
            if chars.get(pos) == Some(&',') { pos + 1 } else { pos }
        });
        found.push(chars[at..end].iter().collect());
        if end == at {
            break;
        }
        at = end;
        if at > chars.len() {
            break;
        }
    }
    let mapped: Vec<String> = found
        .into_iter()
        .map(|cell| {
            let cell = cell.strip_suffix(',').unwrap_or(&cell).to_owned();
            let cell = cell.strip_prefix('"').unwrap_or(&cell).to_owned();
            let cell = cell.strip_suffix('"').unwrap_or(&cell).to_owned();
            trim(&cell).to_owned()
        })
        .collect();
    let count = mapped.len();
    mapped
        .into_iter()
        .enumerate()
        .filter(|(index, cell)| *index + 1 < count || !cell.is_empty())
        .map(|(_, cell)| cell)
        .collect()
}

fn is_blank_cell(cell: &str) -> bool {
    cell.is_empty() || ["n/a", "-", "null"].contains(&cell.to_lowercase().as_str())
}

fn is_decimal_comma(cell: &str) -> bool {
    match cell.split_once(',') {
        Some((whole, fraction)) => {
            !whole.is_empty()
                && !fraction.is_empty()
                && whole.chars().all(|c| c.is_ascii_digit())
                && fraction.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

fn ends_with_ignoring_case(name: &str, end: &str) -> bool {
    name.to_lowercase().ends_with(end)
}

/// What one attached file holds, read as a person would describe it.
fn describe(file: &Attachment) -> String {
    if file.kind == AttachmentKind::Image {
        return format!(
            "**{}** is an image ({}). This model reads text, so I can only say what its name and size suggest.",
            file.name, file.detail
        );
    }
    let Some(text) = &file.text else {
        return format!(
            "**{}** ({}) is not text, so I could not read it.",
            file.name, file.detail
        );
    };
    let lines = lines_of(text);
    if ends_with_ignoring_case(&file.name, ".csv") && lines.len() > 1 {
        let head = cells(lines[0]);
        let rows: Vec<Vec<String>> = lines[1..].iter().map(|line| cells(line)).collect();
        let blank = rows
            .iter()
            .filter(|row| row.iter().any(|cell| is_blank_cell(cell)))
            .count();
        let commas = rows
            .iter()
            .filter(|row| row.iter().any(|cell| is_decimal_comma(cell)))
            .count();
        let mut table = vec![
            format!("| {} |", head.join(" | ")),
            format!("| {} |", head.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")),
        ];
        for row in rows.iter().take(3) {
            let said: Vec<String> = row.iter().map(|cell| format!("`{cell}`")).collect();
            table.push(format!("| {} |", said.join(" | ")));
        }
        let mut notes = Vec::new();
        if blank > 0 {
            notes.push(format!(
                "{blank} {} a blank or `n/a` cell",
                if blank == 1 { "row has" } else { "rows have" }
            ));
        }
        if commas > 0 {
            notes.push(format!(
                "{commas} {} a number with a decimal comma, which a strict parser refuses",
                if commas == 1 { "writes" } else { "write" }
            ));
        }
        let names: Vec<String> = head.iter().map(|name| format!("`{name}`")).collect();
        return format!(
            "**{}** is a CSV of {} rows and {} columns: {}. The first rows:\n\n{}\n{}",
            file.name,
            locale(rows.len() as f64),
            head.len(),
            names.join(", "),
            table.join("\n"),
            if notes.is_empty() {
                String::new()
            } else {
                format!("\nWorth knowing: {}.", notes.join(", and "))
            }
        );
    }
    if ends_with_ignoring_case(&file.name, ".json") {
        return match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => {
                let keys = crate::js::object_keys(&value);
                let listed = if keys.is_empty() {
                    String::new()
                } else {
                    let named: Vec<String> = keys.iter().take(8).map(|key| format!("`{key}`")).collect();
                    format!(" with the keys {}", named.join(", "))
                };
                format!(
                    "**{}** is JSON{listed}, {} lines long.",
                    file.name,
                    locale(lines.len() as f64)
                )
            }
            Err(_) => format!(
                "**{}** looks like JSON but does not parse; the first line is `{}`.",
                file.name,
                lines.first().map(|line| slice16(line, 60)).unwrap_or_default()
            ),
        };
    }
    format!(
        "**{}** is {} lines and {} words of text, starting:\n\n> {}",
        file.name,
        locale(lines.len() as f64),
        locale(words(text).len() as f64),
        lines.first().map(|line| slice16(line, 160)).unwrap_or_default()
    )
}

/// Whether a prompt asks about the files sent with it.
fn about_files(ask: &Ask) -> bool {
    let prompt = ask.prompt.to_lowercase();
    !ask.attachments.is_empty()
        && (trim(&ask.prompt).is_empty()
            || any_word(
                &prompt,
                &[
                    "file",
                    "files",
                    "this",
                    "these",
                    "attached",
                    "attachment",
                    "csv",
                    "log",
                    "what is in",
                    "what's in",
                    "read",
                    "look at",
                ],
            ))
}

/// Plans the answer `ask` gets: which text, its thinking, its tokens and its pace.
pub fn plan(ask: &Ask) -> Plan {
    let kind = kind_of(&ask.prompt, &ask.model);
    let choices = written(kind);
    // Top p keeps the likeliest share of the written answers to draw from: the first ones, at least one.
    let kept = ((choices as f64 * ask.settings.top_p)
        .ceil()
        .min(choices as f64)
        .max(1.0)) as u32;
    let pick = if ask.settings.temperature == 0.0 {
        0
    } else {
        let key = format!(
            "{}|{}|{}|{}",
            ask.prompt,
            ask.model.id,
            ask.attempt,
            number_to_string(round(ask.settings.temperature * 10.0))
        );
        (hash(&key) % kept) as usize
    };
    let mut text = if about_files(ask) {
        ask.attachments.iter().map(describe).collect::<Vec<_>>().join("\n\n")
    } else {
        read_attachments(&ask.attachments) + &answer_of(kind, pick, &ask.prompt, &ask.model)
    };
    let system = ask.settings.system.to_lowercase();
    if any_word(&system, &["brief", "short", "concise", "one sentence", "terse"]) {
        text = text.split("\n\n").next().unwrap_or("").to_owned();
    }
    if ask.model.params < 1.0 {
        text = text.split("\n\n").take(2).collect::<Vec<_>>().join("\n\n");
    }
    let mut note = None;
    let all = pieces(&text);
    let mut used = 0.0;
    let mut cut = all.len();
    for (index, piece) in all.iter().enumerate() {
        used += tokens(piece);
        if used > ask.settings.max_tokens {
            cut = index;
            note = Some("max tokens".to_owned());
            break;
        }
    }
    if note.is_some() {
        text = all[..cut].concat();
    }
    let reasoning = if ask.model.can(Capability::Reasoning) && kind != Kind::Greeting {
        reasoning_of(&ask.prompt)
    } else {
        String::new()
    };
    let input = tokens(&ask.settings.system)
        + ask.history
        + tokens(&ask.prompt)
        + ask
            .attachments
            .iter()
            .fold(0.0, |sum, file| sum + tokens(file.text.as_deref().unwrap_or("")) + 8.0)
        + 6.0;
    let jitter = 0.92 + f64::from(hash(&format!("{}|{}", ask.prompt, ask.attempt)) % 160) / 1000.0;
    Plan {
        reasoning,
        text,
        input,
        prefill: prefill_of(&ask.model, input),
        rate: rate_of(&ask.model) * jitter,
        note,
    }
}

/// A title for a conversation from its first prompt: its first few words.
pub fn title_of(prompt: &str) -> String {
    let line: String = trim(prompt)
        .split('\n')
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !matches!(c, '`' | '*' | '_' | '#' | '>'))
        .collect();
    if line.is_empty() {
        return "Files".to_owned();
    }
    let cut = split_spaces(&line).into_iter().take(7).collect::<Vec<_>>().join(" ");
    if len16(&cut) < len16(&line) {
        let cut = match cut.chars().last() {
            Some('.' | ',' | ';' | ':' | '?' | '!') => &cut[..cut.len() - 1],
            _ => &cut[..],
        };
        format!("{cut}\u{2026}")
    } else {
        match cut.chars().last() {
            Some('.' | '?' | '!') => cut[..cut.len() - 1].to_owned(),
            _ => cut,
        }
    }
}
