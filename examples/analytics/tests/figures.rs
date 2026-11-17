//! The world and its figures are the web's to the last digit: each value here is what bun reads
//! from the web's own `seed.ts`, `world.ts` and `metrics.ts` at 2026-11-18 09:00 UTC.

use analytics::metrics::{EVERYONE, Figures, SIGNUP_FUNNEL};
use analytics::model::InvoiceStatus;
use analytics::view::{Range, span_of};
use analytics::world::{Base, Kept, build};

const NOW: i64 = 1_794_992_400_000;

#[test]
fn the_world_is_the_webs() {
    let world = build(&Base::new(NOW), &Kept::default(), NOW);
    assert_eq!(world.teams.len(), 13_665);
    assert_eq!(world.invoices.len(), 21_363);
    assert_eq!(world.latest[0].team, "t20775-2");
    assert_eq!(world.latest[79].team, "t20773-0");
    let first = &world.teams[0];
    assert_eq!(
        (
            first.id.as_str(),
            first.name.as_str(),
            first.country.as_str(),
            first.created,
            first.dormant
        ),
        (
            "t20046-0",
            "Onyx Florists",
            "United States",
            1_732_024_115_247,
            Some(20_059)
        )
    );
    let last = world.invoices.last().unwrap();
    assert_eq!(
        (
            last.id.as_str(),
            world.teams[last.team].name.as_str(),
            last.amount,
            last.at,
            last.status
        ),
        (
            "INV-261118-003",
            "Sorrelgate Ventures",
            192,
            1_794_991_090_332,
            InvoiceStatus::Paid
        )
    );
}

#[test]
fn the_figures_are_the_webs() {
    let world = build(&Base::new(NOW), &Kept::default(), NOW);
    let figures = Figures::new(&world);
    let span = span_of(&Range::Preset("30d"), NOW);
    let k = figures.kpis(EVERYONE, &span);
    assert_eq!(
        (k.revenue, k.mrr, k.active, k.paying, k.signups),
        (554_767, 577_236, 6061, 1656, 809)
    );
    assert_eq!(k.conversion, 11.866501854140916);
    assert_eq!(k.churn, 2.0100502512562812);
    assert_eq!(k.trend.revenue[..5], [13607.0, 19937.0, 21678.0, 16942.0, 12127.0]);
    let funnel = figures.funnel(EVERYONE, span.from, span.to, &SIGNUP_FUNNEL, 60);
    let counts: Vec<i64> = funnel.iter().map(|s| s.count).collect();
    assert_eq!(counts, [2663, 809, 589, 301, 14]);
    assert_eq!(funnel[2].of_previous, 72.80593325092707);
}

#[test]
fn every_figure_is_the_webs() {
    use analytics::metrics::Segment;
    use analytics::model::{Plan, Region, Step};
    use analytics::view::{Step as Cut, previous_of};
    let world = build(&Base::new(NOW), &Kept::default(), NOW);
    let figures = Figures::new(&world);
    let segment = Segment {
        plan: Some(Plan::Business),
        region: Some(Region::Europe),
    };
    for (id, buckets, cut, figures_of, churn_sum) in [
        ("today", 24, Cut::Hour, (0, 147_150, 178, 178, 2, 0.0, 0.0), 0.0),
        (
            "12m",
            53,
            Cut::Week,
            (
                1_259_417,
                147_150,
                178,
                178,
                2681,
                3.4315553897799327,
                26.31578947368421,
            ),
            14.884504227873343,
        ),
        (
            "ytd",
            46,
            Cut::Week,
            (
                1_170_943,
                147_150,
                178,
                178,
                2431,
                3.455368161250514,
                20.689655172413794,
            ),
            12.478399979237947,
        ),
    ] {
        let span = span_of(&Range::Preset(id), NOW);
        assert_eq!((span.buckets.len(), span.step), (buckets, cut), "{id}");
        let k = figures.kpis(segment, &span);
        assert_eq!(
            (k.revenue, k.mrr, k.active, k.paying, k.signups, k.conversion, k.churn),
            figures_of,
            "{id}"
        );
        assert_eq!(k.trend.churn.iter().fold(0.0, |a, b| a + b), churn_sum, "{id}");
    }
    let span = span_of(&Range::Preset("90d"), NOW);
    let previous = previous_of(&span);
    let f = figures.flows(EVERYONE, span.from, span.to);
    assert_eq!(
        (
            f.signups,
            f.converted,
            f.churned,
            f.new_mrr,
            f.expansion,
            f.contraction,
            f.churn_mrr
        ),
        (2340, 308, 88, 77_718, 45_066, 9516, 19_980)
    );
    let top: Vec<(&str, i64, i64)> = figures
        .top_customers(EVERYONE, &span, 8)
        .iter()
        .map(|t| (world.teams[t.team].name.as_str(), t.mrr, t.before))
        .collect();
    assert_eq!(top[0], ("Onyxgate Robotics", 2790, 2790));
    assert_eq!(top[6], ("Tansybrook Marine", 2430, 2130));
    assert_eq!(top[7], ("Granitemere Aviation", 2400, 2400));
    let countries = figures.countries(EVERYONE, &span, &previous);
    assert_eq!(
        (countries[1].name, countries[1].revenue, countries[1].before),
        ("Germany", 118_528, 105_501)
    );
    let cohorts = figures.cohorts(EVERYONE, span.last);
    assert_eq!(cohorts.cells.len(), 78);
    assert_eq!(
        (cohorts.cells[1].cohort.as_str(), cohorts.cells[1].retained),
        ("Dec 2025", 98.9)
    );
    assert_eq!(cohorts.cells[2].retained, 95.4);
    assert_eq!(cohorts.cells[77].cohort, "Nov 2026");
    let (team, business) = figures.revenue_by_plan(EVERYONE, &span);
    assert_eq!(team[team.len() - 3..], [Some(4875), Some(6368), Some(624)]);
    assert_eq!(business[business.len() - 3..], [Some(14_238), Some(17_785), Some(0)]);
    let by_plan = figures.mrr_by_plan(EVERYONE, NOW);
    assert_eq!((by_plan[&Plan::Team], by_plan[&Plan::Business]), (155_256, 421_980));
    assert_eq!(world.teams[13_000].name, "Basaltstone Audio");
    assert_eq!(figures.refunds(EVERYONE, span.from, span.to), (44, 18_042));
    let steps = [Step::SignedUp, Step::Upgraded, Step::Seats, Step::Business];
    let team_plan = Segment {
        plan: Some(Plan::Team),
        region: None,
    };
    let counts: Vec<i64> = figures
        .funnel(team_plan, span.from, span.to, &steps, 365)
        .iter()
        .map(|s| s.count)
        .collect();
    assert_eq!(counts, [2340, 172, 15, 0]);
    assert_eq!(span_of(&Range::Preset("ytd"), NOW).first, 20_454);
    assert_eq!(span_of(&Range::Preset("mtd"), NOW).first, 20_758);
}
