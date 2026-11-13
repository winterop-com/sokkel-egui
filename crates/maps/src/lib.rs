//! sokkel-ui-egui's maps: one engine, walkers, as sokkel-ui draws every map with one engine. A
//! [`MapView`] keeps where the map looks and its tiles; [`MapView::show`] draws it at a size, with
//! markers and routes over it in the chart colors, its frame in the tokens' rule and corner, and
//! zoom buttons. Without tiles (tests, or offline) it draws the `canvas` ground with a graticule,
//! so the markers and routes still sit somewhere.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText, pos2, vec2};
use sokkel_ui_egui::{Icon, TextRole, Tokens, icons, tokens};
use walkers::{HttpOptions, HttpTiles, Map, MapMemory, Position, Projector, Tiles, lat_lon, sources::OpenStreetMap};

/// A point on the map with a label, in a chart color (0 to 4 for `chart-1` to `chart-5`).
#[derive(Clone, Debug)]
pub struct Marker {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub series: usize,
}

impl Marker {
    pub fn new(lat: f64, lon: f64, label: impl Into<String>) -> Self {
        Self {
            lat,
            lon,
            label: label.into(),
            series: 0,
        }
    }

    pub fn series(mut self, series: usize) -> Self {
        self.series = series;
        self
    }
}

/// A line through points, in a chart color.
#[derive(Clone, Debug)]
pub struct Route {
    pub points: Vec<(f64, f64)>,
    pub series: usize,
}

impl Route {
    pub fn new(points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self {
            points: points.into_iter().collect(),
            series: 0,
        }
    }

    pub fn series(mut self, series: usize) -> Self {
        self.series = series;
        self
    }
}

/// Where a map looks and what it draws its ground from; keep one per map between frames.
pub struct MapView {
    memory: MapMemory,
    tiles: Option<HttpTiles>,
    home: Position,
    zoom: f64,
}

impl MapView {
    /// A map centered on a place at a zoom level (0 the world, about 12 a city), with no tiles.
    pub fn new(lat: f64, lon: f64, zoom: f64) -> Self {
        let mut memory = MapMemory::default();
        let _ = memory.set_zoom(zoom);
        Self {
            memory,
            tiles: None,
            home: lat_lon(lat, lon),
            zoom,
        }
    }

    /// Draw OpenStreetMap's tiles under the map, as their usage policy asks: an identifying
    /// `user_agent` (natively; a browser sends its own), and `cache`, a folder that keeps tiles
    /// until they expire.
    pub fn with_tiles(
        mut self,
        ctx: &egui::Context,
        user_agent: &'static str,
        cache: Option<std::path::PathBuf>,
    ) -> Self {
        let options = HttpOptions {
            cache,
            user_agent: Some(walkers::HeaderValue::from_static(user_agent)),
            ..HttpOptions::default()
        };
        self.tiles = Some(HttpTiles::with_options(OpenStreetMap, options, ctx.clone()));
        self
    }

    /// Look at the place and zoom the map was made with again.
    pub fn reset(&mut self) {
        self.memory = MapMemory::default();
        let _ = self.memory.set_zoom(self.zoom);
    }

