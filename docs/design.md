# Design guidelines

Purpose: define shared visual and interaction rules.

- Reserve field widths and heights for readable, compact content and stable controls.
- Keep text fields on one line within their allocated bounds. Replace overflow with a clickable `...`; show the complete text on hover.
- Open overflow in a separate, scrollable, selectable view with a copy action. Preserve the underlying layout, selection, and scroll position.
- Present multiline documents only in an explicitly opened reading view. Keep their summaries bounded.
- Share column geometry, clip drawing to each cell, and verify long text, Unicode, and supported display scales with in-memory input tests.

Browser layout and streaming behavior: [architecture](architecture.md).
