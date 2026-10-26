# Design guidelines

Purpose: define shared layout and interaction rules.

- Use external references for density, hierarchy, alignment, and readable diffs while following gg's CLI-first workflow and milestone scope. Link to external references; commit original gg visuals and assets with compatible licenses and required attribution.
- Group repository actions in **Repository** and presentation options in **View**. Keep primary navigation visible; place file actions beside the file list.
- Reserve compact, stable slots for text, controls, images, and loading states.
- Embed the root `gg.png` as the native window icon and toolbar brand. Keep the toolbar image in a fixed 30-pixel slot, with its aspect ratio and “gg · good git” accessible label.
- Filter thumbnails to their displayed physical pixel size; regenerate on display-scale changes. Preserve transparent edges and cache list/detail sizes from one source image.
- Keep fields on one line. Expose overflow through clickable `...` and full-text tooltips.
- Open complete text in a separate scrollable, selectable view with Copy. Preserve underlying selection and position.
- Use bundled-font glyphs for interface labels; spell out navigation keys and comparison direction. Verify labels against both font families.
- Use explicit reading views for multiline content; keep summaries bounded.
- Share column geometry and clip each cell. Verify long text, Unicode, and display scales in memory.
- Choose semantic colors for each theme. In light mode, pair deep text colors with pale diff and selection fills; keep text contrast at least 4.5:1 on its background. Preserve symbols and labels alongside color. Check both themes, including selected rows.
- Reveal keyboard selection with one minimal, bounded scroll adjustment. Preserve visible row positions between input and idle frames.

Screen layouts and streaming contracts: [architecture](architecture.md).

Journey review and proposed M7 interfaces: [user flows](user-flows.md) and [interactive mockups](ui-mockups.html). Proposals do not change shipped behavior or milestone status.

Proposed consolidation across views: [coherent interaction model](ui-coherence.md), including the shared reader and the requested Local Changes layout. Increments 1–5 are complete; increment 6 is in progress; later increments remain proposals. Update the owning architecture contracts as each later increment is scheduled.

Retain the labeled displayed diff while a newly selected file loads. Replace identity, metadata, preview, and rows together after interaction settles. Keep loading and metadata slots fixed in height.
