# Podcast: sokkel-ui's podcast example in egui

sokkel-ui's podcast example (Tidevann's podcasts: shows and episodes whose voices a script speaks,
following, New, the queue, downloads, the sleep timer, the search and the time listened, with the
player along the foot), ported one to one as `examples/podcast`. Its manifest is
`parity/podcast.json`; the web's screens are shot into `parity/podcast/web/` by
`bun scripts/parity.ts web podcast`.

## Where it stands

- The data is the web's to the value (`catalog.rs`, `backend.rs`, `script.rs`, `voice.rs`,
  `state.rs`, `discover.rs`, `stats.rs`, `parts.rs`, `themes.rs`, checked in `tests/web.rs` against
  the web's own modules run under bun). The pure parts of sokkel-ui-media it leans on now live in
  the media add-on and are re-exported (`podcast::chapters`, `::transcript`, `::sleep_timer`):
  `chapters` (with `waveform_span`, new), `transcript` and `sleep_timer`, answering the web's inputs
  to the bit as before. The provider's state gained `restore`, the rows kept by eframe's storage put
  back on a new visit.
- The frame (`app.rs`): the rail with Listen now, New, Queue and Discover, the library's Shows,
  Starred and Downloads, and Stats and Settings, each with its count; Queue, Starred and Downloads
  take an episode's row dropped on them when it wants it (not playing nor queued, not starred, not
  kept). The top bar's search over the shows and the released episodes (See all opens Discover with
  the words), the trail with its crumbs going back up, the palette's button. The player along the
  foot, and the status line: what is downloading or that all is saved, new, queued and downloaded.
  The address is the web's hash route: `home`, `inbox`, `queue`, `discover[/<category>[/<words>]]`,
  `shows`, `show/<id>`, `episode/<id>`, `starred`, `downloads`, `stats`, and `settings[/<section>]`
  over the page; anything else is a page that leads nowhere.
- The pages (`pages.rs`): Listen now with Continue listening (cards with their progress), Up next
  and the latest from the shows followed; New with Check for new (its toast once the backend
  answers) and Archive all, each row with its archive button; the queue with what plays now and Up
  next reordered, Play next and Clear (asked first); Discover with its search kept in the address
  and the categories as chips, the shows as a grid and the episodes found; the shows followed; a
  show with Play latest or Resume, Follow or Following (Unfollow asked first), its cover beside what
  it is about, the filter and Mark all played; an episode with Play or Resume at, its star, its
  download and its menu, the summary with Hear the summary, the show notes in markdown whose times
  seek and whose links go, the chapters, and the transcript following the player; Starred;
  Downloads with what comes in and what is kept, Remove all asked first; Stats with its four tiles
  and the bars of the top shows and the last seven days.
- The rows (`episodes.rs`): an episode as a row with its show's cover and its line (the show or its
  number, when it came out, where the listener stands), its star, its download, its menu (the
  queue, played, archive, details, its show); lists that drag onto the rail or reorder the queue;
  the grid of shows with their new counts, the latest played from the card and Follow in its menu.
- The player bar (`player_bar.rs`): `MediaBar` with the title and the show as links
  (`MediaBar::linked`, new in the library), the skip buttons at the interval the settings choose,
  previous and next stepping through the episode's chapters (`MediaBar::chapters`, new), the speed
  over the web's rates, the volume, the sleep timer (`SleepTimerMenu`, new) and the queue's button;
  the full screen with the transcript in the visualizer's place (`Transcript`, new) and a star, the
  voices' sound made for it with its waveform and the visualizer reading it.
- Settings (`settings.rs`): playback, new episodes, the look with the carried themes, and the data
  (clear the stats, reset the demo, each asked first). The palette (`palette.rs`): the player's
  commands (play, skip, chapters, the next, what plays, the speeds, the sleep timer), Check for new,
  the places, every show and every released episode, and the look; the keys Space, the arrows, N, S
  and F. Everything kept is saved with eframe's storage.
- No sound plays: the playhead moves on with the clock at the speed chosen, a quarter second at a
  time, the provider's timers firing as on the web (the place kept, the time listened counted,
  downloads coming in, the sleep timer counting down), and the next in the queue starts as one
  ends. Hear the summary plays for as long as the voice would speak it.
- New in the library besides: `ChapterList` (on `TrackList`, whose rows can now write their second
  line in the code face, `Track::mono_subtitle`) and `SpeakButton` in the core (with `speak_press`);
  each has a story (Media/Chapter list, Media/Transcript, Media/Sleep timer, Media/Speak button).
  A boxed `Section`'s actions now stand centered in its head, and `BarList` measures the chosen
  label at its own weight so it is not cut short.

## Open

- The show's filter is one row of four; the web's toolbar squeezes the segmented control into two
  rows of two, a narrow slot of its own rather than a design.
- The search's results and the palette's shows and episodes show an icon where the web shows the
  show's cover: the core's `SearchResult` and `MenuItem` hold no picture of the app's own.
- The row controls stand 30 apart where the web's stand 34, and card titles are cut a letter or two
  sooner.
- `SpeakButton`'s labeled form takes no press while loading (the core's busy `Button` takes none);
  the example never loads, since its voice is made at once.
- The sleep timer's menu opens under its moon where the web's opens above it: `DropdownMenu` has no
  side of its own yet.
- The bar has no chapter timeline (`timeline="chapter"` on the web), which the podcast does not use.
