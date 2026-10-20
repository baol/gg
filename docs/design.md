# Design guidelines

Purpose: define shared layout and interaction rules.

- Revisit [Fork's screenshots](https://fork.dev/) during layout and interaction refinements. Use them as references for density, hierarchy, alignment, and readable diffs while following gg's CLI-first workflow and milestone scope. Link to external references; commit original gg visuals and assets with compatible licenses and required attribution.
- Reserve compact, stable slots for text, controls, images, and loading states.
- Embed the root `gg.png` as the native window icon and toolbar brand. Keep the toolbar image in a fixed 30-pixel slot, with its aspect ratio and “gg · good git” accessible label.
- Filter thumbnails to their displayed physical pixel size; regenerate on display-scale changes. Preserve transparent edges and cache list/detail sizes from one source image.
- Keep fields on one line. Expose overflow through clickable `...` and full-text tooltips.
- Open complete text in a separate scrollable, selectable view with Copy. Preserve underlying selection and position.
- Use explicit reading views for multiline content; keep summaries bounded.
- Share column geometry and clip each cell. Verify long text, Unicode, and display scales in memory.
- Choose semantic colors for each theme. In light mode, pair deep text colors with pale diff and selection fills; keep text contrast at least 4.5:1 on its background. Preserve symbols and labels alongside color. Check both themes, including selected rows.
- Reveal keyboard selection with one minimal, bounded scroll adjustment. Preserve visible row positions between input and idle frames.

Screen layouts and streaming contracts: [architecture](architecture.md).
