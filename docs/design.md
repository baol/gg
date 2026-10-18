# Design guidelines

Purpose: define shared layout and interaction rules.

- Reserve compact, stable slots for text, controls, images, and loading states.
- Keep fields on one line. Expose overflow through clickable `...` and full-text tooltips.
- Open complete text in a separate scrollable, selectable view with Copy. Preserve underlying selection and position.
- Use explicit reading views for multiline content; keep summaries bounded.
- Share column geometry and clip each cell. Verify long text, Unicode, and display scales in memory.
- Reveal keyboard selection with one minimal, bounded scroll adjustment. Preserve visible row positions between input and idle frames.

Screen layouts and streaming contracts: [architecture](architecture.md).
