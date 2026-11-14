//! sokkel-ui-egui's maps: one engine, walkers, as sokkel-ui draws every map with one engine. A
//! [`MapView`] keeps where the map looks and its tiles; [`MapView::show`] draws it at a size, with
//! markers and routes over it in the chart colors, its frame in the tokens' rule and corner, and
//! zoom buttons. Its tiles are OpenFreeMap's vector street map, as sokkel-ui's default basemap,
//! drawn light or dark by the scheme over the ground in `card`, so a tile not yet loaded shows the
//! ground. Without tiles (tests, or offline) it draws the ground with a graticule, so the markers
//! and routes still sit somewhere.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText, pos2, vec2};
use sokkel_ui_egui::{Icon, TextRole, Tokens, icons, tokens};
use walkers::{
    HttpOptions, HttpTiles, Layer, Map, MapMemory, Position, Projector, Style, Tiles, lat_lon, sources::OpenFreeMap,
};

/// Gets the maps ready to draw their street map's land, water and roads, which are drawn on the
/// GPU: call it once at startup with eframe's wgpu render state,
/// `sokkel_ui_egui_maps::install(cc.wgpu_render_state.as_ref())`. Without it a map draws only the
/// street map's names.
pub use walkers::install_renderer as install;

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
    /// Where the street map comes from, once asked for.
    source: Option<Source>,
    /// The street map, styled for the colors in `styled`.
    tiles: Option<HttpTiles>,
    styled: Option<[Color32; 2]>,
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
            source: None,
            tiles: None,
            styled: None,
            home: lat_lon(lat, lon),
            zoom,
        }
    }

    /// Draw OpenFreeMap's vector street map under the map, light or dark by the scheme, as
    /// sokkel-ui's default basemap: with an identifying `user_agent` (natively; a browser sends its
    /// own), and `cache`, a folder that keeps tiles until they expire.
    pub fn with_tiles(
        mut self,
        ctx: &egui::Context,
        user_agent: &'static str,
        cache: Option<std::path::PathBuf>,
    ) -> Self {
        self.source = Some(Source {
            ctx: ctx.clone(),
            user_agent,
            cache,
        });
        self
    }

    /// The street map styled for the tokens in use, made again when their ground or ink changes.
    fn tiles_for(&mut self, t: &Tokens, dark: bool) {
        let Some(source) = self.source.as_ref() else {
            return;
        };
        let key = [t.color("card"), t.color("foreground")];
        if self.styled != Some(key) || self.tiles.is_none() {
            let options = HttpOptions {
                cache: source.cache.clone(),
                user_agent: Some(walkers::HeaderValue::from_static(source.user_agent)),
                ..HttpOptions::default()
            };
            self.tiles = Some(HttpTiles::with_options_and_style(
                OpenFreeMap,
                options,
                street_style(t, dark),
                source.ctx.clone(),
            ));
            self.styled = Some(key);
        }
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
        let has_tiles = self.source.is_some();
        let dark = ui.visuals().dark_mode;
        let home = self.home;
        self.tiles_for(&t, dark);
        let tiles = self.tiles.as_mut().map(|tiles| tiles as &mut dyn Tiles);
        let response = ui
            .allocate_ui(size, |ui| {
                ui.set_min_size(size);
                // the ground in `card`, under the tiles, showing where one has not loaded
                let room = Rect::from_min_size(ui.cursor().min, size);
                ui.painter().rect_filled(room, CornerRadius::ZERO, t.color("card"));
                Map::new(tiles, &mut self.memory, home)
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

/// Where a map's street map comes from: the host's identifying user agent and a tile cache.
struct Source {
    ctx: egui::Context,
    user_agent: &'static str,
    cache: Option<std::path::PathBuf>,
}

/// `share` of `a` over `b`, as `color-mix` mixes two opaque colors.
fn mix(a: Color32, b: Color32, share: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * share + y as f32 * (1.0 - share)).round() as u8;
    Color32::from_rgb(channel(a.r(), b.r()), channel(a.g(), b.g()), channel(a.b(), b.b()))
}

/// The street map in the tokens' colors, a pale map after the web's light and dark street maps:
/// no background of its own, so the ground (`card`) is the land; water, land use, buildings, roads
/// and borders in the foreground at small shares over it; place names in the web's ink (the
/// foreground at 80%) over a halo in the ground, the rest of the names muted; and every name in
/// its Latin form where the tiles have one, so a name in a script the fonts lack still reads.
fn street_style(t: &Tokens, dark: bool) -> Style {
    let mut style = if dark {
        Style::openmaptiles_basemap_dark()
    } else {
        Style::openmaptiles_basemap_light()
    };
    let ground = t.color("card");
    let ink = t.color("foreground");
    let tone = |share: f32| walkers::Color(walkers::json!(sokkel_ui_egui::hex(mix(ink, ground, share))));
    let source = |layer: &walkers::SourceLayer| layer.to_string();
    style.layers.retain(|layer| !matches!(layer, Layer::Background { .. }));
    for layer in &mut style.layers {
        match layer {
            Layer::Fill {
                source_layer, paint, ..
            } => {
                let from = source(source_layer);
                let share = if from.contains("water") {
                    0.12
                } else if from.contains("building") {
                    0.07
                } else {
                    0.04
                };
                paint.fill_color = Some(tone(share));
            }
            Layer::Line {
                source_layer, paint, ..
            } => {
                let from = source(source_layer);
                let share = if from.contains("boundary") {
                    0.28
                } else if from.contains("water") {
                    0.12
                } else {
                    0.1
                };
                paint.line_color = Some(tone(share));
            }
            Layer::Symbol {
                source_layer,
                layout,
                paint,
                ..
            } => {
                let place = source(source_layer).contains("place");
                if layout.text_field == Some(walkers::json!(["get", "name"])) {
                    layout.text_field = Some(walkers::json!(["coalesce", ["get", "name:latin"], ["get", "name"]]));
                }
                let paint = paint.get_or_insert_with(Default::default);
                paint.text_color = Some(if place {
                    tone(0.8)
                } else {
                    walkers::Color(walkers::json!(sokkel_ui_egui::hex(t.color("muted-foreground"))))
                });
                paint.text_halo_color = Some(walkers::Color(walkers::json!(sokkel_ui_egui::hex(ground))));
            }
            _ => {}
        }
    }
    style
}

/// The ground without tiles: `card`, and a graticule every degree in the foreground at 14% over it.
fn ground(painter: &egui::Painter, t: &Tokens, rect: Rect, projector: &Projector) {
    painter.rect_filled(rect, CornerRadius::ZERO, t.color("card"));
    let center = projector.unproject(rect.center().to_vec2());
    let (lat0, lon0) = (center.y().round(), center.x().round());
    let stroke = Stroke::new(1.0, t.color_at("foreground", 0.14));
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

/// The credits the street map's hosts ask to be shown on the map, as the web's.
fn attribution(ui: &Ui, painter: &egui::Painter, t: &Tokens, rect: Rect) {
    let credits = "OpenFreeMap © OpenMapTiles © OpenStreetMap contributors";
    let text = WidgetText::from(TextRole::Help.rich(t, credits)).into_galley(
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
