//! The sources the library pulls from, simulated, as the web's `data/catalog.ts` has them: a
//! public hub's catalog and the stores of a local and a desktop runner, each entry with the
//! variants it can be pulled in. A search answers after a short delay, as a remote catalog would
//! (`search_delay`).

use std::cmp::Ordering;
use std::sync::OnceLock;

use serde::Serialize;

use super::model::{Capability, FileRole, Format, ModelFile, Source};
use crate::js::{locale_compare, round, words};

/// One way an entry can be pulled: its rounding and format, and the bytes that take.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Variant {
    pub quant: String,
    pub format: Format,
    pub size: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CatalogEntry {
    /// `publisher/name`.
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub family: String,
    pub params: f64,
    pub context: f64,
    pub source: Source,
    pub capabilities: Vec<Capability>,
    pub tags: Vec<String>,
    pub summary: String,
    /// Pulls in the last month.
    pub downloads: f64,
    pub likes: f64,
    /// Days since it last changed.
    pub updated: f64,
    pub variants: Vec<Variant>,
}

/// Bits a weight takes in each rounding.
fn bits(quant: &str) -> f64 {
    match quant {
        "Q3_K_S" => 3.5,
        "Q4_K_M" => 4.85,
        "Q5_K_M" => 5.7,
        "Q6_K" => 6.6,
        "Q8_0" => 8.5,
        "F16" | "BF16" => 16.0,
        "4bit" => 4.5,
        "8bit" => 8.5,
        _ => 8.0,
    }
}

/// What each rounding means, said in a tooltip and on a model's page, in the order the web lists
/// them.
pub const QUANTS: [(&str, &str); 9] = [
    (
        "Q3_K_S",
        "Three bits a weight, small blocks: the smallest that still answers well",
    ),
    (
        "Q4_K_M",
        "Four bits a weight, medium blocks: the usual balance of size and quality",
    ),
    (
        "Q5_K_M",
        "Five bits a weight: a little larger, a little closer to the original",
    ),
    ("Q6_K", "Six bits a weight: hard to tell from the original"),
    (
        "Q8_0",
        "Eight bits a weight: as good as the original, twice the size of four",
    ),
    ("F16", "Half precision: the original weights, unrounded"),
    ("BF16", "Brain float, half precision: the weights as trained"),
    ("4bit", "Four bits a weight, for unified memory"),
    ("8bit", "Eight bits a weight, for unified memory"),
];

/// What a rounding means, if the app says.
pub fn quant_note(quant: &str) -> Option<&'static str> {
    QUANTS.iter().find(|(one, _)| *one == quant).map(|(_, said)| *said)
}

fn size_of(params: f64, quant: &str) -> f64 {
    round((params * 1e9 * bits(quant)) / 8.0 + 24_000_000.0)
}

fn variants(params: f64, formats: &[(Format, &[&str])]) -> Vec<Variant> {
    formats
        .iter()
        .flat_map(|(format, quants)| {
            quants.iter().map(move |quant| Variant {
                quant: (*quant).to_owned(),
                format: *format,
                size: size_of(params, quant),
            })
        })
        .collect()
}

const PACKED: &[&str] = &["Q4_K_M", "Q5_K_M", "Q8_0"];
const UNIFIED: &[&str] = &["4bit", "8bit"];

#[allow(clippy::too_many_arguments)]
fn entry(
    id: &str,
    name: &str,
    publisher: &str,
    family: &str,
    params: f64,
    context: f64,
    source: Source,
    capabilities: &[Capability],
    tags: &[&str],
    summary: &str,
    downloads: f64,
    likes: f64,
    updated: f64,
    formats: &[(Format, &[&str])],
) -> CatalogEntry {
    CatalogEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        publisher: publisher.to_owned(),
        family: family.to_owned(),
        params,
        context,
        source,
        capabilities: capabilities.to_vec(),
        tags: tags.iter().map(|one| (*one).to_owned()).collect(),
        summary: summary.to_owned(),
        downloads,
        likes,
        updated,
        variants: variants(params, formats),
    }
}

