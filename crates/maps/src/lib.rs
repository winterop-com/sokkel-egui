//! sokkel-egui's maps: one engine, walkers, as sokkel-ui draws every map with one engine. A
//! [`MapView`] keeps where the map looks and its tiles; [`MapView::show`] draws it at a size, with
//! markers and routes over it in the chart colors, its frame in the tokens' rule and corner, and
//! zoom buttons. Its tiles are OpenFreeMap's vector street map, as sokkel-ui's default basemap,
//! drawn light or dark by the scheme over the ground in `card`, so a tile not yet loaded shows the
//! ground. Without tiles (tests, or offline) it draws the ground with a graticule, so the markers
//! and routes still sit somewhere.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText, pos2, vec2};
use sokkel_egui::{Icon, TextRole, Tokens, icons, tokens};
mod popup;
pub use popup::{MapPopup, MapPopupResponse, MapPopupRow, MapPopupWords};

use walkers::{
    HttpOptions, HttpTiles, Layer, Map, MapMemory, Position, Projector, Style, TileId, Tiles, lat_lon,
    sources::{Attribution, OpenFreeMap, TileSource},
};

/// Gets the maps ready to draw their street map's land, water and roads, which are drawn on the
/// GPU: call it once at startup with eframe's wgpu render state,
/// `sokkel_egui_maps::install(cc.wgpu_render_state.as_ref())`. Without it a map draws only the
/// street map's names.
pub use walkers::install_renderer as install;

/// A point on the map with a label, in a series' chart color or a color of its own (a state's).
/// An empty label draws the point alone.
#[derive(Clone, Debug)]
pub struct Marker {
    /// What the map calls the point when it is chosen; its label when not given.
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub series: usize,
    pub color: Option<Color32>,
    /// Its radius; 7 when not sized by a value.
    pub radius: f32,
}

impl Marker {
    pub fn new(lat: f64, lon: f64, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id: label.clone(),
            lat,
            lon,
            label,
            series: 0,
            color: None,
            radius: 7.0,
        }
    }

    /// The point this large, as a point layer sizes each by its value: larger points are drawn
    /// first, under the smaller, with a thin ring in the card.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn series(mut self, series: usize) -> Self {
        self.series = series;
        self
    }

    /// What the map calls the point when it is chosen, such as a site's id.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
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
#[derive(Clone, Debug)]
pub struct MapLegend {
    title: String,
    description: Option<String>,
    rows: Vec<(Color32, String, usize)>,
    ramp: Option<(Vec<Color32>, String, String)>,
}

