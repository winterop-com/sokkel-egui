//! A report's figures as CSV, panel by panel, as the web's `csv.ts` writes them.

use crate::days::{day_of, iso_day};
use crate::metrics::{Figures, SIGNUP_FUNNEL, Segment};
use crate::model::{Currency, PanelId, Plan, REGIONS};
use crate::view::{Range, Span, range_name};

/// A figure as CSV writes it: a number to two decimals at most, as the web rounds it.
fn number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// A cell as CSV writes it: quoted when it holds a comma, a quote or a line break.
fn cell(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

fn line(cells: &[String]) -> String {
    cells.iter().map(|c| cell(c)).collect::<Vec<_>>().join(",")
}

/// What a report is read over.
pub struct ReportContext<'a> {
    pub figures: &'a Figures<'a>,
    pub segment: Segment,
    pub span: &'a Span,
    pub previous: &'a Span,
    pub compare: bool,
    pub range: Range,
    pub currency: Currency,
}

fn table_of(id: PanelId, c: &ReportContext<'_>) -> (Vec<String>, Vec<Vec<String>>) {
    let f = c.figures;
    let rate = c.currency.rate();
    let cash = |dollars: f64| number(dollars * rate);
    let s = |text: &str| text.to_owned();
    match id {
        PanelId::Kpis => {
            let now = f.kpis(c.segment, c.span);
            let then = f.kpis(c.segment, c.previous);
            let rows = vec![
                vec![s("Revenue"), cash(now.revenue as f64), cash(then.revenue as f64)],
                vec![s("MRR"), cash(now.mrr as f64), cash(then.mrr as f64)],
                vec![s("Active teams"), number(now.active as f64), number(then.active as f64)],
                vec![s("Sign-ups"), number(now.signups as f64), number(then.signups as f64)],
                vec![s("Conversion (%)"), number(now.conversion), number(then.conversion)],
                vec![s("Churn (%)"), number(now.churn), number(then.churn)],
            ];
            if c.compare {
                (vec![s("Figure"), s("Value"), s("Period before")], rows)
            } else {
                (
                    vec![s("Figure"), s("Value")],
                    rows.into_iter().map(|r| r[..2].to_vec()).collect(),
                )
            }
        }
        PanelId::Revenue => {
            let (team, business) = f.revenue_by_plan(c.segment, c.span);
            let then = f.revenue_series(c.segment, c.previous);
            let mut head = vec![s("Step"), s(Plan::Team.name()), s(Plan::Business.name()), s("Total")];
            if c.compare {
                head.push(s("Period before"));
            }
            let rows = c
                .span
                .buckets
                .iter()
                .enumerate()
                .map(|(i, bucket)| {
                    let t = team.get(i).copied().flatten().unwrap_or(0) as f64;
                    let b = business.get(i).copied().flatten().unwrap_or(0) as f64;
                    let mut row = vec![bucket.label.clone(), cash(t), cash(b), cash(t + b)];
                    if c.compare {
                        row.push(cash(then.get(i).copied().flatten().unwrap_or(0) as f64));
                    }
                    row
                })
                .collect();
            (head, rows)
        }
        PanelId::Funnel => {
            let stages = f.funnel(c.segment, c.span.from, c.span.to, &SIGNUP_FUNNEL, 90);
            (
                vec![
                    s("Step"),
                    s("Reached"),
                    s("Of the first (%)"),
                    s("Of the step before (%)"),
                ],
                stages
                    .iter()
                    .map(|st| {
                        vec![
                            s(st.name),
                            number(st.count as f64),
                            number(st.of_first),
                            number(st.of_previous),
                        ]
                    })
                    .collect(),
            )
        }
        PanelId::Countries => {
            let mut list = f.countries(c.segment, c.span, c.previous);
            list.sort_by_key(|one| std::cmp::Reverse(one.revenue));
            let mut head = vec![s("Country"), s("Region"), s("Revenue")];
            if c.compare {
                head.push(s("Period before"));
            }
            let rows = list
                .iter()
                .map(|one| {
                    let mut row = vec![s(one.name), s(one.region.name()), cash(one.revenue as f64)];
                    if c.compare {
                        row.push(cash(one.before as f64));
                    }
                    row
                })
                .collect();
            (head, rows)
        }
        PanelId::Customers => (
            vec![
                s("Team"),
                s("Country"),
                s("Plan"),
                s("Seats"),
                s("MRR"),
                s("MRR at the start"),
            ],
            f.top_customers(c.segment, c.span, 25)
                .iter()
                .map(|row| {
                    let team = &f.world.teams[row.team];
                    vec![
                        team.name.clone(),
                        team.country.clone(),
                        s(row.plan.name()),
                        number(row.seats as f64),
                        cash(row.mrr as f64),
                        cash(row.before as f64),
                    ]
                })
                .collect(),
        ),
        PanelId::Movements => {
            let start = f.levels(c.segment, &[c.span.from]).mrr[0] as f64;
            let flow = f.flows(c.segment, c.span.from, c.span.to);
            let net = (flow.new_mrr + flow.expansion - flow.contraction - flow.churn_mrr) as f64;
            (
                vec![s("Movement"), s("MRR")],
                vec![
                    vec![s("At the start"), cash(start)],
                    vec![s("New"), cash(flow.new_mrr as f64)],
                    vec![s("Expansion"), cash(flow.expansion as f64)],
                    vec![s("Contraction"), cash(-(flow.contraction as f64))],
                    vec![s("Churn"), cash(-(flow.churn_mrr as f64))],
                    vec![s("Net"), cash(net)],
                    vec![s("At the end"), cash(start + net)],
                ],
            )
        }
        PanelId::Regions => {
            let now = f.revenue_by_region(c.segment, c.span.from, c.span.to);
            let then = f.revenue_by_region(c.segment, c.previous.from, c.previous.to);
            let mut head = vec![s("Region"), s("Revenue")];
            if c.compare {
                head.push(s("Period before"));
            }
            let rows = REGIONS
                .iter()
                .filter(|r| c.segment.region.is_none_or(|one| one == **r))
                .map(|r| {
                    let mut row = vec![s(r.name()), cash(now[r] as f64)];
                    if c.compare {
                        row.push(cash(then[r] as f64));
                    }
                    row
                })
                .collect();
            (head, rows)
        }
        PanelId::Cohorts => {
            let grid = f.cohorts(c.segment, c.span.last);
            let mut head = vec![s("Cohort")];
            head.extend(grid.months.iter().map(|m| format!("{m} (%)")));
            let rows = grid
                .order
                .iter()
                .map(|cohort| {
                    let mut row = vec![cohort.clone()];
                    row.extend(grid.months.iter().map(|month| {
                        grid.cells
                            .iter()
                            .find(|one| one.cohort == *cohort && one.month == *month)
                            .map_or(String::new(), |one| number(one.retained))
                    }));
                    row
                })
                .collect();
            (head, rows)
        }
    }
}