    /// Draw the map at `size`, with `routes` and then `markers` over it. Drag to pan, scroll or
    /// pinch to zoom, double-click the buttons' minus to step back out.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, markers: &[Marker], routes: &[Route]) -> Response {
        let t = tokens(ui.ctx());
        let has_tiles = self.tiles.is_some();
        let tiles = self.tiles.as_mut().map(|tiles| tiles as &mut dyn Tiles);
        let response = ui
            .allocate_ui(size, |ui| {
                ui.set_min_size(size);
                Map::new(tiles, &mut self.memory, self.home)
                    .zoom_with_ctrl(false)
                    .show(ui, |ui, response, projector, _| {
                        let painter = ui.painter_at(response.rect);
                        if !has_tiles {
                            ground(&painter, &t, response.rect, projector);
                        }
                        overlays(&painter, &t, projector, markers, routes);
                        if has_tiles {
                            attribution(ui, &painter, &t, response.rect);
                        }
                    })
                    .response
            })
            .inner;
        let rect = response.rect;
        ui.painter().rect_stroke(
            rect,
            t.radius("surface"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        self.zoom_buttons(ui, &t, rect);
        response
    }

    fn zoom_buttons(&mut self, ui: &mut Ui, t: &Tokens, rect: Rect) {
        let size = 28.0;
        for (i, (icon, label, step)) in [(Icon::Plus, "Zoom in", 1.0), (Icon::Minus, "Zoom out", -1.0)]
            .into_iter()
            .enumerate()
        {
            let button = Rect::from_min_size(
                pos2(rect.right() - size - 10.0, rect.top() + 10.0 + i as f32 * size),
                vec2(size, size),
            );
            let response = ui.interact(button, ui.id().with(("map-zoom", i)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
            let painter = ui.painter();
            let corners = if i == 0 {
                CornerRadius {
                    sw: 0,
                    se: 0,
                    ..t.radius("control")
                }
            } else {
                CornerRadius {
                    nw: 0,
                    ne: 0,
                    ..t.radius("control")
                }
            };
            let fill = if response.hovered() {
                t.color("muted")
            } else {
                t.color("popover")
            };
            painter.rect(
                button,
                corners,
                fill,
                Stroke::new(1.0, t.color("border")),
                StrokeKind::Inside,
            );
            icons::paint(painter, button.center(), icon, 16.0, t.color("foreground"));
            if response.on_hover_text(label).clicked() {
                let _ = self.memory.set_zoom((self.memory.zoom() + step).clamp(2.0, 18.0));
            }
        }
    }
}

/// The ground without tiles: the canvas color and a graticule every degree.
fn ground(painter: &egui::Painter, t: &Tokens, rect: Rect, projector: &Projector) {
    painter.rect_filled(rect, CornerRadius::ZERO, t.color("canvas"));
    let center = projector.unproject(rect.center().to_vec2());
    let (lat0, lon0) = (center.y().round(), center.x().round());
    let stroke = Stroke::new(1.0, t.color("border"));
    // parallels a degree apart, then meridians, each far enough to cross any view
    for d in -60..=60 {
        let lat = lat0 + d as f64;
        if lat.abs() <= 85.0 {
            let a = projector.project(lat_lon(lat, lon0 - 90.0)).to_pos2();
            let b = projector.project(lat_lon(lat, lon0 + 90.0)).to_pos2();
            painter.line_segment([a, b], stroke);
        }
        let lon = lon0 + d as f64;
        let a = projector.project(lat_lon((lat0 - 80.0).max(-85.0), lon)).to_pos2();
        let b = projector.project(lat_lon((lat0 + 80.0).min(85.0), lon)).to_pos2();
        painter.line_segment([a, b], stroke);
    }
}

fn series_color(t: &Tokens, series: usize) -> Color32 {
    t.color(&format!("chart-{}", series % 5 + 1))
}

/// Routes, cased in the ground so they read over any tile, then markers with their labels.
fn overlays(painter: &egui::Painter, t: &Tokens, projector: &Projector, markers: &[Marker], routes: &[Route]) {
    let at = |lat: f64, lon: f64| -> Pos2 { projector.project(lat_lon(lat, lon)).to_pos2() };
    for route in routes {
        let points: Vec<Pos2> = route.points.iter().map(|(lat, lon)| at(*lat, *lon)).collect();
        painter.add(egui::Shape::line(
            points.clone(),
            Stroke::new(5.0, t.color("background")),
        ));
        painter.add(egui::Shape::line(
            points,
            Stroke::new(3.0, series_color(t, route.series)),
        ));
    }
    let font = TextRole::Help.font(t);
    for marker in markers {
        let point = at(marker.lat, marker.lon);
        painter.circle(
            point,
            6.0,
            series_color(t, marker.series),
            Stroke::new(2.0, t.color("background")),
        );
        let galley = painter.layout_no_wrap(marker.label.clone(), font.clone(), t.color("popover-foreground"));
        let pill = Rect::from_min_size(pos2(point.x + 10.0, point.y - 10.0), galley.size() + vec2(12.0, 4.0));
        painter.rect(
            pill,
            t.radius("chip"),
            t.color("popover"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        painter.galley(pill.min + vec2(6.0, 2.0), galley, t.color("popover-foreground"));
    }
}

/// OpenStreetMap's credit, which its tiles ask to be shown on the map.
fn attribution(ui: &Ui, painter: &egui::Painter, t: &Tokens, rect: Rect) {
    let text = WidgetText::from(TextRole::Help.rich(t, "OpenStreetMap contributors")).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        TextRole::Help.font(t),
    );
    let pill = Rect::from_min_size(
        rect.right_bottom() - text.size() - vec2(16.0, 10.0),
        text.size() + vec2(10.0, 4.0),
    );
    painter.rect_filled(pill, t.radius("chip"), t.color("popover").gamma_multiply(0.9));
    painter.galley(pill.min + vec2(5.0, 2.0), text, t.color("muted-foreground"));
}