impl MapLegend {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows: Vec::new(),
            ramp: None,
        }
    }

    /// A sequential scale's classes as a stepped ramp 10 tall, with `low` and `high` under its
    /// ends, as the web's legend of a scale with `captions`.
    pub fn ramp(mut self, colors: Vec<Color32>, low: impl Into<String>, high: impl Into<String>) -> Self {
        self.ramp = Some((colors, low.into(), high.into()));
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn row(mut self, color: Color32, label: impl Into<String>, count: usize) -> Self {
        self.rows.push((color, label.into(), count));
        self
    }

    /// Draws the legend in the bottom-left corner of the map, 12 in, 224 wide: `map` is what
    /// [`MapView::show`] returned, so the legend stands in the map's layer, full screen too.
    pub fn show(self, ui: &Ui, map: &Response) {
        let painter = ui.ctx().layer_painter(map.layer_id).with_clip_rect(map.rect);
        let map = map.rect;
        let t = tokens(ui.ctx());
        let font = TextRole::Help.font(&t);
        let line = t.text("help").line_height;
        let rows = self.rows.len() as f32;
        let ramp_height = self.ramp.as_ref().map_or(0.0, |_| 10.0 + 4.0 + line);
        let height =
            12.0 + line + self.description.as_ref().map_or(0.0, |_| line) + 8.0 + rows * 24.0 + ramp_height + 8.0;
        let width = 224.0_f32.min(map.width() - 24.0);
        let card = Rect::from_min_size(
            pos2(map.left() + 12.0, map.bottom() - 12.0 - height),
            vec2(width, height),
        );
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
        if let Some((colors, low, high)) = &self.ramp {
            let bar = Rect::from_min_size(pos2(x, y), vec2(width - 24.0, 10.0));
            let step = bar.width() / colors.len().max(1) as f32;
            for (index, color) in colors.iter().enumerate() {
                let cell = Rect::from_min_size(pos2(bar.left() + step * index as f32, bar.top()), vec2(step, 10.0));
                painter.rect_filled(cell, CornerRadius::ZERO, *color);
            }
            painter.rect_stroke(
                bar,
                t.radius("mini"),
                Stroke::new(1.0, t.color_at("foreground", 0.1)),
                StrokeKind::Outside,
            );
            let under = bar.bottom() + 4.0 + line / 2.0;
            painter.text(
                pos2(bar.left(), under),
                egui::Align2::LEFT_CENTER,
                low,
                font.clone(),
                muted,
            );
            painter.text(
                pos2(bar.right(), under),
                egui::Align2::RIGHT_CENTER,
                high,
                font.clone(),
                muted,
            );
            y += ramp_height;
        }
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

/// What the map draws under its markers, as sokkel-ui-maps' preset basemaps: the street map
/// light or dark (both in the tokens' colors), NASA's Blue Marble, or the ground alone. A map
/// that has not been given one draws the street map light or dark by the scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Basemap {
    Light,
    Dark,
    Satellite,
    Blank,
}

impl Basemap {
    /// Every basemap the switcher offers, in the web's order.
    pub const ALL: [Basemap; 4] = [Basemap::Light, Basemap::Dark, Basemap::Satellite, Basemap::Blank];

    pub fn name(self) -> &'static str {
        match self {
            Basemap::Light => "Light",
            Basemap::Dark => "Dark",
            Basemap::Satellite => "Satellite",
            Basemap::Blank => "Blank",
        }
    }

    /// The preview the switcher shows beside its name, as the web's; none for the ground alone.
    fn preview(self) -> Option<&'static [u8]> {
        match self {
            Basemap::Light => Some(include_bytes!("../assets/previews/light.jpg")),
            Basemap::Dark => Some(include_bytes!("../assets/previews/dark.jpg")),
            Basemap::Satellite => Some(include_bytes!("../assets/previews/satellite.jpg")),
            Basemap::Blank => None,
        }
    }

    fn credits(self) -> Option<&'static str> {
        match self {
            Basemap::Light | Basemap::Dark => Some("OpenFreeMap © OpenMapTiles © OpenStreetMap contributors"),
            Basemap::Satellite => Some("NASA Blue Marble NASA GIBS"),
            Basemap::Blank => None,
        }
    }
}

/// Where a map looks and what it draws its ground from; keep one per map between frames.
pub struct MapView {
    memory: MapMemory,
    /// Where the street map comes from, once asked for.
    source: Option<Source>,
    /// The street map, styled for the colors in `styled`.
    tiles: Option<HttpTiles>,
    styled: Option<([Color32; 2], bool)>,
    home: Position,
    /// Where the map was made to look.
    start: Position,
    zoom: f64,
    framed: bool,
    /// The bounds to frame once the map's size is known, as `[west, south, east, north]`.
    pending: Option<Option<[f64; 4]>>,
    /// The marker chosen by pressing it, by its id; pressing the map elsewhere lets it go.
    pub selected: Option<String>,
    /// Where each marker was drawn on the last frame, by its id, and the map's room.
    points: Vec<(String, Pos2)>,
    room: Rect,
    /// The bounds last framed, which Fit goes back to; every marker when none were.
    bounds: Option<[f64; 4]>,
    /// The size and zoom the bounds were last framed at: a map the reader has not moved since is
    /// framed again when its size changes, as a grid settles its widths.
    fitted: Option<(Vec2, f64)>,
    /// Drawn over the whole window, as the web's full screen.
    pub full: bool,
    /// The layer the map was drawn in on the last frame, which its legend and card draw in too.
    layer: egui::LayerId,
    /// The basemap chosen; the street map by the scheme when none is.
    pub basemap: Option<Basemap>,
    /// NASA's Blue Marble, once it is drawn.
    satellite: Option<HttpTiles>,
    /// Whether the basemap switcher's menu is open.
    switching: bool,
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
            selected: None,
            points: Vec::new(),
            room: Rect::NOTHING,
            bounds: None,
            fitted: None,
            full: false,
            layer: egui::LayerId::background(),
            basemap: None,
            satellite: None,
            switching: false,
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
        self.bounds = bounds;
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
        // 24 of room around the bounds, as the web's `FIT_MARGIN` frames its data
        let room = |side: f32| f64::from((side - 48.0).max(side * 0.5));
        let zoom = (room(size.x) / (wide * 256.0))
            .log2()
            .min((room(size.y) / (tall * 256.0)).log2())
            .clamp(0.0, 18.0);
        let middle = (top + bottom) / 2.0;
        let lat = (std::f64::consts::PI * (1.0 - 2.0 * middle)).sinh().atan().to_degrees();
        self.home = lat_lon(lat, (west + east) / 2.0);
        self.memory = MapMemory::default();
        let _ = self.memory.set_zoom(zoom);
        self.fitted = Some((size, self.memory.zoom()));
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
        let key = ([t.color("card"), t.color("foreground")], dark);
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

