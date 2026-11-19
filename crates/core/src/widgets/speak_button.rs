//! A button that reads a text aloud, after sokkel-ui's `media/SpeakButton`.

use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Ui, Widget, WidgetInfo, vec2};

use super::{Button, ButtonVariant};
use crate::{Icon, icons, tokens};

/// Where a read-aloud stands: waiting, fetching its audio, playing it, or failed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeakState {
    #[default]
    Idle,
    Loading,
    Playing,
    Error,
}

/// What a press does: fetch the audio and play it, play the audio kept, stop it playing, or stop
/// the fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakPress {
    Load,
    Play,
    Stop,
    Cancel,
}

/// What a press does in `state`, with the audio fetched before (`kept`) or not; after a failure it
/// fetches again.
pub fn speak_press(state: SpeakState, kept: bool) -> SpeakPress {
    match state {
        SpeakState::Loading => SpeakPress::Cancel,
        SpeakState::Playing => SpeakPress::Stop,
        SpeakState::Error => SpeakPress::Load,
        SpeakState::Idle if kept => SpeakPress::Play,
        SpeakState::Idle => SpeakPress::Load,
    }
}

/// The words the button says.
#[derive(Clone, Debug)]
pub struct SpeakButtonWords {
    /// The button's name while it waits, and its text when labeled.
    pub speak: String,
    /// Its name while the audio is fetched; a press stops the fetch.
    pub loading: String,
    /// Its name while the audio plays, and its text when labeled; a press stops it.
    pub stop: String,
    /// Said on hover once the audio could not be fetched or played.
    pub failed: String,
    /// Its text after a failure, when labeled; a press fetches the audio again.
    pub retry: String,
}

impl Default for SpeakButtonWords {
    fn default() -> Self {
        Self {
            speak: "Read aloud".into(),
            loading: "Loading the audio".into(),
            stop: "Stop".into(),
            failed: "The audio could not be played".into(),
            retry: "Try again".into(),
        }
    }
}

/// Lucide's square filled, as `fill-current` fills it: 18 of 24 on corners of 2.
pub fn paint_square(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let s = size / 24.0;
    let rect = Rect::from_center_size(center, vec2(18.0 * s, 18.0 * s));
    painter.rect_filled(rect, CornerRadius::same((2.0 * s).round() as u8), color);
}

/// A button that reads a text aloud, after sokkel-ui's `SpeakButton`: a ghost icon button (an
/// `IconButton`, for a hover bar) or, `labeled`, a ghost button with its words (for a row of
/// actions). Its glyph says where it stands: a speaker waiting, the spinner while the audio is
/// fetched, a filled square while it plays, and a circled mark in `critical-ink` once it failed,
/// the hover then saying so.
///
/// It is controlled: the app keeps its `state` and the audio, and a press answers what to do with
/// [`speak_press`]: fetch and play, play what is kept, stop it, or stop the fetch.
#[must_use = "add it with `ui.add(...)`"]
pub struct SpeakButton {
    state: SpeakState,
    labeled: bool,
    words: SpeakButtonWords,
}

impl SpeakButton {
    pub fn new(state: SpeakState) -> Self {
        Self {
            state,
            labeled: false,
            words: SpeakButtonWords::default(),
        }
    }

    /// With its words beside the glyph, as the web's `variant="labeled"`.
    pub fn labeled(mut self, labeled: bool) -> Self {
        self.labeled = labeled;
        self
    }

    pub fn words(mut self, words: SpeakButtonWords) -> Self {
        self.words = words;
        self
    }
}

impl Widget for SpeakButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let words = &self.words;
        let name = match self.state {
            SpeakState::Idle => &words.speak,
            SpeakState::Loading => &words.loading,
            SpeakState::Playing => &words.stop,
            SpeakState::Error => &words.failed,
        };
        if self.labeled {
            let text = match self.state {
                SpeakState::Idle | SpeakState::Loading => &words.speak,
                SpeakState::Playing => &words.stop,
                SpeakState::Error => &words.retry,
            };
            let mut button = Button::new(text).variant(ButtonVariant::Ghost);
            match self.state {
                SpeakState::Idle => button = button.icon(Icon::Volume2),
                SpeakState::Loading => button.spinning = true,
                SpeakState::Playing => button = button.icon(Icon::Square).filled_icon(true),
                SpeakState::Error => {
                    button = button.icon(Icon::CircleAlert);
                    button.icon_ink = Some(t.color("critical-ink"));
                }
            }
            let response = ui.add(button);
            response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, name));
            return if self.state == SpeakState::Error {
                response.on_hover_text(&words.failed)
            } else {
                response
            };
        }
        // as an icon button: `icon-sm`, the glyph muted until the pointer is on it
        let side = super::finger_tall(ui.ctx(), &t, t.density("control-sm"));
        let radius = CornerRadius::same(t.radius("control").nw.min(12));
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, name));
        let hot = response.hovered();
        if hot {
            ui.painter().rect_filled(rect, radius, t.color("hover"));
        }
        super::focus_ring(ui.painter(), &t, rect, radius, &response);
        let ink = if hot {
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        let at = rect.center();
        match self.state {
            SpeakState::Idle => icons::paint(ui.painter(), at, Icon::Volume2, 16.0, ink),
            SpeakState::Loading => super::spinner(ui, ui.painter(), at, 16.0 * 0.4, ink),
            SpeakState::Playing => paint_square(ui.painter(), at, 16.0, ink),
            SpeakState::Error => icons::paint(ui.painter(), at, Icon::CircleAlert, 16.0, t.color("critical-ink")),
        }
        response.on_hover_text(name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_does_what_the_state_says() {
        assert_eq!(speak_press(SpeakState::Idle, false), SpeakPress::Load);
        assert_eq!(speak_press(SpeakState::Idle, true), SpeakPress::Play);
        assert_eq!(speak_press(SpeakState::Loading, true), SpeakPress::Cancel);
        assert_eq!(speak_press(SpeakState::Playing, true), SpeakPress::Stop);
        assert_eq!(speak_press(SpeakState::Error, true), SpeakPress::Load);
    }
}
