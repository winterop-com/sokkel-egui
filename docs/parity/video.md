# Video: sokkel-ui's video example in egui

sokkel-ui's video example (Tidevann's video site: channels and their videos, each video's clip
made from a scene, following, likes, the history and where the viewer stopped, Watch later and
playlists, comments, downloads, the player's settings, the search and what plays next), ported one
to one as `examples/video`. Its manifest is `parity/video.json`; the web's screens are shot into
`parity/video/web/` by `bun scripts/parity.ts web video`.

## Where it stands

- The data is the web's to the value (`catalog.rs`, `clips/`, `data.rs`, `format.rs`, `site.rs`,
  `parts.rs`, `dialogs.rs` and each page's view and actions in `pages/`, checked in `tests/web.rs`
  against the web's own modules run under bun). The pure parts it leaned on now live in the core:
  `numbers` (en-US compact counts, grouping and `toFixed`, which `js::compact_number`,
  `locale_number` and `to_fixed` now are; `js` itself is `tidevann_shared::js`, the browser
  stand-ins the examples share, and the data layer's collections and clock are
  `tidevann_shared::backend`), `captions` (the sizes, inks, boxes and edges, a choice read back field by
  field, the track chosen, and the tokens a cue is painted with) and `toggle_reaction` with its
  `Reaction` (the comments' `CommentReaction`). The data layer gained `Kept` and `restore`, the rows
  eframe's storage keeps put back on a new visit.
- The frame (`app.rs`): the rail with Home, Subscriptions and Library over their descriptions, You
  with History, Watch later, Liked videos, Playlists and Downloads and their counts, and the
  channels followed each with its color's dot; collapsed to its icons on a video's page, the counts
  as round badges at their corners (the web's `useRailMode('collapsed')`). The top bar's search
  (Enter goes to `search/<words>`, `/` reaches it), the trail with its links, the palette's button;
  the status line with what was watched and downloaded. The address is the web's hash route:
  `home`, `watch/<id>[/list/<id>|/at/<second>]`, `channel/<id>[/about]`, `subscriptions`, `library`,
  `history[/clear]`, `later`, `liked`, `playlists`, `playlist/<id>`, `downloads`, `search/<words>`,
  and `settings[/<section>]` over the page; anything else is a page that leads nowhere.
- The pages (`screens.rs`): Home with its topic chips over the grid; a channel with its banner,
  Follow, the Videos and About tabs, the sort and Play; Subscriptions with its channel chips, This
  week and Earlier, and Manage; the Library's parts with See all; History by day with its search,
  Keep history and Clear all (asked first); Watch later and a playlist with Play all, Shuffle,
  renaming in place, Delete (asked first) and each video's Move up, Move down and Remove; Liked
  videos; Playlists with its field for a new one; Downloads as a table; the search with its sort,
  its chips and the channels with Follow over the videos. A video's card is the web's `VideoCard`:
  its still painted from its own scene a third of the way in, its length, how far the viewer got,
  and its menu (Watch later, Save to, Share, its channel, then the page's own).
- The watch page (`watch_page.rs`): the player with the frame painted from the clip's scene at the
  second playing (the title card first, the poster before it plays), the stats over the picture,
  the captions' cue, Up next counting down as it ends with Stay here; the title, the channel with
  Follow, like and dislike, Share, Save, Download (the core's `DownloadButton`) and More with Stats;
  the description held to two lines under its facts; the comments with their sort, the composer,
  likes, reactions, replies, Delete and Report with Undo; and what plays next with Autoplay and its
  chips, or the playlist playing. The keys are the web's: K and Space, J and L, the arrows, M, F, C,
  T, I, Shift+N and the digits; Escape leaves full screen.
- Settings (`settings_view.rs`): playback, captions, the history, the look with the carried themes,
  storage and the demo data, each section's Reset to defaults. Save to and Share (`overlays.rs`).
  The palette (`palette.rs`): this video's commands, the places, the playlists, the player's
  settings, every video and channel, and the look. Everything kept is saved with eframe's storage.
- No sound plays and nothing is encoded: the player's time moves on with the clock at the speed
  chosen, a tenth of a second at a time, the data layer's timers firing as on the web (the place
  kept every four seconds, the history row made, a download's share moving and its toast), and the
  next video plays when Up next's countdown runs out. The making state does not exist here, so the
  status line always says every video is made in this browser.
- New in the library: in the core, `VideoPlayer` (with `VideoQuality`, built from the media
  controls), `ShowMore`, `CommentThread` and `CommentComposer` (with `ThreadComment` and
  `QUICK_REACTIONS`), `numbers`, `captions`, `Reaction` and `toggle_reaction`; `RailEntry::dot` and
  a collapsed rail's count badges; `PersonAvatar::color` and its `Xl` size; `ChipBar`'s back and
  forward arrows where the chips run past the row; `SearchField`'s clear button; `Button::on_picture`
  and `IconButton::on_picture`; and the emoji face behind the text faces (egui's monochrome Noto
  Emoji) for reactions. In the media add-on: `WatchLayout`, `ChannelRow`, `MediaActions` (with
  `MediaAction` and `MediaRating`), `UpNext` and `VideoStatsOverlay` (with `VideoStats`). Each has a
  story (Media/Video player, Media/Up next, Media/Video stats, Media/Channel row, Media/Watch layout,
  Patterns/Show more, Patterns/Comment thread, Patterns/Rail collapsed).

## Open

- The web's screenshot of the watch page shows the video paused at its start: a browser lets a
  video play by itself only after the viewer has pressed something on the site, which the shot
  never does. The app holds the same in its tests and screenshots (`VideoApp::at`), and plays as
  the page opens when run (`may_autoplay`).
- The small outlined buttons under the video (Share, Saved, Download) are a few points narrower
  than the web's, and the quality button's `Auto` a little narrower, so the player's end controls
  stand a few points further right: the core's `Button` measures the code face and its icon gap a
  little tighter than the browser.
- The like and dislike pair are two buttons side by side with their own corners, not one joined
  group, and Saved's bookmark and a like's thumb are outlines: Lucide's glyphs are drawn from the
  icon font, which has no filled forms.
- The home's chips stand 8 apart where the web's read 10, so the row fits without the forward arrow
  the web shows at its end.
- The comments' sort is a ghost button with a menu rather than a quiet select. Picture in picture
  is the app's own: the player small in the window's corner while the page stays open, its place in
  the page saying where it plays, as the browser's floating window would be outside the window.
- The palette's search lists every video and channel as commands rather than searching from the
  second letter with the stills beside them: the core's `CommandPalette` has no search of the app's
  own nor pictures in its rows.
- Captions offer the one English track and turn on or off from the player; their size, ink, box
  and edge are set in Settings, not in the player's menu, which has no submenus.