/// Every entry the sources list.
pub fn catalog() -> &'static [CatalogEntry] {
    static CATALOG: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        use Capability::*;
        use Format::*;
        use Source::*;
        vec![
            entry(
                "ada/tide-7b-instruct",
                "tide-7b-instruct",
                "ada",
                "tide",
                7.2,
                32_768.0,
                Hub,
                &[Tools],
                &["chat", "general"],
                "A general model tuned to follow instructions, good at short answers and tool calls.",
                1_284_112.0,
                3_904.0,
                12.0,
                &[(Packed, PACKED), (Unified, UNIFIED)],
            ),
            entry(
                "ada/tide-14b-instruct",
                "tide-14b-instruct",
                "ada",
                "tide",
                14.1,
                65_536.0,
                Hub,
                &[Tools, Reasoning],
                &["chat", "general"],
                "The larger tide: slower, more careful, with a longer context.",
                612_480.0,
                2_210.0,
                12.0,
                &[(Packed, PACKED), (Unified, UNIFIED)],
            ),
            entry(
                "ada/tide-32b-instruct",
                "tide-32b-instruct",
                "ada",
                "tide",
                32.5,
                131_072.0,
                Hub,
                &[Tools, Reasoning],
                &["chat", "long context"],
                "The largest tide, for long documents and hard questions. Needs a machine with room.",
                201_337.0,
                1_502.0,
                30.0,
                &[(Packed, &["Q3_K_S", "Q4_K_M"]), (Unified, &["4bit"])],
            ),
            entry(
                "grace/kelp-3b",
                "kelp-3b",
                "grace",
                "kelp",
                3.1,
                16_384.0,
                Hub,
                &[],
                &["small", "fast"],
                "A small, quick model for drafts, labels and short summaries on any laptop.",
                904_551.0,
                1_877.0,
                4.0,
                &[(Packed, PACKED), (Unified, UNIFIED)],
            ),
            entry(
                "grace/kelp-1b",
                "kelp-1b",
                "grace",
                "kelp",
                1.2,
                8_192.0,
                Hub,
                &[],
                &["small", "fast", "edge"],
                "The smallest kelp, for classifying and routing at hundreds of tokens a second.",
                388_020.0,
                640.0,
                4.0,
                &[(Packed, &["Q4_K_M", "Q8_0", "F16"])],
            ),
            entry(
                "linus/brine-coder-14b",
                "brine-coder-14b",
                "linus",
                "brine",
                14.7,
                65_536.0,
                Hub,
                &[Code, Tools],
                &["code"],
                "Trained on code: writes, explains and fixes functions, queries and scripts.",
                742_906.0,
                4_118.0,
                9.0,
                &[(Packed, PACKED), (Unified, UNIFIED), (Sharded, &["BF16"])],
            ),
            entry(
                "linus/brine-coder-3b",
                "brine-coder-3b",
                "linus",
                "brine",
                3.0,
                32_768.0,
                Hub,
                &[Code],
                &["code", "small", "completion"],
                "A small coder for completing lines in an editor as they are typed.",
                455_390.0,
                1_202.0,
                9.0,
                &[(Packed, PACKED), (Unified, UNIFIED)],
            ),
            entry(
                "margaret/reef-32b",
                "reef-32b",
                "margaret",
                "reef",
                32.0,
                32_768.0,
                Hub,
                &[Reasoning],
                &["reasoning", "math"],
                "Thinks before it answers: slow, thorough, strong at math and planning.",
                330_712.0,
                2_870.0,
                21.0,
                &[(Packed, &["Q3_K_S", "Q4_K_M"]), (Unified, &["4bit"])],
            ),
            entry(
                "margaret/reef-8b",
                "reef-8b",
                "margaret",
                "reef",
                8.0,
                32_768.0,
                Hub,
                &[Reasoning],
                &["reasoning"],
                "The reef way of thinking in a size most machines can hold.",
                520_004.0,
                1_990.0,
                21.0,
                &[(Packed, PACKED), (Unified, UNIFIED)],
            ),
            entry(
                "grace/drift-vision-11b",
                "drift-vision-11b",
                "grace",
                "drift",
                10.6,
                32_768.0,
                Hub,
                &[Vision, Tools],
                &["vision", "documents"],
                "Reads images: screenshots, charts, scanned pages and photos of whiteboards.",
                287_655.0,
                1_334.0,
                15.0,
                &[(Packed, &["Q4_K_M", "Q8_0"]), (Unified, &["4bit"])],
            ),
            entry(
                "linus/shoal-0.5b",
                "shoal-0.5b",
                "linus",
                "shoal",
                0.5,
                4_096.0,
                Hub,
                &[],
                &["tiny", "draft"],
                "A tiny model that drafts tokens for a larger one to check, which makes the larger faster.",
                98_211.0,
                312.0,
                60.0,
                &[(Packed, &["Q8_0", "F16"])],
            ),
            entry(
                "ada/eddy-22b",
                "eddy-22b",
                "ada",
                "eddy",
                22.2,
                131_072.0,
                Hub,
                &[Tools],
                &["chat", "long context", "multilingual"],
                "Writes in many languages and keeps a long context: translation and long summaries.",
                176_408.0,
                980.0,
                45.0,
                &[(Packed, &["Q4_K_M", "Q5_K_M"]), (Unified, &["4bit"])],
            ),
            entry(
                "margaret/skerry-9b",
                "skerry-9b",
                "margaret",
                "skerry",
                9.2,
                8_192.0,
                Hub,
                &[],
                &["chat", "writing"],
                "A careful writer: letters, release notes and plain explanations.",
                233_119.0,
                870.0,
                70.0,
                &[(Packed, PACKED)],
            ),
            entry(
                "grace/swell-70b",
                "swell-70b",
                "grace",
                "swell",
                70.6,
                131_072.0,
                Hub,
                &[Tools, Reasoning],
                &["chat", "large"],
                "Very large and very capable. Needs more memory than most single machines have.",
                140_870.0,
                3_150.0,
                33.0,
                &[(Packed, &["Q3_K_S", "Q4_K_M"])],
            ),
            entry(
                "runner/tide-7b-instruct",
                "tide-7b-instruct",
                "ada",
                "tide",
                7.2,
                32_768.0,
                Runner,
                &[Tools],
                &["chat", "general"],
                "Already in the local runner's store, in its own blob format; taken in as a copy.",
                0.0,
                0.0,
                2.0,
                &[(Packed, &["Q4_K_M"])],
            ),
            entry(
                "runner/kelp-3b",
                "kelp-3b",
                "grace",
                "kelp",
                3.1,
                16_384.0,
                Runner,
                &[],
                &["small"],
                "In the local runner's store since last week.",
                0.0,
                0.0,
                8.0,
                &[(Packed, &["Q4_K_M"])],
            ),
            entry(
                "runner/cove-embed-0.3b",
                "cove-embed-0.3b",
                "linus",
                "cove",
                0.3,
                8_192.0,
                Runner,
                &[],
                &["embedding", "search"],
                "Turns text into vectors for search; it does not chat.",
                0.0,
                0.0,
                40.0,
                &[(Packed, &["F16"])],
            ),
            entry(
                "desktop/brine-coder-14b",
                "brine-coder-14b",
                "linus",
                "brine",
                14.7,
                65_536.0,
                Desktop,
                &[Code, Tools],
                &["code"],
                "In the desktop runner's downloads folder.",
                0.0,
                0.0,
                18.0,
                &[(Unified, &["4bit"])],
            ),
            entry(
                "desktop/drift-vision-11b",
                "drift-vision-11b",
                "grace",
                "drift",
                10.6,
                32_768.0,
                Desktop,
                &[Vision, Tools],
                &["vision"],
                "In the desktop runner's downloads folder, with its vision projector.",
                0.0,
                0.0,
                18.0,
                &[(Packed, &["Q4_K_M"])],
            ),
            entry(
                "desktop/pebble-2b",
                "pebble-2b",
                "margaret",
                "pebble",
                2.4,
                8_192.0,
                Desktop,
                &[],
                &["small"],
                "A small model the desktop runner pulled for its quick replies.",
                0.0,
                0.0,
                90.0,
                &[(Unified, &["4bit", "8bit"])],
            ),
        ]
    })
}

