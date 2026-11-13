use eframe::egui::{Ui, vec2};
use sokkel_ui_egui_maps::{Marker, Route};

use super::{State, section};

/// Sites in the Nordic region, by plain geography.
fn sites() -> Vec<Marker> {
    vec![
        Marker::new(59.9139, 10.7522, "Oslo"),
        Marker::new(60.3913, 5.3221, "Bergen").series(1),
        Marker::new(63.4305, 10.3951, "Trondheim").series(2),
        Marker::new(59.3293, 18.0686, "Stockholm").series(3),
        Marker::new(57.7089, 11.9746, "Gothenburg").series(4),
    ]
}

pub fn map(state: &mut State, ui: &mut Ui) {
    let width = ui.available_width();
    section(ui, "Sites and the routes between them");
    let routes = [
        Route::new([(59.9139, 10.7522), (60.3913, 5.3221)]),
        Route::new([(59.9139, 10.7522), (63.4305, 10.3951)]).series(2),
        Route::new([(59.9139, 10.7522), (57.7089, 11.9746), (59.3293, 18.0686)]).series(3),
    ];
    state.map.show(ui, vec2(width, 460.0), &sites(), &routes);
}
