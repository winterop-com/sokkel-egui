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
    HttpOptions, HttpTiles, Layer, Map, MapMemory, Position, Projector, Style, TileId, Tiles, lat_lon,
    sources::{Attribution, OpenFreeMap, TileSource},
};

/// Gets the maps ready to draw their street map's land, water and roads, which are drawn on the
/// GPU: call it once at startup with eframe's wgpu render state,
/// `sokkel_ui_egui_maps::install(cc.wgpu_render_state.as_ref())`. Without it a map draws only the
/// street map's names.
pub use walkers::install_renderer as install;

/// A point on the map with a label, in a series' chart color or a color of its own (a state's).
/// An empty label draws the point alone.
#[derive(Clone, Debug)]
pub struct Marker {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub series: usize,
    pub color: Option<Color32>,
}

impl Marker {
    pub fn new(lat: f64, lon: f64, label: impl Into<String>) -> Self {
        Self {
            lat,
            lon,
            label: label.into(),
            series: 0,
            color: None,
        }
    }

    pub fn series(mut self, series: usize) -> Self {
        self.series = series;
        self
    }

    /// The point in this color rather than its series', such as its state's.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// What a map's corner says its points' colors mean, after sokkel-ui's `MapLegend`: a title, a
/// line under it, and a row per class with its dot, its words and how many points are in it.
pub struct MapLegend {
    title: String,
    description: Option<String>,
    rows: Vec<(Color32, String, usize)>,
}

impl MapLegend {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn row(mut self, color: Color32, label: impl Into<String>, count: usize) -> Self {
        self.rows.push((color, label.into(), count));
        self
    }

    /// Draws the legend in the bottom-left corner of `map`, 12 in, 224 wide.
    pub fn show(self, ui: &Ui, map: Rect) {
        let t = tokens(ui.ctx());
        let font = TextRole::Help.font(&t);
        let line = t.text("help").line_height;
        let rows = self.rows.len() as f32;
        let height = 12.0 + line + self.description.as_ref().map_or(0.0, |_| line) + 8.0 + rows * 24.0 + 8.0;
        let width = 224.0_f32.min(map.width() - 24.0);
        let card = Rect::from_min_size(
            pos2(map.left() + 12.0, map.bottom() - 12.0 - height),
            vec2(width, height),
        );
        let painter = ui.painter_at(map);
        painter.rect(
            card,
            t.radius("surface"),
            t.color("popover"),
            Stroke::new(1.0, t.color("border-strong")),
            StrokeKind::Inside,
        );
        let ink = t.color("popover-foreground");
        let muted = t.color("muted-foreground");
        let x = card.left() + 12.0;
        let mut y = card.top() + 12.0;
        let title = WidgetText::from(TextRole::Help.rich(&t, &self.title).variation("wght", 500.0)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            width - 24.0,
            font.clone(),
        );
        painter.galley(pos2(x, y + (line - title.size().y) / 2.0), title, ink);
        y += line;
        if let Some(description) = &self.description {
            painter.text(
                pos2(x, y + line / 2.0),
                egui::Align2::LEFT_CENTER,
                description,
                font.clone(),
                muted,
            );
            y += line;
        }
        y += 8.0;
        for (color, label, count) in &self.rows {
            let middle = y + 12.0;
            painter.circle_filled(pos2(x + 6.0, middle), 6.0, *color);
            painter.text(
                pos2(x + 20.0, middle),
                egui::Align2::LEFT_CENTER,
                label,
                font.clone(),
                ink,
            );
            painter.text(
                pos2(card.right() - 12.0, middle),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                font.clone(),
                muted,
            );
            y += 24.0;
        }
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
    /// Where the map was made to look.
    start: Position,
    zoom: f64,
    framed: bool,
    /// The bounds to frame once the map's size is known, as `[west, south, east, north]`.
    pending: Option<Option<[f64; 4]>>,
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
            start: lat_lon(lat, lon),
            zoom,
            framed: true,
            pending: None,
        }
    }

    /// Whether the map draws its own border and corners; off for a map that fills a pane.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Frame `bounds` (`[west, south, east, north]` in degrees) the next time the map is drawn, or,
    /// with `None`, the place and zoom it was made with.
    pub fn frame(&mut self, bounds: Option<[f64; 4]>) {
        self.pending = Some(bounds);
    }