/// The id a library model of this entry and variant gets: its name and rounding, lowercased.
pub fn model_id(name: &str, quant: &str) -> String {
    format!("{name}-{}", quant.to_lowercase())
}

/// Weights split into shards of under five gigabytes, as full weights are published.
fn shards(weights: f64) -> Vec<ModelFile> {
    let count = (weights / 4.9e9).ceil().max(1.0) as usize;
    (0..count)
        .map(|index| ModelFile {
            name: format!("model-{:05}-of-{count:05}.tensors", index + 1),
            size: round(weights / count as f64),
            role: FileRole::Weights,
        })
        .collect()
}

/// The files a variant lands as on disk, the weights first.
pub fn files_of(entry: &CatalogEntry, variant: &Variant) -> Vec<ModelFile> {
    let vision = entry.capabilities.contains(&Capability::Vision);
    let projector = if vision {
        round(entry.params * 62_000_000.0)
    } else {
        0.0
    };
    let weights = variant.size - projector - 1_200_000.0;
    let base = format!("{}-{}", entry.name, variant.quant);
    let file = |name: String, size: f64, role: FileRole| ModelFile { name, size, role };
    let mut files = match variant.format {
        Format::Packed => vec![file(format!("{base}.pack"), weights, FileRole::Weights)],
        Format::Unified if weights > 5e9 => vec![
            file("weights-1-of-2.uw".to_owned(), round(weights / 2.0), FileRole::Weights),
            file(
                "weights-2-of-2.uw".to_owned(),
                weights - round(weights / 2.0),
                FileRole::Weights,
            ),
        ],
        Format::Unified => vec![file("weights.uw".to_owned(), weights, FileRole::Weights)],
        Format::Sharded => shards(weights),
    };
    if vision {
        let name = if variant.format == Format::Packed {
            format!("vision-{}-F16.pack", entry.name)
        } else {
            "vision.uw".to_owned()
        };
        files.push(file(name, projector, FileRole::Projector));
    }
    files.push(file("tokenizer.json".to_owned(), 1_100_000.0, FileRole::Tokenizer));
    files.push(file("config.json".to_owned(), 2_400.0, FileRole::Config));
    files.push(file("README.md".to_owned(), 9_600.0, FileRole::Card));
    files
}

