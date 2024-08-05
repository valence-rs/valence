# `dump_schedule`

A simple debugging utility for visualizing Valence's schedule graph. Generates a SVG file.

1. Ensure that [Graphviz](https://graphviz.org/) is installed and the `dot` and `tred` commands are available.
2. Run the program with `cargo r -p dump_schedule -- PostUpdate`
3. Open the generated `graph.svg` in your browser or other program, e.g. `chromium graph.svg`.