    /// Look at `bounds` within a map `size` large: their middle, at the deepest zoom that shows them whole.
    fn fit(&mut self, bounds: [f64; 4], size: Vec2) {
        let [west, south, east, north] = bounds;
        let y = |lat: f64| {
            let r = lat.clamp(-85.0, 85.0).to_radians();
            (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0
        };
        let (top, bottom) = (y(north), y(south));
        let wide = ((east - west) / 360.0).max(1e-6);
        let tall = (bottom - top).max(1e-6);
        // 48 of room around the bounds, as a map frames its data
        let room = |side: f32| f64::from((side - 96.0).max(side * 0.5));
        let zoom = (room(size.x) / (wide * 256.0))
            .log2()
            .min((room(size.y) / (tall * 256.0)).log2())
            .clamp(0.0, 18.0);
        let middle = (top + bottom) / 2.0;
        let lat = (std::f64::consts::PI * (1.0 - 2.0 * middle)).sinh().atan().to_degrees();
        self.home = lat_lon(lat, (west + east) / 2.0);
        self.memory = MapMemory::default();
        let _ = self.memory.set_zoom(zoom);
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
                Planet::new(source.user_agent),
                options,
                street_style(t, dark),
                source.ctx.clone(),
            ));
            self.styled = Some(key);
        }
    }

    /// Look at the place and zoom the map was made with again.
    pub fn reset(&mut self) {
        self.home = self.start;
        self.memory = MapMemory::default();
        let _ = self.memory.set_zoom(self.zoom);
    }

    /// Draw the map at `size`, with `routes` and then `markers` over it. Drag to pan, scroll or
    /// pinch to zoom, double-click the buttons' minus to step back out.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, markers: &[Marker], routes: &[Route]) -> Response {
        let t = tokens(ui.ctx());
        let has_tiles = self.source.is_some();
        let dark = ui.visuals().dark_mode;
        match self.pending.take() {
            Some(Some(bounds)) => self.fit(bounds, size),
            Some(None) => self.reset(),
            None => {}
        }
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
        if self.framed {
            ui.painter().rect_stroke(
                rect,
                t.radius("surface"),
                Stroke::new(1.0, t.color("border")),
                StrokeKind::Inside,
            );
        }
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

/// OpenFreeMap's current build of the world map, which it names at `https://tiles.openfreemap.org/planet`
/// (its TileJSON). Its builds are dated and an old one may be retired, so the build is looked up
/// once a run, in the background; until the answer comes, or if the lookup fails, tiles come from
/// the build walkers names.
struct Planet {
    current: &'static std::sync::Mutex<Option<String>>,
}

impl Planet {
    fn new(user_agent: &'static str) -> Self {
        static CURRENT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
        static ASKED: std::sync::Once = std::sync::Once::new();
        ASKED.call_once(|| {
            std::thread::spawn(move || {
                if let Some(template) = current_planet(user_agent)
                    && let Ok(mut current) = CURRENT.lock()
                {
                    *current = Some(template);
                }
            });
        });
        Self { current: &CURRENT }
    }
}

/// The tile url template OpenFreeMap's TileJSON names for its current build.
fn current_planet(user_agent: &str) -> Option<String> {
    let answer = reqwest::blocking::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?
        .get("https://tiles.openfreemap.org/planet")
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;
    let json: serde_json::Value = serde_json::from_str(&answer).ok()?;
    let template = json.get("tiles")?.get(0)?.as_str()?;
    template.contains("{z}").then(|| template.to_owned())
}

impl TileSource for Planet {
    fn tile_url(&self, tile_id: TileId) -> String {
        match self.current.lock().ok().and_then(|c| c.clone()) {
            Some(template) => template
                .replace("{z}", &tile_id.zoom.to_string())
                .replace("{x}", &tile_id.x.to_string())
                .replace("{y}", &tile_id.y.to_string()),
            None => OpenFreeMap.tile_url(tile_id),
        }
    }

    fn attribution(&self) -> Attribution {
        OpenFreeMap.attribution()
    }

    fn tile_size(&self) -> u32 {
        OpenFreeMap.tile_size()
    }

    fn max_zoom(&self) -> u8 {
        OpenFreeMap.max_zoom()
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

/// The ground without tiles: `card`, and a graticule in the foreground at 14% over it, its lines
/// at least 64 apart (every 1, 2, 5, 10, 15, 30 or 45 degrees, as the zoom allows) across the view.
fn ground(painter: &egui::Painter, t: &Tokens, rect: Rect, projector: &Projector) {
    painter.rect_filled(rect, CornerRadius::ZERO, t.color("card"));
    let stroke = Stroke::new(1.0, t.color_at("foreground", 0.14));
    let corner = |p: Pos2| projector.unproject(p.to_vec2());
    let (nw, se) = (corner(rect.left_top()), corner(rect.right_bottom()));
    let (west, east) = (nw.x(), se.x());
    let (north, south) = (nw.y().min(85.0), se.y().max(-85.0));
    let per_degree = f64::from(rect.width()) / (east - west).max(1e-6);
    let step = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 45.0]
        .into_iter()
        .find(|step| step * per_degree >= 64.0)
        .unwrap_or(90.0);
    let mut lat = (south / step).floor() * step;
    while lat <= north {
        let a = projector.project(lat_lon(lat, west)).to_pos2();
        let b = projector.project(lat_lon(lat, east)).to_pos2();
        painter.line_segment([a, b], stroke);
        lat += step;
    }
    let mut lon = (west / step).floor() * step;
    while lon <= east {
        let a = projector.project(lat_lon(north, lon)).to_pos2();
        let b = projector.project(lat_lon(south, lon)).to_pos2();
        painter.line_segment([a, b], stroke);
        lon += step;
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
            7.0,
            marker.color.unwrap_or_else(|| series_color(t, marker.series)),
            Stroke::new(1.5, t.color("card")),
        );
        if marker.label.is_empty() {
            continue;
        }
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

#[cfg(test)]
mod tests {
    /// Asks OpenFreeMap for its current build; it reaches the network, so it runs only when asked
    /// (`cargo test -p sokkel-ui-egui-maps -- --ignored`).
    #[test]
    #[ignore]
    fn the_current_planet_is_named() {
        let template = super::current_planet("sokkel-ui-egui-test").expect("an answer");
        assert!(template.starts_with("https://tiles.openfreemap.org/planet/"));
        assert!(template.ends_with("/{z}/{x}/{y}.pbf"));
    }
}