/// What a search asks of the sources: its words, and one source and one format or any.
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogQuery {
    pub text: String,
    pub source: Option<Source>,
    pub format: Option<Format>,
}

/// The entries a query finds, the most pulled first.
pub fn search_catalog(query: &CatalogQuery) -> Vec<&'static CatalogEntry> {
    let lowered = query.text.to_lowercase();
    let wanted = words(crate::js::trim(&lowered));
    let mut found: Vec<&CatalogEntry> = catalog()
        .iter()
        .filter(|entry| {
            query.source.is_none_or(|source| entry.source == source)
                && query
                    .format
                    .is_none_or(|format| entry.variants.iter().any(|variant| variant.format == format))
                && {
                    let mut said = vec![
                        entry.name.clone(),
                        entry.publisher.clone(),
                        entry.family.clone(),
                        entry.summary.clone(),
                    ];
                    said.extend(entry.tags.iter().cloned());
                    said.extend(entry.capabilities.iter().map(|one| one.key().to_owned()));
                    let said = said.join(" ").to_lowercase();
                    wanted.iter().all(|word| said.contains(word))
                }
        })
        .collect();
    found.sort_by(|a, b| {
        b.downloads
            .partial_cmp(&a.downloads)
            .unwrap_or(Ordering::Equal)
            .then_with(|| locale_compare(&a.name, &b.name, false))
    });
    found
}

/// Milliseconds a search takes to answer: longer the more words it has.
pub fn search_delay(query: &CatalogQuery) -> i64 {
    let lowered = query.text.to_lowercase();
    280 + words(crate::js::trim(&lowered)).len() as i64 * 60
}

pub fn entry_of(id: &str) -> Option<&'static CatalogEntry> {
    catalog().iter().find(|entry| entry.id == id)
}