/// A report as one CSV file: what it covers, then each panel's figures under its name.
pub fn report_csv(name: &str, panels: &[PanelId], c: &ReportContext<'_>) -> String {
    let s = |text: &str| text.to_owned();
    let mut lines = vec![
        line(&[s(name)]),
        line(&[
            s("Range"),
            format!(
                "{}, {} to {}",
                range_name(&c.range),
                iso_day(c.span.first),
                iso_day(c.span.last)
            ),
        ]),
        line(&[s("Plan"), s(c.segment.plan.map_or("All plans", Plan::name))]),
        line(&[s("Region"), s(c.segment.region.map_or("Everywhere", |r| r.name()))]),
        line(&[s("Currency"), s(c.currency.code())]),
        line(&[s("Made"), iso_moment(c.figures.world.now)]),
    ];
    for id in panels {
        let (head, rows) = table_of(*id, c);
        lines.push(String::new());
        lines.push(line(&[s(id.name())]));
        lines.push(line(&head));
        lines.extend(rows.iter().map(|r| line(r)));
    }
    format!("{}\n", lines.join("\n"))
}

/// A moment as `toISOString` writes it: `2026-11-18T09:00:05.000Z`.
fn iso_moment(at: i64) -> String {
    let day = at.div_euclid(crate::days::DAY);
    let ms = at.rem_euclid(crate::days::DAY);
    format!(
        "{}T{:02}:{:02}:{:02}.{:03}Z",
        iso_day(day),
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// A file name from a report's name and the day: `monthly-board-report-2026-11-09.csv`.
pub fn file_name(name: &str, now: i64) -> String {
    let mut slug = String::new();
    for ch in name.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "report" } else { slug };
    format!("{slug}-{}.csv", iso_day(day_of(now)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_file_is_named_as_the_web_names_it() {
        assert_eq!(
            super::file_name("Monthly board report", 1_794_992_400_000),
            "monthly-board-report-2026-11-18.csv"
        );
        assert_eq!(super::file_name("  ", 1_794_992_400_000), "report-2026-11-18.csv");
        assert_eq!(super::cell("a, b"), "\"a, b\"");
        assert_eq!(super::number(11.866501854140916), "11.87");
    }
}
