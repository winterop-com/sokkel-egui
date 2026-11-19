# Cut: sokkel-ui's cut example in egui

sokkel-ui's cut example (Tidevann's video editor: projects cut from a bin of procedural clips,
with a preview, an inspector and a multi-track timeline, on a simulated backend), ported one to one
as `examples/cut`. Its manifest is `parity/cut.json`; the web's screens are shot into
`parity/cut/web/` by `bun scripts/parity.ts web cut`.

## Where it stands

- The data is the web's to the value (`model.rs`, `seed.rs`, `media.rs`, `sound.rs`, `render.rs`,
  `history.rs`, `playback.rs`, `export.rs`, `backend.rs`, `editor.rs`, checked in `tests/web.rs`
  against the web's own modules run under bun): the projects and their edits, every edit a pure
  function with undo and redo over it, the playhead at its rates, the clips' scenes and sound, and
  the export's frames and words.
- The frame (`app.rs`): the rail with Projects, the five projects edited last under Recent and
  Settings, collapsed to its icons while a project is open (`Shell::collapsed`); the trail; New
  project in the top bar; the status line with the projects counted, or the open edit's length.
- The projects (`projects_page.rs`): a `MediaGrid` of wide `MediaCard`s (the media add-on), each
  with its edit's still a second in, its length, its clips and when it was edited, and its menu
  (Open, Rename, Duplicate, Delete); the empty state.
- The editor (`editor_page.rs`): the header with Undo and Redo (shut with their reason), Rename,
  Duplicate, Delete and Export; three splits as the web's (`cut-rows` down, `cut-bin`,
  `cut-inspector`). The edit is saved 600 ms after it last changed and once more as the editor
  closes. The keys are the web's: Space, J, K, L, the arrows (a second with Shift), Home and End,
  Mod-Z and Mod-Shift-Z, S, Delete and Backspace (Shift closes the gap), T, `=`, `-`, `0` and
  Escape; while the timeline has the focus the keys it answers itself are its own
  (`ClipTimelineResponse::focused`, new in the library).
- The bin (`bin_pane.rs`): the pictures under their stills and the sounds under their bars, each
  dragged onto a track of its kind (`DragData`), placed at the playhead when pressed or at the end
  of its track from its menu; Title adds a title at the playhead.
- The preview (`preview.rs`): the edit's frame at the playhead painted straight from the scenes,
  its titles at weight 700 as the web's canvas font; the scrub bar, the timecode, the transport,
  the rate chip while J or L play faster, and the level.
- The inspector (`inspector.rs`): the edit's summary; a clip's name, text, start, in and out points
  and duration as timecodes (`NumberField::format`, `parse` and `mono`, new in the library), its
  volume and what it is; several clips each with its start and length; Split and Delete.
- The timeline: `ClipTimeline` with moves, trims, splits, deletes, the tracks' toggles, drops from
  the bin, snap, and the zoom buttons over it.
- The dialogs (`dialogs.rs`): a new project's name, a rename, a delete; the export
  (`export_dialog.rs`) with the file's name, the resolution and what it would make.
- Settings over the page (`settings.rs`: playback, editing with the new title's length as a bare
  number field, `NumberField::bare` new in the library, the export's resolution, the look with
  the carried themes, the demo's data) and the palette (`palette.rs`).

## Open

- The bin's titles are cut later than the web's, and the panes' heads stand a few pixels taller.
- No export is made: the web encodes the frames and the mixed sound as WebM through WebCodecs, and
  no encoder is carried here. The dialog says what it would write and Export stays shut with why.
- No sound plays: the playhead's cues are let go, there being no audio output. The level and
  mute are kept but change nothing heard.
- The light preview setting is kept but changes nothing: the preview is painted as shapes at the
  pane's size, where the web draws a canvas of half the pixels.
- A time field changes the clip as each keystroke reads as a time, the keystrokes joined into one
  step of undo; whether the web's `TypedField` waits for the field to be left is not checked yet.