    /// The basemap drawn: the one chosen, or the street map by the scheme.
    pub fn shown(&self, dark: bool) -> Basemap {
        self.basemap
            .unwrap_or(if dark { Basemap::Dark } else { Basemap::Light })
    }

    /// NASA's Blue Marble from its tile service, made the first time it is drawn.
    fn satellite_tiles(&mut self) {
        let Some(source) = self.source.as_ref() else {
            return;
        };
        if self.satellite.is_none() {
            let options = HttpOptions {
                cache: source.cache.clone(),
                user_agent: Some(walkers::HeaderValue::from_static(source.user_agent)),
                ..HttpOptions::default()
            };
            self.satellite = Some(HttpTiles::with_options(BlueMarble, options, source.ctx.clone()));
        }
    }

    /// Look at the place and zoom the map was made with again.
    pub fn reset(&mut self) {
        self.home = self.start;
        self.memory = MapMemory::default();
        let _ = self.memory.set_zoom(self.zoom);
    }

    /// Draw the map at `size`, with `routes` and then `markers` over it. Drag to pan, scroll or
    /// pinch to zoom; its buttons zoom in and out, fit back to what it framed (or to every
    /// marker), and take it to full screen: over the whole window, its place in the page keeping
    /// its ground, until Exit full screen or Escape.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, markers: &[Marker], routes: &[Route]) -> Response {
        if !self.full {
            return self.draw(ui, size, markers, routes);
        }
        let t = tokens(ui.ctx());
        let (place, _) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect_filled(place, CornerRadius::ZERO, t.color("card"));
        // Escape leaves full screen, unless a card is open to close first
        if self.selected.is_none() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.full = false;
            ui.ctx().request_repaint();
        }
        let screen = ui.ctx().content_rect();
        egui::Area::new(ui.id().with("map-full"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ui.ctx(), |ui| self.draw(ui, screen.size(), markers, routes))
            .inner
    }

    /// The layer the map was drawn in on the last frame: its page's, or the full screen's.
    pub fn layer(&self) -> egui::LayerId {
        self.layer
    }

    fn draw(&mut self, ui: &mut Ui, size: Vec2, markers: &[Marker], routes: &[Route]) -> Response {
        let t = tokens(ui.ctx());
        let dark = ui.visuals().dark_mode;
        let shown = self.shown(dark);
        let has_tiles = self.source.is_some() && shown != Basemap::Blank;
        match self.pending.take() {
            Some(Some(bounds)) => self.fit(bounds, size),
            Some(None) => self.reset(),
            None => {
                let unmoved = self.fitted.is_some_and(|(at, zoom)| {
                    at != size && self.memory.detached().is_none() && self.memory.zoom() == zoom
                });
                if unmoved && let Some(bounds) = self.bounds {
                    self.fit(bounds, size);
                }
            }
        }
        // never so far out that the world is narrower than the map: it is drawn once, not
        // repeated sideways, so the room either side of it would stand empty
        let least = least_zoom(size);
        if self.memory.zoom() < least {
            let _ = self.memory.set_zoom(least);
        }
        let home = self.home;
        // the street map light or dark whatever the page's scheme, in that scheme's colors: the
        // page's tokens, or the same theme's other scheme's
        let wants_dark = match shown {
            Basemap::Light => Some(false),
            Basemap::Dark => Some(true),
            _ => None,
        };
        let map_t = match wants_dark {
            Some(want) if want != t.dark => sokkel_egui::scope(ui, &t.theme, Some(want), |ui| tokens(ui.ctx())),
            _ => t.clone(),
        };
        let t_page = t;
        let t = map_t;
        let tiles = match shown {
            Basemap::Light | Basemap::Dark => {
                self.tiles_for(&t, shown == Basemap::Dark);
                self.tiles.as_mut().map(|tiles| tiles as &mut dyn Tiles)
            }
            Basemap::Satellite => {
                self.satellite_tiles();
                self.satellite.as_mut().map(|tiles| tiles as &mut dyn Tiles)
            }
            Basemap::Blank => None,
        };
        let mut points = Vec::new();
        let response = ui
            .allocate_ui(size, |ui| {
                ui.set_min_size(size);
                // the ground in `card`, under the tiles, showing where one has not loaded
                let room = Rect::from_min_size(ui.cursor().min, size);
                let corners = if self.framed && !self.full {
                    t.radius("surface")
                } else {
                    CornerRadius::ZERO
                };
                ui.painter().rect_filled(room, corners, t.color("card"));
                Map::new(tiles, &mut self.memory, home)
                    .zoom_with_ctrl(false)
                    .show(ui, |ui, response, projector, _| {
                        let painter = ui.painter_at(response.rect);
                        if !has_tiles {
                            ground(&painter, &t, response.rect, projector, corners);
                        }
                        overlays(&painter, &t, projector, markers, routes);
                        points = markers
                            .iter()
                            .map(|m| {
                                let at = projector.project(lat_lon(m.lat, m.lon)).to_pos2();
                                (m.id.clone(), at, m.radius)
                            })
                            .collect();
                        if has_tiles && let Some(credits) = shown.credits() {
                            attribution(ui, &painter, &t, response.rect, credits);
                        }
                    })
                    .response
            })
            .inner;
        let rect = response.rect;
        // a marker pressed is chosen, the map pressed elsewhere lets it go
        let near = |pos: Pos2| {
            points
                .iter()
                .filter(|(_, at, radius)| at.distance(pos) <= radius.max(7.0) + 3.0)
                .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)))
                .map(|(id, _, _)| id.clone())
        };
        if response.hover_pos().and_then(near).is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            self.selected = response.interact_pointer_pos().and_then(near);
        }
        if let Some(chosen) = &self.selected
            && !points.iter().any(|(id, _, _)| id == chosen)
        {
            self.selected = None;
        }
        self.points = points.into_iter().map(|(id, at, _)| (id, at)).collect();
        self.room = rect;
        self.layer = response.layer_id;
        let t = t_page;
        if self.framed && !self.full {
            ui.painter().rect_stroke(
                rect,
                t.radius("surface"),
                Stroke::new(1.0, t.color("border")),
                StrokeKind::Inside,
            );
        }
        self.controls(ui, &t, rect, markers, shown);
        response
    }

    /// Where the marker with this id was drawn on the last frame.
    pub fn point_of(&self, id: &str) -> Option<Pos2> {
        self.points.iter().find(|(one, _)| one == id).map(|(_, at)| *at)
    }

    /// The zoom the map looks at now: 0 the world in 256 points, one more each halving.
    pub fn zoom(&self) -> f64 {
        self.memory.zoom()
    }

    /// The map's room on the last frame.
    pub fn room(&self) -> Rect {
        self.room
    }

    /// The control stack in the top right corner: zoom in, zoom out and fit as one group, then
    /// full screen, each a 28 square in the popover's ground, as the web's map controls.
    fn controls(&mut self, ui: &mut Ui, t: &Tokens, rect: Rect, markers: &[Marker], shown: Basemap) {
        const SIZE: f32 = 28.0;
        let right = rect.right() - 10.0;
        let mut top = rect.top() + 10.0;
        let fit_words = "Fit to the data";
        let (full_icon, full_words) = if self.full {
            (Icon::Shrink, "Exit full screen")
        } else {
            (Icon::Expand, "Full screen")
        };
        let basemap_words = "Basemap";
        let groups: [&[(Icon, &str)]; 3] = [
            &[
                (Icon::Plus, "Zoom in"),
                (Icon::Minus, "Zoom out"),
                (Icon::LocateFixed, fit_words),
            ],
            &[(full_icon, full_words)],
            &[(Icon::Layers, basemap_words)],
        ];
        let mut switcher = Rect::NOTHING;
        let mut pressed = None;
        for group in groups {
            let whole = Rect::from_min_size(pos2(right - SIZE, top), vec2(SIZE, SIZE * group.len() as f32));
            let corners = t.radius("control");
            ui.painter().rect_filled(whole, corners, t.color("popover"));
            for (i, (icon, label)) in group.iter().enumerate() {
                let button = Rect::from_min_size(pos2(whole.left(), whole.top() + i as f32 * SIZE), vec2(SIZE, SIZE));
                let response = ui.interact(button, ui.id().with(("map-control", *label)), Sense::click());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, *label));
                if response.hovered() {
                    let corner = |first: bool| if first { corners.nw } else { 0 };
                    let last = i + 1 == group.len();
                    let shape = CornerRadius {
                        nw: corner(i == 0),
                        ne: corner(i == 0),
                        sw: if last { corners.sw } else { 0 },
                        se: if last { corners.se } else { 0 },
                    };
                    ui.painter().rect_filled(button, shape, t.color("muted"));
                }
                if i > 0 {
                    ui.painter()
                        .hline(button.x_range(), button.top(), Stroke::new(1.0, t.color("border")));
                }
                icons::paint(ui.painter(), button.center(), *icon, 16.0, t.color("foreground"));
                if *label == basemap_words {
                    switcher = button;
                }
                if response.on_hover_text(*label).clicked() {
                    pressed = Some(*label);
                }
            }
            ui.painter()
                .rect_stroke(whole, corners, Stroke::new(1.0, t.color("border")), StrokeKind::Inside);
            top = whole.bottom() + 8.0;
        }
        match pressed {
            Some("Zoom in") => {
                let _ = self.memory.set_zoom((self.memory.zoom() + 1.0).clamp(2.0, 18.0));
            }
            Some("Zoom out") => {
                let least = least_zoom(rect.size()).max(2.0);
                let _ = self.memory.set_zoom((self.memory.zoom() - 1.0).clamp(least, 18.0));
            }
            Some(label) if label == fit_words => {
                let every = markers.iter().fold(None::<[f64; 4]>, |b, m| {
                    Some(match b {
                        None => [m.lon, m.lat, m.lon, m.lat],
                        Some([w, s, e, n]) => [w.min(m.lon), s.min(m.lat), e.max(m.lon), n.max(m.lat)],
                    })
                });
                self.pending = Some(self.bounds.or(every));
            }
            Some(label) if label == basemap_words => self.switching = !self.switching,
            Some(_) => {
                self.full = !self.full;
                ui.ctx().request_repaint();
            }
            None => {}
        }
        if self.switching && pressed != Some(basemap_words) {
            self.basemap_menu(ui, t, switcher, shown);
        }
    }

    /// The basemap switcher's menu, to the left of its button: the label, then a row per basemap
    /// with its preview and name, a check before the one drawn, as the web's `BasemapSwitcher`.
    fn basemap_menu(&mut self, ui: &mut Ui, t: &Tokens, button: Rect, shown: Basemap) {
        const ROW: f32 = 44.0;
        const LABEL: f32 = 28.0;
        let size = vec2(208.0, 4.0 + LABEL + ROW * Basemap::ALL.len() as f32 + 4.0);
        let menu = Rect::from_min_size(pos2(button.left() - 4.0 - size.x, button.top()), size);
        // over the map and its legend, a full screen map too
        let area = egui::Area::new(ui.id().with("map-basemap-menu"))
            .order(egui::Order::Tooltip)
            .fixed_pos(menu.min)
            .show(ui.ctx(), |ui| Self::basemap_rows(ui, t, menu, shown));
        let chosen = area.inner;
        let away = ui.input(|i| i.pointer.any_click())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !menu.contains(p) && !button.contains(p));
        if let Some(basemap) = chosen {
            self.basemap = Some(basemap);
            self.switching = false;
        } else if away || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.switching = false;
        }
    }

    /// The menu's ground, its label and a row per basemap; the basemap pressed.
    fn basemap_rows(ui: &mut Ui, t: &Tokens, menu: Rect, shown: Basemap) -> Option<Basemap> {
        const ROW: f32 = 44.0;
        const LABEL: f32 = 28.0;
        let size = menu.size();
        ui.allocate_rect(menu, Sense::hover());
        let corners = t.radius("md");
        ui.painter().add(t.shadow("elevation-pop").as_shape(menu, corners));
        ui.painter().rect(
            menu,
            corners,
            t.color("popover"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        let label = WidgetText::from(TextRole::Help.rich(t, "Basemap").variation("wght", 500.0)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(t),
        );
        ui.painter().galley(
            pos2(
                menu.left() + 4.0 + 6.0,
                menu.top() + 4.0 + (LABEL - label.size().y) / 2.0,
            ),
            label,
            t.color("muted-foreground"),
        );
        let mut chosen = None;
        for (i, basemap) in Basemap::ALL.into_iter().enumerate() {
            let row = Rect::from_min_size(
                pos2(menu.left() + 4.0, menu.top() + 4.0 + LABEL + i as f32 * ROW),
                vec2(size.x - 8.0, ROW),
            );
            let response = ui.interact(row, ui.id().with(("map-basemap", basemap.name())), Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, basemap == shown, basemap.name())
            });
            if response.hovered() {
                ui.painter().rect_filled(row, t.radius("md"), t.color("accent"));
            }
            if basemap == shown {
                icons::paint(
                    ui.painter(),
                    pos2(row.left() + 6.0 + 8.0, row.center().y),
                    Icon::Check,
                    16.0,
                    t.color("popover-foreground"),
                );
            }
            let preview = Rect::from_min_size(pos2(row.left() + 28.0, row.center().y - 16.0), vec2(44.0, 32.0));
            let mini = CornerRadius::same(t.length("radius-mini").floor() as u8);
            match basemap
                .preview()
                .and_then(|bytes| preview_texture(ui.ctx(), basemap.name(), bytes))
            {
                Some(texture) => {
                    egui::Image::new((texture.id(), preview.size()))
                        .corner_radius(mini)
                        .paint_at(ui, preview);
                }
                None => {
                    // the ground alone: the card with a dot every 6, as the web's blank preview
                    ui.painter().rect_filled(preview, mini, t.color("card"));
                    let dots = ui.painter().with_clip_rect(preview.shrink(1.0));
                    let mut y = preview.top() + 3.0;
                    while y < preview.bottom() {
                        let mut x = preview.left() + 3.0;
                        while x < preview.right() {
                            dots.circle_filled(pos2(x, y), 0.5, t.color("border-strong"));
                            x += 6.0;
                        }
                        y += 6.0;
                    }
                }
            }
            ui.painter().rect_stroke(
                preview,
                mini,
                Stroke::new(1.0, t.color_at("foreground", 0.15)),
                StrokeKind::Outside,
            );
            ui.painter().text(
                pos2(preview.right() + 10.0, row.center().y),
                egui::Align2::LEFT_CENTER,
                basemap.name(),
                TextRole::Body.font(t),
                t.color("popover-foreground"),
            );
            if response.clicked() {
                chosen = Some(basemap);
            }
        }
        chosen
    }
}

