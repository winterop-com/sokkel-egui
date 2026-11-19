//! A choropleth's color scale, as sokkel-ui-maps' `sequentialScale`: numbers sorted into classes
//! by ascending breaks, each class a color, with the breaks written for a legend.

use std::sync::Arc;

use egui::Color32;
use sokkel_egui::Tokens;

use crate::sequential_colors;

/// How a scale writes a break: grouped, at most one decimal by default, as the web's.
pub type ScaleFormat = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// The words a scale's classes are written in, English by default.
#[derive(Clone, Debug)]
pub struct ClassWords {
    /// The one class of a scale with no breaks.
    pub all: String,
    /// Before the first break: `Below 200`.
    pub below: String,
    /// After the last break: `380 and above`.
    pub above: String,
}

impl Default for ClassWords {
    fn default() -> Self {
        Self {
            all: "All values".into(),
            below: "Below".into(),
            above: "and above".into(),
        }
    }
}

/// Numbers sorted into classes by ascending `breaks`: a value below the first takes the first
/// color, a value at or above the last the last, so there is one more color than breaks.
#[derive(Clone)]
pub struct SequentialScale {
    pub breaks: Vec<f64>,
    pub colors: Vec<Color32>,
    pub format: ScaleFormat,
}

impl SequentialScale {
    /// A scale over `breaks` ramped in one `hue` (a color token, `chart-2` in the web's default),
    /// as the web's `sequentialScale({ breaks, hue })`.
    pub fn new(t: &Tokens, breaks: Vec<f64>, hue: &str) -> Self {
        let colors = sequential_colors(t, hue, breaks.len() + 1);
        Self {
            breaks,
            colors,
            format: Arc::new(default_format),
        }
    }

    /// The classes in these colors, one per class, in place of the ramp, such as a threshold
    /// scale's good, warning and critical; `None` where there is not one more color than breaks.
    pub fn with_colors(breaks: Vec<f64>, colors: Vec<Color32>) -> Option<Self> {
        (colors.len() == breaks.len() + 1).then(|| Self {
            breaks,
            colors,
            format: Arc::new(default_format),
        })
    }

    /// How the breaks are written, in a legend and its classes.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Arc::new(format);
        self
    }

    /// The class a value falls in, or none for no data.
    pub fn class_of(&self, value: Option<f64>) -> Option<usize> {
        let value = value.filter(|v| !v.is_nan())?;
        Some(self.breaks.iter().take_while(|b| value >= **b).count())
    }

    /// The color a value is drawn in, or none for no data.
    pub fn color_of(&self, value: Option<f64>) -> Option<Color32> {
        self.class_of(value).and_then(|class| self.colors.get(class).copied())
    }

    /// How many of the values fall in each class, in class order, and how many have no data.
    pub fn class_counts(&self, values: &[Option<f64>]) -> (Vec<usize>, usize) {
        let mut counts = vec![0; self.colors.len()];
        let mut none = 0;
        for value in values {
            match self.class_of(*value) {
                Some(class) => counts[class] += 1,
                None => none += 1,
            }
        }
        (counts, none)
    }

    /// Each class written out: `Below 200`, `200 – 260`, `380 and above`.
    pub fn class_labels(&self, words: &ClassWords) -> Vec<String> {
        let f = &self.format;
        let Some((first, last)) = self.breaks.first().zip(self.breaks.last()) else {
            return vec![words.all.clone()];
        };
        let mut labels = vec![format!("{} {}", words.below, f(*first))];
        labels.extend(
            self.breaks
                .windows(2)
                .map(|pair| format!("{} \u{2013} {}", f(pair[0]), f(pair[1]))),
        );
        labels.push(format!("{} {}", f(*last), words.above));
        labels
    }

    /// Each break written by the scale's format, as a ramp's legend writes them under the class
    /// edges.
    pub fn break_labels(&self) -> Vec<String> {
        self.breaks.iter().map(|b| (self.format)(*b)).collect()
    }
}

/// A figure grouped by thousands with at most one decimal, as `Intl.NumberFormat('en-US', {
/// maximumFractionDigits: 1 })`.
fn default_format(value: f64) -> String {
    let written = sokkel_egui::format_value(value, Some(1));
    written.strip_suffix(".0").map_or(written.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale() -> SequentialScale {
        SequentialScale::with_colors(
            vec![200.0, 260.0, 380.0],
            vec![Color32::RED, Color32::GREEN, Color32::BLUE, Color32::BLACK],
        )
        .expect("four colors for three breaks")
    }

    #[test]
    fn values_fall_in_the_class_their_breaks_reach() {
        let scale = scale();
        assert_eq!(scale.class_of(Some(10.0)), Some(0));
        assert_eq!(scale.class_of(Some(200.0)), Some(1));
        assert_eq!(scale.class_of(Some(379.0)), Some(2));
        assert_eq!(scale.class_of(Some(1000.0)), Some(3));
        assert_eq!(scale.class_of(None), None);
        assert_eq!(scale.class_of(Some(f64::NAN)), None);
        assert_eq!(scale.color_of(Some(300.0)), Some(Color32::BLUE));
        assert_eq!(
            scale.class_counts(&[Some(1.0), None, Some(500.0), Some(600.0)]),
            (vec![1, 0, 0, 2], 1)
        );
        assert!(SequentialScale::with_colors(vec![1.0], vec![Color32::RED]).is_none());
    }

    #[test]
    fn classes_are_written_out() {
        let scale = scale();
        assert_eq!(
            scale.class_labels(&ClassWords::default()),
            ["Below 200", "200 \u{2013} 260", "260 \u{2013} 380", "380 and above"]
        );
        let none = SequentialScale::with_colors(Vec::new(), vec![Color32::RED]).expect("one color");
        assert_eq!(none.class_labels(&ClassWords::default()), ["All values"]);
        let short = scale.format(|v| format!("{}K", v / 1000.0));
        assert_eq!(short.break_labels(), ["0.2K", "0.26K", "0.38K"]);
        assert_eq!(default_format(2345.67), "2,345.7");
        assert_eq!(default_format(2300.0), "2,300");
    }
}
