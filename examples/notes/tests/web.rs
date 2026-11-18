//! The data layer held against the web's own: `web/data.json` and `web/ops.json` are what the web
//! example's modules (`model.ts`, `seed.ts`, `vault.ts`, `backend.ts`, `state.tsx`, the Search
//! page's `searchVault`, sokkel-ui's `wiki-links.ts`, `search.ts` and `markdownHeadings`) give
//! under bun at 2026-11-18 09:00 UTC, the clock fixed and the zone UTC. Each test builds the same
//! JSON here and asks for it to be equal.

use notes::backend::{Backend, Follow, NewNote, Notice, Placing, TabClosed};
use notes::model::{
    Order, TEMPLATE, View, bad_title, clock, day, free_name, from_template, locale_compare, long_day, word_count,
};
use notes::search::{SearchChosen, VaultSearch, query_ranges, query_words, search_vault};
use notes::vault::{
    MarkdownHeading, Vault, folder_and_below, folder_tree, line_around, markdown_headings, notes_in_order,
    rename_links, unlinked_mentions, vault_tree,
};
use notes::wiki_links::{WikiLink, mentions_of, tag_matches, tags_in, wiki_link_query, wiki_links_in, without_code};
use serde_json::{Value, json};
use sokkel_egui::TreeNode;

const NOW: i64 = 1_794_992_400_000;

