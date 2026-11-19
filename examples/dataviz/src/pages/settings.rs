//! The visualizer's preferences, as the web's `pages/Settings.tsx` words them: chart colors, how
//! figures are written, new charts, and the demo's data.

use crate::model::{Dataset, DatasetSource, Decimals, NumberStyle, Preferences, counted};
use crate::shape::format_figure;

/// The number formats offered.
pub const NUMBER_STYLES: [NumberStyle; 3] = NumberStyle::ALL;

/// The decimals offered.
pub const DECIMALS: [Decimals; 4] = Decimals::ALL;

/// The Number format setting's description, with a sample written as the settings write it.
pub fn style_description(prefs: &Preferences) -> String {
    format!(
        "How figures are written in charts, tables and tiles: 1234567.891 is written {}.",
        format_figure(prefs.numbers, prefs.decimals, 1_234_567.891)
    )
}

/// The Reset setting's description, naming the datasets imported that go.
pub fn reset_description(datasets: &[Dataset]) -> String {
    let imported = datasets.iter().filter(|one| one.source != DatasetSource::Seed).count();
    format!(
        "Puts the datasets, charts, boards and these preferences back as they first were{}.",
        if imported > 0 {
            format!("; the {} imported here go", counted(imported, "dataset"))
        } else {
            String::new()
        }
    )
}
