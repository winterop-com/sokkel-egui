# Logs: a live log viewer across Tidevann's services

An app of sokkel-ui-egui's own (`examples/logs`, `make logs`), not a port: what a reader of
Tidevann's logs needs, drawn with the library's public parts alone. A part it lacks goes into the
library, never into the app.

## What it does

- Every service writes lines as the board's clock runs, a reading every 2 s while live
  (`stream.rs`: a seeded generator, so a run reads the same every time): requests on web and api,
  runs and their steps on worker, queued jobs on scheduler, disks on storage, the index on search,
  digests on mail; mostly info, now and then a warning or an error. The viewer keeps the last 2000.
- The rail chooses all services or one, each service counting its errors. The top bar pauses and
  resumes the stream; the status line says when the last line came, the lines and the errors.
- The page: its head counting the lines shown, a toolbar with the search (`SearchField`, reached
  by `/`, every word in the line's service, host, message or trace) and the level (`Segmented`
  with each level's count and dot), then a `SplitPane`: the lines newest first in a `DataTable`
  (time, level as its chip, service as a tag, the message), stacking as labeled cards below 520,
  and beside them the line pressed, its fields in a `DescriptionList`, or an `Empty` saying to
  pick one.

## What it brought into the library

- `SearchField` takes 256 in a row such as a toolbar, the room it is given down a column, or a
  width of its own, so a toolbar's end stands beside it rather than over it.

## Next

- Follow the stream's foot as it grows, as a terminal does, with a way back to it when scrolled.
- A line's trace opening every line that carries it; a time range; saved searches.
