//! The vault a new reader starts with, as the web's `seed.ts`: Tidevann's runbooks, jobs,
//! projects, people, meetings and a journal.

use crate::model::{DAY, Folder, HOUR, Note, day};

/// The folders, each `(id, name, parent)`.
const FOLDERS: [(&str, &str, Option<&str>); 8] = [
    ("daily", "Daily", None),
    ("runbooks", "Runbooks", None),
    ("jobs", "Jobs", None),
    ("projects", "Projects", None),
    ("meetings", "Meetings", None),
    ("people", "People", None),
    ("services", "Services", Some("projects")),
    ("templates", "Templates", None),
];

/// The vault's folders as it starts, unranked.
pub fn seed_folders() -> Vec<Folder> {
    FOLDERS
        .iter()
        .map(|(id, name, parent)| Folder {
            id: (*id).to_owned(),
            name: (*name).to_owned(),
            parent: parent.map(str::to_owned),
            rank: None,
        })
        .collect()
}

/// The vault as it starts at `now`: the notes below, then the journal, its days counted from today.
pub fn seed_notes(now: i64) -> Vec<Note> {
    let journal = DAILY.iter().map(|(offset, body)| {
        let date = day(*offset, now);
        let at = now + offset * DAY;
        Note {
            id: format!("daily-{date}"),
            title: date.clone(),
            folder: Some("daily".to_owned()),
            body: body.replace("{date}", &date),
            created: at,
            updated: at + 8 * HOUR,
            day: Some(date),
            rank: None,
        }
    });
    let notes = NOTES.iter().map(|(id, title, folder, hours, body)| Note {
        id: (*id).to_owned(),
        title: (*title).to_owned(),
        folder: folder.map(str::to_owned),
        body: (*body).to_owned(),
        created: now - hours * HOUR - 30 * DAY,
        updated: now - hours * HOUR,
        day: None,
        rank: None,
    });
    journal.chain(notes).collect()
}