fn web(name: &str) -> Value {
    let path = format!("{}/tests/web/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("the web's dump")).expect("JSON")
}

/// A byte offset into a text as the web counts it, in UTF-16 units.
fn units(text: &str, at: usize) -> usize {
    text[..at].encode_utf16().count()
}

fn link(text: &str, link: &WikiLink) -> Value {
    json!({
        "target": link.target,
        "heading": link.heading,
        "label": link.label,
        "from": units(text, link.from),
        "to": units(text, link.to),
    })
}

fn headings(list: &[MarkdownHeading]) -> Value {
    list.iter()
        .map(|one| json!({ "id": one.id, "label": one.label, "level": one.level }))
        .collect()
}

fn tree(nodes: &[TreeNode], folders_only: bool) -> Value {
    nodes
        .iter()
        .map(|node| {
            let branch = folders_only || node.id.starts_with("folder:");
            json!({
                "id": node.id,
                "label": node.label,
                "children": if branch { tree(&node.children, folders_only) } else { Value::Null },
            })
        })
        .collect()
}

fn searched(vault: &Vault, query: &str, chosen: Value) -> Value {
    let picked: SearchChosen = serde_json::from_value(chosen.clone()).expect("choices");
    let VaultSearch { groups, facets } = search_vault(vault, query, &picked);
    json!({
        "query": query,
        "chosen": chosen,
        "groups": groups.iter().map(|group| json!({
            "kind": group.kind,
            "title": group.title,
            "total": group.total,
            "results": group.results.iter().map(|result| json!({
                "id": result.id,
                "title": result.title,
                "detail": result.detail,
                "meta": result.meta,
                "folder": result.facets["folder"],
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "facets": facets.iter().map(|facet| json!({
            "id": facet.id,
            "title": facet.title,
            "values": facet.values.iter().map(|value| json!({
                "value": value.value,
                "label": value.label,
                "count": value.count,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// Asks for each key of the web's dump to be equal, one at a time, so a failure names the part.
fn same(ours: &Value, theirs: &Value) {
    let (Value::Object(ours), Value::Object(theirs)) = (ours, theirs) else {
        panic!("not objects");
    };
    for (key, value) in theirs {
        match (ours.get(key), value) {
            (Some(Value::Array(mine)), Value::Array(web)) => {
                assert_eq!(mine.len(), web.len(), "{key}: lengths");
                for (at, (mine, web)) in mine.iter().zip(web).enumerate() {
                    assert_eq!(mine, web, "{key}[{at}]");
                }
            }
            (mine, web) => assert_eq!(mine, Some(web), "{key}"),
        }
    }
    assert_eq!(ours.len(), theirs.len(), "keys");
}

#[test]
fn the_seeded_vault_reads_as_the_web_reads_it() {
    let backend = Backend::seeded(NOW);
    let vault = backend.vault();
    let notes: Vec<Value> = vault
        .notes
        .iter()
        .map(|note| {
            json!({
                "id": note.id,
                "title": note.title,
                "folder": note.folder,
                "created": note.created,
                "updated": note.updated,
                "day": note.day,
                "rank": note.rank,
                "tags": vault.tags_of(&note.id),
                "links": vault.links(&note.id).iter().map(|one| link(&note.body, one)).collect::<Vec<_>>(),
                "backlinks": vault.backlinks(&note.id).iter().map(|back| {
                    let from = &vault.note(&back.from).expect("a note").body;
                    json!({ "from": back.from, "link": link(from, &back.link), "line": back.line })
                }).collect::<Vec<_>>(),
                "mentions": unlinked_mentions(vault, note).iter().map(|mention| {
                    let from = &vault.note(&mention.from).expect("a note").body;
                    json!({
                        "from": mention.from,
                        "at": [units(from, mention.at.start), units(from, mention.at.end)],
                        "line": mention.line,
                    })
                }).collect::<Vec<_>>(),
                "outline": headings(&markdown_headings(&note.body, &[1, 2, 3, 4])),
                "words": word_count(&note.body),
                "path": vault.path(note.folder.as_deref()),
            })
        })
        .collect();
    let ids = |list: Vec<&notes::model::Note>| list.into_iter().map(|note| note.id.clone()).collect::<Vec<_>>();
    // The texts `data.ts` reads links and tags in.
    let texts = [
        "See [[Retry a run]] and [[nightly-export#Steps|the steps]].",
        "[[ Spaced  ]] [[#Heading only]] [[a|]] [[b#]] [[c#h|w]]",
        "[[[nested]]] [[one\ntwo]] [[x]y]] [[]] [[ok]]",
        "Code `[[not a link]]` and ``a `[[x]]` b`` then [[yes]]",
        "```\n[[fenced]]\n```\n[[after]]\n~~~~\n[[tilde]]\n~~~\n~~~~\n[[out]]",
        "#tag at start, #two-words #1 #a1 (#paren) \"#quote\" x#no #with/slash #\u{fc}n\u{ef}code #_u [[l #nottag]] `#code`",
        "# Heading\n#tag\n  #indented #a#b",
    ];
    let heading_text = "# Title\n\n## Steps\n\n### Sub **bold** `code`\n\nText\n===\n\nOther\n---\n\n- item\n---\n\n## Steps\n## Steps\n#### Deep [link](http://x) ##\n```\n# not\n```\n#nospace\n#    \n# #\n## trailing ###   ";
    let collate = [
        "a",
        "A",
        "b",
        "B",
        "a-b",
        "a b",
        "ab",
        "a_b",
        "a:b",
        "a.b",
        "item 2",
        "item 10",
        "Item 2",
        "x9",
        "x10",
        "x09",
        "2026-11-05",
        "2026-11-17",
        "10",
        "9",
        "Zebra",
        "apple",
        "#tag",
        "(x)",
        "",
        "Start here",
        "Runbooks",
        "Vault",
        "Projects / Services",
        "nightly-export",
        "Incident review: sync-inventory",
        "api",
        "Api",
    ];
    let signs = |numeric: bool| -> Value {
        collate
            .iter()
            .map(|a| {
                collate
                    .iter()
                    .map(|b| locale_compare(a, b, numeric) as i8)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into()
    };
    let long = format!("{} word {}", "x".repeat(200), "y".repeat(200));
    let edge = format!("{}{}", "a".repeat(50), "b".repeat(80));
    let ours = json!({
        "notes": notes,
        "folders": backend.folders.iter().map(|folder| json!({
            "id": folder.id, "name": folder.name, "parent": folder.parent, "rank": folder.rank,
        })).collect::<Vec<_>>(),
        "prefs": serde_json::to_value(&backend.prefs).expect("prefs"),
        "tagged": vault.tags().iter().map(|(tag, ids)| json!([tag, ids])).collect::<Vec<_>>(),
        "order": {
            "manual": ids(notes_in_order(vault, None, Order::Manual, Some("daily"))),
            "name": ids(notes_in_order(vault, None, Order::Name, Some("daily"))),
            "projects": ids(notes_in_order(vault, Some("projects"), Order::Name, Some("daily"))),
            "tree": tree(&vault_tree(vault, Some("daily"), Order::Manual), false),
            "folderTree": tree(&folder_tree(vault, "Vault", Order::Name), true),
            "below": folder_and_below(vault, "projects"),
        },
        "searches": [
            searched(vault, "retry", json!({})),
            searched(vault, "nightly export", json!({})),
            searched(vault, "grace", json!({})),
            searched(vault, "Scheduler STUCK", json!({})),
            searched(vault, "on-call", json!({})),
            searched(vault, "run run", json!({})),
            searched(vault, "xyz", json!({})),
            searched(vault, "2026", json!({})),
            searched(vault, "   ", json!({})),
            searched(vault, "the", json!({ "kind": ["text"] })),
            searched(vault, "the", json!({ "folder": ["Runbooks", "Jobs"] })),
            searched(vault, "api", json!({ "kind": ["names"], "folder": ["Projects / Services"] })),
        ],
        "queryWords": (["  Run  run RUNS r ", "a b ab", ""].map(query_words)),
        "queryRanges": query_ranges("Retry a run, run again", "run re")
            .into_iter()
            .map(|range| json!({ "start": range.start, "end": range.end }))
            .collect::<Vec<_>>(),
        "today": {
            "day": day(0, NOW),
            "yesterday": day(-1, NOW),
            "long": long_day(&day(0, NOW)),
            "template": from_template(&format!("{TEMPLATE}{{{{long}}}}"), &day(0, NOW)),
            "clock": clock(NOW + 5 * 60_000),
            "dateOf": long_day("2026-13-01"),
            "invalid": long_day("x"),
        },
        "names": {
            "free": [free_name("Untitled", &["untitled", "Untitled 2"]), free_name("New", &[])],
            "bad": [bad_title("  ", &[]), bad_title("a#b", &[]), bad_title(" Grace ", &["grace"]), bad_title("ok", &["grace"])],
        },
        "words": word_count("  one two \u{2014} 3 ** x-y "),
        "wiki": texts.iter().map(|text| json!({
            "links": wiki_links_in(text).iter().map(|one| link(text, one)).collect::<Vec<_>>(),
            "tags": tag_matches(text).iter().map(|one| json!({
                "tag": one.tag, "from": units(text, one.from), "to": units(text, one.to),
            })).collect::<Vec<_>>(),
            "tagsIn": tags_in(text),
            "code": without_code(text),
        })).collect::<Vec<_>>(),
        "query": [
            wiki_link_query("see [[Ret", 9).map(|one| json!({ "start": one.start, "query": one.query })),
            wiki_link_query("a\n[[x]] b", 9).map(|one| json!({ "start": one.start, "query": one.query })),
            wiki_link_query("[[a", 3).map(|one| json!({ "start": one.start, "query": one.query })),
        ],
        "mentions": ([
            ("nightly-export ran; Nightly-Export again; nightly-exports no; [[nightly-export]] linked; `nightly-export` code", "nightly-export"),
            ("grace and graceful, grace_x, (grace).", "Grace"),
            ("nothing", ""),
        ].map(|(text, name)| mentions_of(text, name).into_iter().map(|range| json!({ "from": range.start, "to": range.end })).collect::<Vec<_>>())),
        "headings": [headings(&markdown_headings(heading_text, &[1, 2, 3, 4]))],
        "headingsDefault": [headings(&markdown_headings(heading_text, &[2, 3]))],
        "renamed": rename_links("[[Old]] [[old#H|w]] [[Older]] [[ old ]]", "Old", "New"),
        "around": [line_around("short line\nnext", 2, 4, 90), line_around(&long, 201, 205, 90), line_around(&edge, 10, 11, 90)],
        "collate": signs(true),
        "collatePlain": signs(false),
        "collateList": collate.as_slice(),
    });
    same(&ours, &web("data.json"));
}

fn toast(notice: &Notice) -> Value {
    json!({ "title": notice.title, "description": notice.description })
}

/// One step of `ops.tsx` as it was answered: what it gave or why it was refused, its toasts, and the
/// address after it.
struct Step {
    answer: Value,
    error: Option<String>,
    toasts: Vec<Value>,
}

impl Step {
    fn of(answer: Value) -> Self {
        Self {
            answer,
            error: None,
            toasts: Vec::new(),
        }
    }

    fn done<T>(result: Result<T, String>, answer: impl FnOnce(T) -> (Value, Vec<Value>)) -> Self {
        match result {
            Ok(value) => {
                let (answer, toasts) = answer(value);
                Self {
                    answer,
                    error: None,
                    toasts,
                }
            }
            Err(error) => Self {
                answer: Value::Null,
                error: Some(error),
                toasts: Vec::new(),
            },
        }
    }
}

#[test]
fn changes_land_as_the_web_lands_them() {
    let theirs = web("ops.json");
    let mut backend = Backend::seeded(NOW);
    let mut now = NOW;
    let mut hash = "#/notes".to_owned();
    let mut heading: Option<String> = None;
    let mut asking: Option<Value> = None;
    let mut steps: Vec<Value> = Vec::new();
    let notice = |result: Result<Notice, String>| Step::done(result, |notice| (Value::Null, vec![toast(&notice)]));
    let placed = |result: Result<Option<Notice>, String>| {
        Step::done(result, |notice| (Value::Null, notice.iter().map(toast).collect()))
    };
    let names = [
        "create in runbooks",
        "create untitled",
        "create untitled again",
        "rename nightly-export",
        "rename retry",
        "rename same",
        "rename empty",
        "rename bar",
        "rename taken",
        "rename gone",
        "move ideas",
        "move to top",
        "edit",
        "inspect and flush one",
        "flush all",
        "open today",
        "open yesterday",
        "folder",
        "folder again",
        "folder blank",
        "rename folder",
        "rename folder blank",
        "rename folder taken",
        "move folder",
        "move folder inside",
        "place note first",
        "place note across",
        "place note far",
        "place folder",
        "place folder inside",
        "order by name",
        "place by name",
        "place by name same",
        "place folder by name",
        "order manual",
        "show harbor",
        "show retry",
        "view",
        "close active",
        "close other",
        "delete note",
        "delete harbor",
        "daily folder",
        "delete folder",
        "follow known",
        "follow unknown",
    ];
    for name in names {
        now += 1000;
        let close = |closed: TabClosed, hash: &mut String| {
            if let TabClosed::Active { next } = closed
                && hash.starts_with("#/note/")
            {
                *hash = next.map_or("#/notes".to_owned(), |next| format!("#/note/{next}"));
            }
        };
        let step = match name {
            "create in runbooks" => Step::of(json!(
                backend
                    .create_note(
                        NewNote {
                            title: Some(" Deploy checklist ".to_owned()),
                            folder: Some("runbooks".to_owned()),
                            ..NewNote::default()
                        },
                        now
                    )
                    .id
            )),
            "create untitled" => Step::of(json!(backend.create_note(NewNote::default(), now).id)),
            "create untitled again" => Step::of(json!(
                backend
                    .create_note(
                        NewNote {
                            title: Some("   ".to_owned()),
                            ..NewNote::default()
                        },
                        now
                    )
                    .id
            )),
            "rename nightly-export" => {
                Step::done(backend.rename_note("nightly-export", "nightly-export-v2", now), |n| {
                    (json!(n), vec![])
                })
            }
            "rename retry" => Step::done(backend.rename_note("retry", " Retry a failed run ", now), |n| {
                (json!(n), vec![])
            }),
            "rename same" => Step::done(backend.rename_note("grace", "grace", now), |n| (json!(n), vec![])),
            "rename empty" => Step::done(backend.rename_note("grace", "  ", now), |n| (json!(n), vec![])),
            "rename bar" => Step::done(backend.rename_note("grace", "a|b", now), |n| (json!(n), vec![])),
            "rename taken" => Step::done(backend.rename_note("grace", "LINUS", now), |n| (json!(n), vec![])),
            "rename gone" => Step::done(backend.rename_note("nope", "x", now), |n| (json!(n), vec![])),
            "move ideas" => notice(backend.move_note("ideas", Some("projects"))),
            "move to top" => notice(backend.move_note("api", None)),
            "edit" => {
                backend.edit("ideas", "# Ideas\n\nSee [[harbor]] and grace. #idea #new\n", now);
                backend.edit("welcome", "Start here: harbor\n", now);
                Step::of(Value::Null)
            }
            "inspect and flush one" => {
                let vault = backend.vault();
                let seen = json!({
                    "drafts": backend.drafted(),
                    "backlinks": vault.backlinks("harbor").iter().map(|one| one.from.clone()).collect::<Vec<_>>(),
                    "mentions": vault.tagged("new"),
                });
                backend.flush(Some("ideas"), now);
                Step::of(seen)
            }
            "flush all" => {
                backend.flush(None, now);
                Step::of(Value::Null)
            }
            "open today" | "open yesterday" => {
                let date = if name == "open today" {
                    "2026-11-18"
                } else {
                    "2026-11-17"
                };
                let (id, made) = backend.open_day(date, now);
                hash = format!("#/note/{id}");
                Step {
                    answer: Value::Null,
                    error: None,
                    toasts: made.iter().map(toast).collect(),
                }
            }
            "folder" => Step::of(json!(backend.create_folder(" Archive ", None, now).id)),
            "folder again" => Step::of(json!(backend.create_folder("archive", None, now).id)),
            "folder blank" => Step::of(json!(backend.create_folder("", Some("projects"), now).id)),
            "rename folder" => Step::done(backend.rename_folder("templates", " Stencils "), |()| {
                (Value::Null, vec![])
            }),
            "rename folder blank" => Step::done(backend.rename_folder("templates", " "), |()| (Value::Null, vec![])),
            "rename folder taken" => Step::done(backend.rename_folder("templates", "jobs"), |()| (Value::Null, vec![])),
            "move folder" => notice(backend.move_folder("services", None)),
            "move folder inside" => notice(backend.move_folder("projects", Some("projects"))),
            "place note first" => placed(backend.place(Placing::Note, "linus", Some("people"), 0)),
            "place note across" => placed(backend.place(Placing::Note, "harbor", Some("jobs"), 2)),
            "place note far" => placed(backend.place(Placing::Note, "worker", Some("jobs"), 99)),
            "place folder" => placed(backend.place(Placing::Folder, "meetings", None, 0)),
            "place folder inside" => placed(backend.place(Placing::Folder, "jobs", Some("jobs"), 0)),
            "order by name" | "order manual" => {
                let order = if name == "order by name" {
                    Order::Name
                } else {
                    Order::Manual
                };
                backend.set_prefs(|prefs| prefs.order = Some(order));
                Step::of(Value::Null)
            }
            "place by name" => placed(backend.place(Placing::Note, "ada", Some("meetings"), 0)),
            "place by name same" => placed(backend.place(Placing::Note, "grace", Some("people"), 3)),
            "place folder by name" => placed(backend.place(Placing::Folder, "people", Some("jobs"), 0)),
            "show harbor" => {
                backend.show("harbor");
                Step::of(Value::Null)
            }
            "show retry" => {
                backend.show("retry");
                Step::of(Value::Null)
            }
            "view" => {
                backend.set_view("harbor", View::Edit);
                Step::of(Value::Null)
            }
            "close active" | "close other" => {
                let id = if name == "close active" {
                    "retry"
                } else {
                    "nightly-export"
                };
                close(backend.close_tab(id, now), &mut hash);
                Step::of(Value::Null)
            }
            "delete note" | "delete harbor" => {
                let id = if name == "delete note" { "reading" } else { "harbor" };
                let (said, closed) = backend.delete_note(id, now);
                close(closed, &mut hash);
                Step {
                    answer: Value::Null,
                    error: None,
                    toasts: vec![toast(&said)],
                }
            }
            "daily folder" => {
                backend.set_prefs(|prefs| prefs.daily_folder = Some("meetings".to_owned()));
                Step::of(Value::Null)
            }
            "delete folder" => {
                let (said, closed) = backend.delete_folder("meetings", now);
                close(closed, &mut hash);
                Step {
                    answer: Value::Null,
                    error: None,
                    toasts: said.iter().map(toast).collect(),
                }
            }
            "follow known" | "follow unknown" => {
                let (target, from) = if name == "follow known" {
                    ("  JUNIPER ", None)
                } else {
                    (" Fresh note ", Some("juniper"))
                };
                match backend.follow(target, from) {
                    Follow::Open { id } => {
                        hash = format!("#/note/{id}");
                        heading = Some("Steps".to_owned());
                    }
                    Follow::Create { title, folder } => {
                        asking = Some(json!({ "kind": "create", "title": title, "folder": folder }));
                    }
                }
                Step::of(Value::Null)
            }
            other => panic!("no step {other}"),
        };
        // Drafts come due as the clock moves, as the web's timers fire while a step settles.
        backend.tick(now);
        steps.push(json!({
            "name": name,
            "answer": step.answer,
            "error": step.error,
            "toasts": step.toasts,
            "hash": hash,
        }));
    }
    let ours = json!({
        "steps": steps,
        "asking": asking,
        "heading": heading,
        "prefs": serde_json::to_value(&backend.prefs).expect("prefs"),
        "drafts": {},
        "folders": serde_json::to_value(&backend.vault().folders).expect("folders"),
        "notes": serde_json::to_value(&backend.vault().notes).expect("notes"),
        "tree": notes_in_order(backend.vault(), None, Order::Manual, backend.prefs.daily_folder.as_deref())
            .into_iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
    });
    assert!(backend.drafted().is_empty());
    same(&ours, &theirs);
}
