//! What a run wrote and gave back, up to a moment, worked out from where it stands, as the web's
//! `logs.ts`.

use serde_json::{Value, json};
use sokkel_egui::{LogLevel, LogLine};

use crate::model::{By, Run, Status, clock, iso_string, thousands, took};
use crate::seed::seeded;
use crate::timeline::RunAt;

/// A moment of the log on the UTC clock with its milliseconds, `07:15:19.155`; a fraction of a
/// millisecond is dropped, as `Date` drops it.
fn time(at: f64) -> String {
    let whole = at.trunc() as i64;
    format!("{}.{:03}", clock(whole, true), whole.rem_euclid(1000))
}

/// What a step says while it works, by the word its code starts with.
fn working(code: &str, run: &Run) -> Vec<String> {
    let project = run.project.word();
    let rows = thousands(run.records);
    match code.split('-').next().unwrap_or_default() {
        "fetch" => {
            let rest = code.strip_prefix("fetch").unwrap_or(code);
            let rest = rest.strip_prefix('-').unwrap_or(rest);
            let what = if rest.is_empty() { run.job.as_str() } else { rest };
            vec![
                format!("GET https://api.example.org/{project}/{what}"),
                format!("read {rows} rows"),
            ]
        }
        "transform" => vec![
            format!("joined and reshaped {rows} rows"),
            "wrote 4 columns: sku, amount, placed, region".to_owned(),
        ],
        "validate" => vec![format!("checked {rows} rows against the schema")],
        "upload" => vec![format!(
            "PUT https://storage.example.org/{project}/{}/{}.csv",
            run.job, run.id
        )],
        _ => vec![format!("sent the summary to {project}-team@example.org")],
    }
}

fn line(when: f64, level: LogLevel, source: &str, text: String) -> LogLine {
    LogLine {
        at: Some(time(when)),
        level: Some(level),
        source: Some(source.to_owned()),
        text,
        fields: Vec::new(),
    }
}

/// The run's log up to `now`: each step's start, what it did and how it ended.
pub fn run_log(run: &Run, at: &RunAt, now: i64) -> Vec<LogLine> {
    let now = now as f64;
    let mut lines: Vec<(f64, LogLine)> = Vec::new();
    let mut add = |when: f64, level: LogLevel, source: &str, text: String| {
        if when <= now {
            lines.push((when, line(when, level, source, text)));
        }
    };
    let who = match run.by {
        By::Schedule => "the scheduler",
        By::Person(person) => person.word(),
    };
    add(run.created as f64, LogLevel::Info, "run", format!("queued by {who}"));
    if let Some(skipped) = &run.skipped {
        add(
            (run.created + 1) as f64,
            LogLevel::Warning,
            "run",
            format!("skipped: {skipped}"),
        );
        return lines.into_iter().map(|(_, line)| line).collect();
    }
    let mut random = seeded(run.created);
    let worker = 1 + (random.draw() * 3.0).floor() as i64;
    add(
        at.started as f64,
        LogLevel::Info,
        "run",
        format!("picked up by worker-{worker}"),
    );
    for step in &at.steps {
        match step.status {
            Status::Queued => continue,
            Status::Skipped => {
                // A step skipped by a cancel says so at the cancel; one skipped by a failure, once the failure is in.
                let when = match run.cancelled_at {
                    Some(cancelled) if step.start >= cancelled => cancelled,
                    _ => step.end,
                };
                add(when as f64, LogLevel::Warning, &step.code, "skipped".to_owned());
                continue;
            }
            _ => {}
        }
        add(
            step.start as f64,
            LogLevel::Debug,
            &step.code,
            format!("started on {}", step.kind),
        );
        let said = working(&step.code, run);
        let parts = said.len() as f64 + 1.0;
        for (index, text) in said.into_iter().enumerate() {
            let when = step.start as f64 + ((step.end - step.start) as f64 * (index as f64 + 1.0)) / parts;
            add(when, LogLevel::Info, &step.code, text);
        }
        let ran = took((step.end - step.start) as f64);
        match step.status {
            Status::Failed => {
                let error = run.error.clone().unwrap_or_else(|| "failed".to_owned());
                add(step.end as f64, LogLevel::Error, &step.code, error);
                add(
                    (step.end + 1) as f64,
                    LogLevel::Error,
                    &step.code,
                    format!("failed after {ran}"),
                );
            }
            Status::Succeeded => add(step.end as f64, LogLevel::Info, &step.code, format!("done in {ran}")),
            Status::Cancelled => {
                if let Some(cancelled) = run.cancelled_at {
                    let by = run.cancelled_by.map_or("someone", |person| person.word());
                    add(
                        cancelled as f64,
                        LogLevel::Warning,
                        &step.code,
                        format!("cancelled by {by}"),
                    );
                }
            }
            _ => {}
        }
    }
    if let Some(ended) = at.ended {
        let duration = took(at.duration as f64);
        let (level, words) = match at.state {
            Status::Succeeded => (LogLevel::Info, format!("succeeded in {duration}")),
            Status::Failed => (
                LogLevel::Error,
                format!("failed at {} after {duration}", at.step.as_deref().unwrap_or("a step")),
            ),
            _ => (LogLevel::Warning, format!("cancelled after {duration}")),
        };
        add((ended + 2) as f64, level, "run", words);
    }
    // A stable sort, as the web's, keeps lines written at one moment in the order they were added.
    lines.sort_by(|left, right| left.0.total_cmp(&right.0));
    lines.into_iter().map(|(_, line)| line).collect()
}