/// The notes, each `(id, title, folder, hours ago, body)`.
pub const NOTES: [(&str, &str, Option<&str>, i64, &str); 29] = [
    (
        "welcome",
        "Start here",
        None,
        400,
        r#"# Start here

This vault holds what the team knows about Tidevann's jobs: how they run, what to do when one
fails, and what was said about them.

- The [[Runbooks]] say what to do when something breaks: start with [[Retry a run]].
- Each job has a note: [[nightly-export]], [[sync-inventory]], [[rebuild-index]], [[send-digest]]
  and [[import-orders]].
- The projects are [[harbor]], [[juniper]] and [[larkspur]].
- The daily notes are the journal; open today's from **Daily** in the rail.

Type `[[` in the editor to link to another note, and `#` to tag one. #welcome
"#,
    ),
    (
        "runbooks-index",
        "Runbooks",
        Some("runbooks"),
        390,
        r#"# Runbooks

What to do, step by step, when something needs a person.

| Runbook | When |
| --- | --- |
| [[Retry a run]] | A run failed and the cause is fixed |
| [[Restore an export]] | An export was overwritten or lost |
| [[Rotate the api keys]] | Every 90 days, or after a leak |
| [[Scheduler is stuck]] | Runs stay queued past their time |
| [[On call]] | Who answers this week, and how |

#runbook
"#,
    ),
    (
        "retry",
        "Retry a run",
        Some("runbooks"),
        120,
        r#"# Retry a run

A failed run can start again from its failed step once the cause is fixed.

## Steps

1. Open the run and read the failed step's log.
2. Fix the cause: a bad row in [[import-orders]], an upload that timed out, a key that expired.
3. Press **Run again from the failed step**. The steps before it keep their output.
4. Watch the validate step; it fails the same way if the fix did not hold.

> [!WARNING]
> Never retry nightly-export twice at once: the second run overwrites the first one's upload.

Asked about in [[Weekly sync with grace]]. #runbook #on-call
"#,
    ),
    (
        "restore",
        "Restore an export",
        Some("runbooks"),
        300,
        r#"# Restore an export

The larkspur bucket keeps a week of exports. To bring one back:

```sh
tidevann exports restore --project larkspur --day 2026-11-02
```

How long exports are kept is open: see [[Export retention]]. #runbook
"#,
    ),
    (
        "rotate",
        "Rotate the api keys",
        Some("runbooks"),
        500,
        r#"# Rotate the api keys

Every 90 days, or at once after a key leaks.

1. Make the new key on the [[api]] service.
2. Put it in the worker's settings and restart the [[worker]].
3. Run sync-inventory once by hand to check it.
4. Revoke the old key.

grace did the last rotation. #runbook #security
"#,
    ),
    (
        "stuck",
        "Scheduler is stuck",
        Some("runbooks"),
        260,
        r#"# Scheduler is stuck

Runs stay queued past their time and nothing starts.

- Check the [[scheduler]] service answers on scheduler.example.org.
- Look for a run holding the lock: usually rebuild-index after a deploy.
- Restart the scheduler; queued runs start in their order.

Happened twice in October; see [[Incident review: sync-inventory]]. #runbook #on-call
"#,
    ),
    (
        "oncall",
        "On call",
        Some("runbooks"),
        48,
        r#"# On call

This week: [[grace]], then [[linus]].

The one on call answers failed runs within an hour in working time, starting from [[Retry a run]].
A failure of nightly-export after midnight waits for the morning unless send-digest depends on it.

#on-call
"#,
    ),
    (
        "nightly-export",
        "nightly-export",
        Some("jobs"),
        30,
        r#"# nightly-export

Runs every night at 02:00 on the [[worker]] for [[larkspur]].

## Steps

fetch, transform, validate, upload, notify.

## Owner

[[ada]]. The digest that [[send-digest]] mails on Mondays reads its output.

## Known failures

- validate rejects orders without an `order_id`; fixed upstream in [[import-orders]].
- upload times out when the bucket is busy; a retry works.

#job
"#,
    ),
    (
        "sync-inventory",
        "sync-inventory",
        Some("jobs"),
        70,
        r#"# sync-inventory

Syncs the inventory from the warehouses into [[harbor]] every hour.

Fails now and then on the fetch step when the [[api]] is slow; see
[[Incident review: sync-inventory]].

Owner: [[grace]]. #job
"#,
    ),
    (
        "rebuild-index",
        "rebuild-index",
        Some("jobs"),
        150,
        r#"# rebuild-index

Rebuilds the search index of [[juniper]] every Sunday. Takes about forty minutes and holds the
scheduler's lock while it runs.

Owner: [[linus]]. Speeding it up is on the list for the [[Quarter review]]. #job
"#,
    ),
    (
        "send-digest",
        "send-digest",
        Some("jobs"),
        90,
        r#"# send-digest

Mails each project's owner a summary of the week every Monday at 09:00, from what
[[nightly-export]] wrote.

The wording is [[margaret]]'s; drafts are in [[Planning the digest]]. #job
"#,
    ),
    (
        "import-orders",
        "import-orders",
        Some("jobs"),
        110,
        r#"# import-orders

Imports the day's orders for [[juniper]] from the shops' files.

The validate step was added after rows without an order id reached nightly-export.

Owner: [[linus]]. #job
"#,
    ),
    (
        "harbor",
        "harbor",
        Some("projects"),
        200,
        r#"# harbor

The inventory project. Its jobs: [[sync-inventory]].

Lead: [[grace]]. Moving harbor to the new scheduler is planned for next quarter; see the
[[Quarter review]]. #project
"#,
    ),
    (
        "juniper",
        "juniper",
        Some("projects"),
        210,
        r#"# juniper

Orders and search. Its jobs: [[import-orders]] and [[rebuild-index]].

Lead: [[linus]]. #project
"#,
    ),
    (
        "larkspur",
        "larkspur",
        Some("projects"),
        220,
        r#"# larkspur

Exports and reports. Its jobs: [[nightly-export]] and [[send-digest]].

Lead: [[ada]]. The bucket keeps a week of exports. #project
"#,
    ),
    (
        "api",
        "api",
        Some("services"),
        320,
        r#"# api

Answers on api.example.org. Every job's fetch step reads from it.

Keys are rotated by [[Rotate the api keys]]. #service
"#,
    ),
    (
        "worker",
        "worker",
        Some("services"),
        330,
        r#"# worker

Runs the jobs' steps. Four of them, each taking one step at a time.

#service
"#,
    ),
    (
        "scheduler",
        "scheduler",
        Some("services"),
        340,
        r#"# scheduler

Starts runs on their schedules. When it stops starting them, see [[Scheduler is stuck]].

#service
"#,
    ),
    (
        "ada",
        "ada",
        Some("people"),
        600,
        r#"# ada

Owns [[nightly-export]] and leads [[larkspur]]. Ask about exports and reports.

#person
"#,
    ),
    (
        "grace",
        "grace",
        Some("people"),
        610,
        r#"# grace

Leads [[harbor]] and owns [[sync-inventory]]. On call this week.

#person
"#,
    ),
    (
        "linus",
        "linus",
        Some("people"),
        620,
        r#"# linus

Leads [[juniper]]; owns [[import-orders]] and [[rebuild-index]].

#person
"#,
    ),
    (
        "margaret",
        "margaret",
        Some("people"),
        630,
        r#"# margaret

Writes the wording of [[send-digest]] and runs the [[Quarter review]].

#person
"#,
    ),
    (
        "weekly-sync",
        "Weekly sync with grace",
        Some("meetings"),
        26,
        r#"# Weekly sync with grace

With [[grace]] and [[ada]].

- sync-inventory failed twice on Tuesday; the api was slow. grace adds a retry to the fetch step.
- [[Retry a run]] needs a line on what not to retry twice.
- Who keeps the exports longer than a week? Open: [[Export retention]].

## Decisions

- Retries stay manual for nightly-export. #decision

#meeting
"#,
    ),
    (
        "quarter-review",
        "Quarter review",
        Some("meetings"),
        170,
        r#"# Quarter review

With [[margaret]], [[grace]], [[linus]] and [[ada]].

- [[harbor]] moves to the new scheduler next quarter.
- [[rebuild-index]] should finish under twenty minutes.
- [[larkspur]] asks for exports kept a month.

#meeting
"#,
    ),
    (
        "digest-planning",
        "Planning the digest",
        Some("meetings"),
        95,
        r#"# Planning the digest

With [[margaret]].

- The digest leads with the failures, then the slowest steps.
- A project with no runs gets no mail.
- Send it at 09:00 on Mondays, after nightly-export has run.

#meeting #send-digest
"#,
    ),
    (
        "incident-sync",
        "Incident review: sync-inventory",
        Some("meetings"),
        140,
        r#"# Incident review: sync-inventory

On Tuesday sync-inventory failed four times between 10:00 and 14:00.

## What happened

The [[api]] answered slowly under load, and the fetch step gave up after 30 seconds.

## What we changed

- The fetch step waits 60 seconds and tries twice.
- The [[scheduler]] no longer starts a second run while one is going.

Follow up in [[Postmortem template]]. #incident
"#,
    ),
    (
        "meeting-template",
        "Meeting template",
        Some("templates"),
        700,
        r#"# Meeting

With:

## Notes

## Decisions

## Actions

- [ ]

#meeting
"#,
    ),
    (
        "reading",
        "Reading list",
        None,
        800,
        r#"# Reading list

- How the scheduler orders queued runs.
- A guide to writing runbooks people follow.

#idea
"#,
    ),
    (
        "ideas",
        "Ideas",
        None,
        75,
        r#"# Ideas

- A dashboard of the slowest steps of each job, for the [[Quarter review]].
- Tag every runbook with the jobs it touches.
- Let send-digest say which runs were retried.

#idea
"#,
    ),
];