/// A basemap's preview as a texture, decoded once and kept by the context.
fn preview_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("sokkel-map-preview", name));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    let texture = ctx.load_texture(format!("map-preview-{name}"), pixels, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
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

/// NASA's cloud-free Blue Marble mosaic from GIBS, public domain, sharp to about a region's scale.
struct BlueMarble;

impl TileSource for BlueMarble {
    fn tile_url(&self, tile_id: TileId) -> String {
        format!(
            "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_NextGeneration/default/GoogleMapsCompatible_Level8/{}/{}/{}.jpeg",
            tile_id.zoom, tile_id.y, tile_id.x
        )
    }

    fn attribution(&self) -> Attribution {
        Attribution {
            text: "NASA Blue Marble, NASA GIBS",
            url: "https://earthobservatory.nasa.gov/features/BlueMarble",
            logo_light: None,
            logo_dark: None,
        }
    }

    fn max_zoom(&self) -> u8 {
        8
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
    let tone = |share: f32| walkers::Color(walkers::json!(sokkel_egui::hex(mix(ink, ground, share))));
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
                filter,
                layout,
                paint,
            } => {
                let place = source(source_layer).contains("place");
                // a country's name small, as the web's street maps write it, not walkers' 18
                if place && format!("{filter:?}").contains("\"country\"") {
                    layout.text_size = Some(walkers::Float(walkers::json!(11.0)));
                    layout.text_field = Some(walkers::json!([
                        "coalesce",
                        ["get", "name:latin"],
                        ["get", "name:en"],
                        ["get", "name"]
                    ]));
                }
                if layout.text_field == Some(walkers::json!(["get", "name"])) {
                    layout.text_field = Some(walkers::json!(["coalesce", ["get", "name:latin"], ["get", "name"]]));
                }
                let paint = paint.get_or_insert_with(Default::default);
                paint.text_color = Some(if place {
                    tone(0.8)
                } else {
                    walkers::Color(walkers::json!(sokkel_egui::hex(t.color("muted-foreground"))))
                });
                paint.text_halo_color = Some(walkers::Color(walkers::json!(sokkel_egui::hex(ground))));
            }
            _ => {}
        }
    }
    style
}