/// What the run gave back once it ended: its rows and file, or what went wrong.
pub fn outputs_of(run: &Run, at: &RunAt) -> Option<Value> {
    match at.state {
        Status::Succeeded => Some(json!({
            "rows": run.records,
            "file": format!("https://storage.example.org/{}/{}/{}.csv", run.project.word(), run.job, run.id),
            "bytes": run.records * 38,
            "checksum": format!("sha256:{}{}", run.id.get(4..).unwrap_or_default(), "0f3a9c".repeat(3)),
        })),
        Status::Failed => Some(json!({ "error": { "step": at.step, "message": run.error } })),
        Status::Cancelled => Some(json!({
            "cancelled": {
                "by": run.cancelled_by.map(|person| person.word()),
                "at": at.ended.map(iso_string),
            }
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Person;
    use crate::seed::tests::{NOW, line as run_line};
    use crate::seed::{HISTORY, JOBS, history, settle};
    use crate::timeline::run_at;

    fn said(lines: &[LogLine]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                format!(
                    "{} {} {} {}",
                    line.at.as_deref().unwrap_or_default(),
                    line.level.map_or("", LogLevel::word),
                    line.source.as_deref().unwrap_or_default(),
                    line.text
                )
            })
            .collect()
    }

    fn steps(at: &RunAt) -> Vec<String> {
        at.steps
            .iter()
            .map(|step| {
                format!(
                    "{} {} {} {} {:?} {:?}",
                    step.code,
                    step.status.word(),
                    step.start,
                    step.end,
                    step.progress,
                    step.duration_ms
                )
            })
            .collect()
    }

    fn runs() -> Vec<Run> {
        history(&JOBS, NOW - HISTORY, NOW)
    }

    #[test]
    fn a_failed_run_says_what_the_web_says() {
        let runs = runs();
        let run = runs.iter().find(|run| run.state == Status::Failed).unwrap();
        assert_eq!(run.id, "run_toos50a9");
        let at = run_at(run, NOW);
        assert_eq!(
            steps(&at),
            [
                "fetch failed 1794986100470 1794986119155 None Some(18685)",
                "transform skipped 1794986119155 1794986119155 None None",
                "validate skipped 1794986119155 1794986119155 None None",
                "upload skipped 1794986119155 1794986119155 None None",
            ]
        );
        assert_eq!(
            said(&run_log(run, &at, NOW)),
            [
                "07:15:00.000 info run queued by the scheduler",
                "07:15:00.470 info run picked up by worker-3",
                "07:15:00.470 debug fetch started on http",
                "07:15:06.698 info fetch GET https://api.example.org/larkspur/sync-inventory",
                "07:15:12.926 info fetch read 236,979 rows",
                "07:15:19.155 error fetch GET https://api.example.org/larkspur/sync-inventory answered 503 after 4 tries",
                "07:15:19.155 warning transform skipped",
                "07:15:19.155 warning validate skipped",
                "07:15:19.155 warning upload skipped",
                "07:15:19.156 error fetch failed after 19s",
                "07:15:19.157 error run failed at fetch after 19s",
            ]
        );
        assert_eq!(
            outputs_of(run, &at).unwrap().to_string(),
            r#"{"error":{"step":"fetch","message":"GET https://api.example.org/larkspur/sync-inventory answered 503 after 4 tries"}}"#
        );
    }

    #[test]
    fn a_skipped_run_says_what_the_web_says() {
        let runs = runs();
        let run = runs.iter().find(|run| run.state == Status::Skipped).unwrap();
        assert_eq!(run.id, "run_tooj40pi");
        let at = run_at(run, NOW);
        assert_eq!(
            said(&run_log(run, &at, NOW)),
            [
                "04:00:00.000 info run queued by the scheduler",
                "04:00:00.001 warning run skipped: No worker slot was free within ten minutes.",
            ]
        );
        assert_eq!(outputs_of(run, &at), None);
        assert_eq!(at.ended, Some(1_794_974_400_000));
    }

    const MANUAL_LOG: [&str; 20] = [
        "07:09:57.368 info run queued by ada",
        "07:09:59.314 info run picked up by worker-2",
        "07:09:59.314 debug fetch started on http",
        "07:11:15.797 info fetch GET https://api.example.org/harbor/rebuild-index",
        "07:12:32.280 info fetch read 171,566 rows",
        "07:13:48.764 info fetch done in 3m 49s",
        "07:13:48.764 debug transform started on python",
        "07:15:34.111 info transform joined and reshaped 171,566 rows",
        "07:17:19.458 info transform wrote 4 columns: sku, amount, placed, region",
        "07:19:04.805 info transform done in 5m 16s",
        "07:19:04.805 debug validate started on schema",
        "07:21:49.097 info validate checked 171,566 rows against the schema",
        "07:24:33.389 info validate done in 5m 29s",
        "07:24:33.389 debug upload started on storage",
        "07:27:21.755 info upload PUT https://storage.example.org/harbor/rebuild-index/run_toorwlm.csv",
        "07:30:10.121 info upload done in 5m 37s",
        "07:30:10.121 debug notify started on webhook",
        "07:32:05.088 info notify sent the summary to harbor-team@example.org",
        "07:34:00.056 info notify done in 3m 50s",
        "07:34:00.058 info run succeeded in 24m 01s",
    ];

    fn manual() -> Run {
        runs()
            .into_iter()
            .find(|run| run.trigger == crate::model::Trigger::Manual && run.state == Status::Succeeded)
            .unwrap()
    }

    #[test]
    fn a_manual_run_says_what_the_web_says() {
        let run = manual();
        assert_eq!(
            run_line(&run, NOW),
            r#"run_toorwlm|rebuild-index|ada|manual|1794985797368|1946|229450,316041,328584,336732,229935|||||171566|{"project":"harbor","shards":6,"full":true}|succeeded|1794987240056|1794985799314|1440742|notify"#
        );
        let at = run_at(&run, NOW);
        assert_eq!(said(&run_log(&run, &at, NOW)), MANUAL_LOG);
        assert_eq!(
            outputs_of(&run, &at).unwrap().to_string(),
            r#"{"rows":171566,"file":"https://storage.example.org/harbor/rebuild-index/run_toorwlm.csv","bytes":6519508,"checksum":"sha256:toorwlm0f3a9c0f3a9c0f3a9c"}"#
        );
    }

    #[test]
    fn a_run_seen_midway_says_what_the_web_says() {
        let run = manual();
        let mid = 1_794_986_100_000;
        let at = run_at(&run, mid);
        assert_eq!(
            (at.state, at.step.as_deref(), at.started, at.ended, at.duration),
            (Status::Running, Some("transform"), 1_794_985_799_314, None, 300_686)
        );
        assert_eq!(
            steps(&at),
            [
                "fetch succeeded 1794985799314 1794986028764 None Some(229450)",
                "transform running 1794986028764 1794986344805 Some(0.22540113466290765) Some(71236)",
                "validate queued 1794986344805 1794986673389 None None",
                "upload queued 1794986673389 1794987010121 None None",
                "notify queued 1794987010121 1794987240056 None None",
            ]
        );
        assert_eq!(said(&run_log(&run, &at, mid)), MANUAL_LOG[..7]);
        assert_eq!(outputs_of(&run, &at), None);
        assert_eq!(settle(run, mid).state, Status::Running);
    }

    #[test]
    fn a_cancelled_run_says_what_the_web_says() {
        let run = Run {
            cancelled_at: Some(1_794_986_200_000),
            cancelled_by: Some(Person::Grace),
            ..manual()
        };
        let at = run_at(&run, NOW);
        assert_eq!(
            (at.state, at.step.as_deref(), at.started, at.ended, at.duration),
            (
                Status::Cancelled,
                Some("transform"),
                1_794_985_799_314,
                Some(1_794_986_200_000),
                400_686
            )
        );
        assert_eq!(
            steps(&at)[1],
            "transform cancelled 1794986028764 1794986344805 Some(0.5418157770668995) Some(171236)"
        );
        assert_eq!(steps(&at)[2], "validate skipped 1794986344805 1794986673389 None None");
        assert_eq!(
            said(&run_log(&run, &at, NOW)),
            [
                "07:09:57.368 info run queued by ada",
                "07:09:59.314 info run picked up by worker-2",
                "07:09:59.314 debug fetch started on http",
                "07:11:15.797 info fetch GET https://api.example.org/harbor/rebuild-index",
                "07:12:32.280 info fetch read 171,566 rows",
                "07:13:48.764 info fetch done in 3m 49s",
                "07:13:48.764 debug transform started on python",
                "07:15:34.111 info transform joined and reshaped 171,566 rows",
                "07:16:40.000 warning transform cancelled by grace",
                "07:16:40.000 warning validate skipped",
                "07:16:40.000 warning upload skipped",
                "07:16:40.000 warning notify skipped",
                "07:16:40.002 warning run cancelled after 6m 41s",
                "07:17:19.458 info transform wrote 4 columns: sku, amount, placed, region",
            ]
        );
        assert_eq!(
            outputs_of(&run, &at).unwrap().to_string(),
            r#"{"cancelled":{"by":"grace","at":"2026-11-18T07:16:40.000Z"}}"#
        );
    }
}