/// The journal: `(days from today, body)`, `{date}` in the body standing for the day.
pub const DAILY: [(i64, &str); 7] = [
    (
        -1,
        r#"# {date}

## Plan

- [x] Read the failed nightly-export from last night
- [ ] Ask linus about the validate step

## Log

- nightly-export failed on upload at 02:14; retried at 08:30 and it went through. Followed [[Retry a run]].
- Talked to grace about sync-inventory; see [[Weekly sync with grace]].

#journal
"#,
    ),
    (
        -2,
        r#"# {date}

## Log

- Rotated the api keys with grace. It took twenty minutes; the runbook was right.
- sync-inventory ran clean all day.

#journal
"#,
    ),
    (
        -3,
        r#"# {date}

## Log

- Quiet day. Read up on how the scheduler orders runs.
- Drafted the digest wording with margaret: [[Planning the digest]].

#journal
"#,
    ),
    (
        -6,
        r#"# {date}

## Log

- rebuild-index held the lock for an hour after the deploy; restarted the scheduler as the
  runbook says.
- Added a line to [[Scheduler is stuck]].

#journal #on-call
"#,
    ),
    (
        -7,
        r#"# {date}

## Log

- On call. Two failed runs of sync-inventory, both on fetch.
- import-orders rejected 12 rows without an order id.

#journal #on-call
"#,
    ),
    (
        -9,
        r#"# {date}

## Log

- Quarter review prep: collected the slowest steps of each job for [[Quarter review]].

#journal
"#,
    ),
    (
        -13,
        r#"# {date}

## Log

- First day keeping a journal in the vault. Started [[Start here]].

#journal
"#,
    ),
];
