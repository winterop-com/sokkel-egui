# Models: sokkel-ui's models example in egui

sokkel-ui's models example (Tidevann's local model runner: the library of models on the disk, what
they take on it, discovering and pulling models from a public hub and the runners on the machine,
a chat with a model, one prompt compared across several models, and the runner that holds models
in memory, on a simulated backend), ported one to one as `examples/models`. Its manifest is
`parity/models.json`; the web's screens are shot into `parity/models/web/` by
`bun scripts/parity.ts web models`.

## Where it stands

- The data is the web's to the value (`data/*`, `engine/*`, `actions.rs`, `app.rs` and each page's
  words and logic in `pages/*`, checked in `tests/web.rs` against the web's own modules run under
  bun), on the example's own held clock, timers and promises (`runtime.rs`). A library view's state
  is now the core's `ListViewState`, kept as the web keeps it, where the example carried a copy.
- The frame (`workbench.rs`): the rail (Library: Models, Disk usage; Get models: Discover, Downloads
  with the pulls under way counted; Use: Chat, Compare; Run: Runner with the models in memory
  counted; System: Settings); the account menu; the top bar's trail (a model's name in the code
  face, a conversation's title, `Comparison` or `New`) with its links, the palette's button and the
  runner's server menu (Running, Stopped, Starting or Stopping, its address and uptime, Check again
  logging the probe 600 ms on, Stop or Start the runner, Open the runner); the status line (the
  runner's uptime, the pulls under way or the runner stopped, the models loaded and what is on the
  disk). The address is the web's hash route: `library[/<id>]`, `disk`, `discover`, `downloads`,
  `chat[/<id>]`, `compare[/<id>|new]`, `runner`, and `settings` over the page; anything else is Not
  found. The backend answers on the app's clock, 150 ms after each call; the routes the engines go
  to, the pulls that land (a toast with Open) and the promises the pages wait on are heard each
  frame.
- The pages (`screens.rs`, `talk.rs`): the library as a table with its search, filter bar, saved
  views (picked, saved, saved as new, renamed, deleted), selection with Delete N, cards view, columns
  menu and each row's menu (chat with it, load or unload, delete); a model with its details and
  tags, the path with its copy button, its files largest first, its card, the defaults its chats
  start with (temperature, top p, max tokens, the system prompt kept when the field is left, Reset
  to the app's defaults) and what it takes in use, with Delete (asked first, saying the room freed,
  the files and the conversations that use it), Load or Unload, and Chat; the disk with its four
  tiles, the whole disk as a stacked bar, the models by source as a ring and by format as bars,
  every model as a treemap, and the models kept twice with Delete all but the smallest; Discover's
  search over the sources (220 ms after the last change, then the sources' answer), the source and
  format choices, each entry's pull button through its states and its sheet with the variants and
  their pulls; the downloads with their tiles, under way and ended groups, progress, speed, time
  left, each pull's control and menu, and Clear ended; the chat beside its conversations (pinned,
  today and earlier, each renamed, pinned or deleted), the question's bubble with its reactions,
  each answer streaming token by token with its usage line, Copy, Regenerate, the thumbs and the
  versions, the inline composer with the model chosen in its tools, the suggestions of a new
  conversation, and the settings panel (the model and what it is, temperature, top p, max tokens,
  the system prompt with its presets, back to the model's defaults); the comparison with the
  earlier ones, the prompt's models (added, changed, taken away), temperature, max tokens and the
  composer that runs it, what was asked with Edit and Run all again or Stop all, and a column per
  model with its state, reasoning, answer and usage, each stopped, run again or taken away; the
  runner with its tiles, the memory taken, the loaded models (load one, chat with it, unload), its
  health and its log by source and search.
- Settings: Library (the root, moved once it reads as a folder path, and asking before deleting),
  Chat (the default model), Downloads (pulls at a time, bandwidth), Runner (memory, keep-alive,
  starting with the app, loading on demand), Appearance with the carried themes, and Demo data's
  reset (asked first); each section's Reset to defaults. The palette: the pages, New chat, Pull a
  model, Compare models, Stop or Start the runner, Clear ended downloads, every model, a chat with
  each, every conversation, and the look. The backend's rows and the values the web keeps in the
  browser (the chat's drafts, panel and history, the comparison's draft, Discover's search) are
  saved with eframe's storage.
- New in the library: the chat parts in the core (`widgets/chat.rs`: `Composer`, inline and
  stacked after the sokkel-design spec; `ConversationHistory` with `conversation_group` and
  `conversation_time`; `AssistantReply`; `UsageLine`; `Bubble`; `ReactionChips`; `AnswerColumn`;
  `Conversation`), `IconButton::pressed` and `enabled`, and fenced code in `Markdown` colored by its
  language. `Progress` and `CheckList` (`widgets/progress.rs`). The table's `Cell::Badge` (a quiet
  badge in the code face), `Cell::Slot` with `DataTable::slot` (a cell the app draws: a pull's
  button, a progress bar) and `DataTable::bulk` (the selection bar's actions). `Stat::chip`, a
  state where a tile's number stands. `Select::quiet`, and `Select::small` keeping the body's words
  as the select spec has it (only its height changes). The shell's toggle that opens a shut side
  panel, at the top bar's end. `ListViewState` gained `order` and `widths` and is kept as the web
  keeps it (the sort as `{ column, direction }`), and `FilterOperator` is named as the web names it
  (`atMost`). `DownloadButtonWords::share`, what a running download with a share says (a queued
  pull's `Queued`). In the charts add-on: `TreemapPanel` with `tree_of` and `squarify`. Fixed: the
  table's bar stands the view's toggle before the columns' menu, as the web's; the status line's
  items take up to 70% of the line, cut short only where they would pass it, where each was cut at
  120; the conversation history's times are measured in the face they are drawn in. Stories:
  Primitives/Chat/Bubble, Patterns/Usage line, Assistant reply, Answer column, Composer,
  Conversation history, Chat, Primitives/Display/Progress, Patterns/Check list, Patterns/Stat
  state, Primitives/Inputs/Select quiet, Patterns/Data table cells, Charts/Treemap.

## Open

- The composer's attach button opens no file picker: egui has none without a native dialog, so
  files cannot be sent; the attachments a conversation already holds are listed by name over the
  question rather than as the web's attachment cards.
- The comparison's models are not dragged into another order: each stands with its grip, but the
  grip does not move it.
- A select lists its options without the web's format groups and their labels.
- The treemap is the add-on's own squarified layout, not ECharts': its bands and colors differ.
- As in every example, a table's first column is not ruled off from the others as the web's is, and
  a table with only hideable columns shows its row count in the bar where the web shows the
  columns' button alone (Downloads).
