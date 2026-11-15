# The dashboard example's log grows to its lines

For sokkel-ui's dashboard example (`examples/dashboard/web/src/widgets.tsx`, the `log` case), seen
at sokkel-ui `a86307f6` while laying egui's port beside it.

The log is given `className="h-72 min-h-0 flex-1"`. Inside the panel's column flexbox,
`flex-1` (a basis of 0, growing) wins over `h-72`, and the panel's height is its content's, so the
box is as tall as its lines: on Operations, at 1440 wide, it measures 816 (40 lines and its
padding), scrolls nowhere, and shows its first line at the top. Lines keep arriving, up to 200, so
the panel keeps growing, to about 4,000 tall, and `follow` has nothing to follow.

What the classes ask for reads as a log 288 tall (`h-72`) that may grow with its row and scrolls
past that, keeping the newest line in view. Egui draws that. If it is what sokkel-ui means, the
example could cap the box (for example `h-72 flex-none`, or `max-h-72` beside `flex-1`) so the
panel stays its own height; if the growing log is meant, egui follows it.
