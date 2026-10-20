# Design guidelines

Purpose: define shared layout and interaction rules.

- Revisit [Fork's screenshots](https://fork.dev/) during layout and interaction refinements. Use them as references for density, hierarchy, alignment, and readable diffs while following gg's CLI-first workflow and milestone scope. Link to external references; commit original gg visuals and assets with compatible licenses and required attribution.
- Reserve compact, stable slots for text, controls, images, and loading states.
- Keep fields on one line. Expose overflow through clickable `...` and full-text tooltips.
- Open complete text in a separate scrollable, selectable view with Copy. Preserve underlying selection and position.
- Use explicit reading views for multiline content; keep summaries bounded.
- Share column geometry and clip each cell. Verify long text, Unicode, and display scales in memory.
- Reveal keyboard selection with one minimal, bounded scroll adjustment. Preserve visible row positions between input and idle frames.

Screen layouts and streaming contracts: [architecture](architecture.md).
