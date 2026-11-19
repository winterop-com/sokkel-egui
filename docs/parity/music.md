# Music: sokkel-ui's music example in egui

sokkel-ui's music example (Tidevann's music library: albums, artists and songs a synthesizer makes,
favorites, downloads, radio and playlists, with a player along the foot and its queue beside the
page), ported one to one as `examples/music`. Its manifest is `parity/music.json`; the web's screens
are shot into `parity/music/web/` by `bun scripts/parity.ts web music`.

## Where it stands

- The data is the web's to the value (`catalog.rs`, `synth.rs`, `data.rs`, `downloads.rs`,
  `lyrics.rs`, `player.rs`, `queue.rs`, `search.rs`, `look.rs`, `themes.rs`, checked in
  `tests/web.rs` against the web's own modules run under bun). The words' LRC is read by the media
  add-on's `lrc` (re-exported as `music::lrc`), which answers the web's inputs to the bit.
- The frame (`app.rs`): the rail with the places, their lines under them (the albums and songs
  counted, the favorites starred, the downloads coming in, the station on air), the playlists by
  name, each taking a song's row dropped on it (`Rail::drops`), and Settings; while the listener
  browses by artist the rail holds the library's tree with its filter (`RailBody`, `Tree`), the
  artist the page is of opened. The top bar's search over the library with its results page, the
  trail with its crumbs going back up (`TopBar::linked`, new in the library), the palette's button.
  The player along the foot across the whole window (`Shell::bottom`, new in the library), the
  queue beside the page as the shell's panel, and the status line: what plays and from where, the
  crossfade and speed when set, the library and what is kept.
- The pages (`pages.rs`): the albums with their filter, order (kept in the settings) and Shuffle
  all; an album with its header (the artist a link in its meta line, `PageHeader::meta_line` and
  `MetaLine::link`, new in the library), Download, the star, Add to playlist and More, the cover
  (`Artwork::framed`, new) with Play or Pause or Resume and Shuffle (the play glyph filled,
  `Button::filled_icon`, new), the songs as a draggable list and more by the artist; the artists
  by name or most albums; an artist with their most played songs and albums, the star, Radio,
  Shuffle and Play; every song, windowed, in four orders; the favorites; the downloads in a table;
  radio with the stations, what the one playing makes next, and the listener's own stations; the
  playlists as cards and one playlist with its name and line edited in place, its songs reordered;
  the search's results.
- The cards and rows (`songs.rs`): an album's and an artist's card with their menus; a song's row
  with its star and its menu (the queue, a playlist, its album and artist, a station); the empty
  states.
- The player bar (`player_bar.rs`): `MediaBar` with what plays, the transport, shuffle and repeat,
  the speed, the volume and the queue's button; grown, the song's waveform made by the synthesizer
  as its scrubber and the visualizer behind it; the full screen with the looks, the words and a
  star. The visualizer reads the song's own samples at the playhead the way the web's analyser
  reads the sound (`sound_visualizer::frequency_bytes`, new in the library). The one-line bar now
  stands 71 tall with its time's row under the transport as the web's, and its cover 48.
- The queue (`queue_panel.rs`): what plays now, Up next reordered or taken out and cleared
  (`panel_section_actions`, new in the library), where it plays from, and History.
- The dialogs (`dialogs.rs`): a new playlist (holding songs when made from a menu), a rename, a
  delete, and the question before every download goes; Settings over the page (`settings.rs`:
  playback, the player bar, the visualizer, the library on this device, the look with the carried
  themes; a setting with no description, as Volume, now stands as its title alone); the palette
  with playback, the places, a new playlist, the look and the library's search; the player's keys
  (`palette.rs`). The library is kept with eframe's storage.
- No sound plays: the playhead moves on with the clock at the speed chosen, the next song starts
  as one ends (or crossfades in), and radio keeps making the next.

## Open

- The menus' "Add to playlist" is a submenu on the web; here its items stand in the menu under an
  "Add to playlist" heading, the library's `DropdownMenu` having no submenus yet.
- The search's results show the kind's icon where the web shows the cover or the artist's round
  picture: the core's `SearchResult` has no picture of the app's own.
- The downloads' table writes the album and its state as words: the web's cells hold the cover and
  a `DownloadButton`, which the core's `DataTable` cells cannot hold yet.
- A playlist's line under its name is edited under the header rather than in its place: the
  header's description takes words alone.
- Rows of the queue and of a playlist are reordered but not dragged onto a playlist in the rail:
  `ReorderableTrackList` carries no `DragData` kind as the web's does.
- `music::queue` stays the example's own: the media add-on's `Queue` keeps its place as an
  `Option<usize>` and takes whole numbers where the web's (and `tests/web.rs`) hand it JavaScript's
  numbers (a start and a seed as floats, an order read back from storage as signed numbers), so the
  same inputs would not give the web's answers.
- The pages' content stands within a pixel or two of the web's; the rail's rows sit a pixel lower.