/// The least zoom at which the world, 256 points wide at zoom 0, is as wide as `size`.
fn least_zoom(size: Vec2) -> f64 {
    (f64::from(size.x.max(1.0)) / 256.0).log2().max(0.0)
}

/// The ground without tiles: `card`, and a graticule in the foreground at 14% over it, its lines
/// at least 64 apart (every 1, 2, 5, 10, 15, 30 or 45 degrees, as the zoom allows) across the view.
fn ground(painter: &egui::Painter, t: &Tokens, rect: Rect, projector: &Projector, corners: CornerRadius) {
    painter.rect_filled(rect, corners, t.color("card"));
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
    let mut order: Vec<&Marker> = markers.iter().collect();
    order.sort_by(|a, b| b.radius.total_cmp(&a.radius));
    for marker in order {
        let point = at(marker.lat, marker.lon);
        let sized = marker.radius != 7.0;
        let fill = marker.color.unwrap_or_else(|| series_color(t, marker.series));
        painter.circle(
            point,
            marker.radius,
            if sized { fill.gamma_multiply(0.95) } else { fill },
            Stroke::new(if sized { 1.25 } else { 1.5 }, t.color("card")),
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
fn attribution(ui: &Ui, painter: &egui::Painter, t: &Tokens, rect: Rect, credits: &str) {
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

/// The colors of a sequential scale of `count` classes, as sokkel-ui-maps' `sequentialScale` ramps
/// them from `hue`: the first 70% of the way from 26% of the hue over the card to the hue itself,
/// the rest toward the foreground, 35% at the last.
pub fn sequential_colors(t: &Tokens, hue: &str, count: usize) -> Vec<Color32> {
    let hue = t.color(hue);
    (0..count)
        .map(|index| {
            let at = if count <= 1 {
                1.0
            } else {
                index as f32 / (count - 1) as f32
            };
            if at <= 0.7 {
                let share = (26.0 + 74.0 * at / 0.7).round() / 100.0;
                mix(hue, t.color("card"), share)
            } else {
                let share = (35.0 * (at - 0.7) / 0.3).round() / 100.0;
                mix(t.color("foreground"), hue, share)
            }
        })
        .collect()
}

/// The class of `value` on a scale of ascending `breaks`: how many breaks it reaches.
pub fn class_of(breaks: &[f64], value: f64) -> usize {
    breaks.iter().take_while(|b| value >= **b).count()
}

/// A point's radius sized by its value, as sokkel-ui-maps' `pointRadius`: the value's square root
/// drawn from `near` to `far` across the domain from 0 to `most`.
pub fn point_radius(value: f64, most: f64, near: f32, far: f32) -> f32 {
    if most <= 0.0 {
        return far;
    }
    let share = (value.max(0.0).sqrt() / most.sqrt()).clamp(0.0, 1.0) as f32;
    near + (far - near) * share
}

#[cfg(test)]
mod tests {
    /// Asks OpenFreeMap for its current build; it reaches the network, so it runs only when asked
    /// (`cargo test -p sokkel-egui-maps -- --ignored`).
    #[test]
    #[ignore]
    fn the_current_planet_is_named() {
        let template = super::current_planet("sokkel-egui-test").expect("an answer");
        assert!(template.starts_with("https://tiles.openfreemap.org/planet/"));
        assert!(template.ends_with("/{z}/{x}/{y}.pbf"));
    }
}
