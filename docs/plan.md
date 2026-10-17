# Plan

Purpose: sequence the implementation steps for the current and next milestone.

Product entry and usage: [README](../README.md). Command contracts: [architecture](architecture.md). Milestones and release gates: [roadmap](../ROADMAP.md).

## Delivery sequence

1. Define typed engine requests and results.
2. Implement Git access with synthetic fixtures and independent Git oracles.
3. Connect terminal commands and native views to the same behavior.
4. Verify errors, responsiveness, and rendering; record evidence.
5. Update milestone status.

## M0 · Delivered

Three-crate workspace, bounded history adapter, background loading, virtualized browser, filtering, details, copy-ID, JSON command, and native CI. Evidence: [verification](verification.md).

## M1 · Next

1. Define commit-tree and blob-diff requests.
2. Compare roots against an empty tree and merges against a selected parent.
3. Represent text, binary changes, renames, and truncation explicitly.
4. Load selected diffs on demand in terminal and native workflows.
5. Compare outputs and measure responsiveness with large files.

## Performance

Measure startup, history loading, and filtering on representative packed repositories before expanding the history window. Record hardware, repository size, and timings.
