# Planner: Tidevann's jobs on a week calendar

An app of sokkel-ui-egui's own (`examples/planner`, `make planner`), not a port: when Tidevann's
jobs run, drawn with the library's public parts alone. A part it lacks goes into the library,
never into the app.

## What it does

- Five jobs, each with its schedule: nightly-export every day at 02:00, sync-inventory every six
  hours at :30, rebuild-index Monday and Thursday at 03:00, send-digest on weekdays at 07:00 and
  import-orders on weekdays every two hours from 08:00 to 18:00. Their runs fill the week shown;
  it is Wednesday 18 November 2026 at 09:00. The runs before now say how they went (now and then
  one failed), the rest are scheduled.
- The page: its head with the week and the runs shown, the week before and the next one, and
  This week. Then a `SplitPane`: the `WeekGrid` (the days side by side, the hours running down,
  today tinted and the time now a line across it), and beside it the jobs, each one ticked to be
  shown, and the run pressed: its state as a chip, when, its project, its owner and its schedule
  in a `DescriptionList`, and Skip this run for one ahead, which fades it on the week.
- The status line counts the runs scheduled and the ones failed.

## What it brought into the library

- `WeekGrid`, `CalendarEvent` and `layout_day`, after sokkel-ui's calendar: runs at the same time
  split their day as sokkel-ui's `layoutDay` splits them, a muted event is faded, the hours scroll
  under the heads to the hour asked for with its label whole. Its story is Patterns/Calendar.

## Next

- A failed run is told only when it is opened: sokkel-ui's `CalendarEvent` has no state of its
  own, so a mark for it would be a question for sokkel-design first.
- The month (`MonthGrid`), the day (`DayAgenda`) and moving a run by dragging it, as sokkel-ui's
  Apps/Calendar story does; that story is the next port for the calendar.
