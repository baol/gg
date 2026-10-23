//! Deterministic worker arrivals combined with real egui pointer and keyboard input.
use super::*;
use gg_engine::Commit;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn commit(row: usize) -> Commit {
    Commit {
        id: format!("{row:040x}"),
        parents: vec![],
        author: "Ada".into(),
        email: String::new(),
        time: String::new(),
        message: format!("Commit {row}\n\nFull body {row}"),
    }
}
fn update(request: stream::Request, count: usize) -> stream::Update {
    let rows = request
        .range
        .clone()
        .filter(|row| *row < count)
        .map(|row| (row, commit(row)))
        .collect();
    let selection = request
        .selected_row
        .or((count > 0).then_some(0))
        .filter(|row| *row < count)
        .map(|row| (row, commit(row)));
    stream::Update {
        query: request.query.clone(),
        request,
        anchor_row: None,
        identity: History {
            path: "repo".into(),
            head: "main".into(),
            commits: vec![],
            truncated: false,
            shallow: false,
        },
        count,
        scanned: count,
        complete: true,
        rows,
        graphs: vec![],
        refs: Default::default(),
        selection,
        error: None,
    }
}
struct Harness {
    app: Browser,
    ctx: egui::Context,
    driver: stream::Driver,
    time: f64,
}
impl Harness {
    fn new(count: usize) -> Self {
        let (worker, driver) = stream::Worker::controlled();
        let mut app = Browser::new("repo".into());
        app.state
            .accept(update(stream::Request::default(), count), true, true, None);
        app.worker = Some(worker);
        let mut h = Self {
            app,
            ctx: egui::Context::default(),
            driver,
            time: 1.0,
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        self.time += 0.1;
        let mut input = super::tests::frame_input(1120.0, 740.0, events);
        input.time = Some(self.time);
        self.app.prepare_input(&mut input);
        crate::tests::run_ui(&self.ctx, input, |ctx| self.app.draw_ui(ctx))
    }
    fn point(&self, row: usize) -> std::result::Result<egui::Pos2, String> {
        self.app
            .row_rects
            .iter()
            .find(|(index, _)| *index == row)
            .map(|(_, r)| egui::pos2(r.left() + 80.0, r.center().y))
            .ok_or_else(|| format!("Row {row} is not visible"))
    }
    fn publish(&self, count: usize) -> Result {
        self.driver
            .publish(Ok(update(self.driver.requested()?, count)))?;
        Ok(())
    }
}
fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn loading_row_stays_inert_through_press_and_becomes_clickable_after_reentry() -> Result {
    let mut h = Harness::new(100);
    h.app.state.rows.remove(&5);
    h.frame(vec![]);
    let point = h.point(5)?;
    h.frame(vec![egui::Event::PointerMoved(point)]);
    h.publish(100)?;
    h.frame(vec![button(point, true)]);
    assert!(!h.app.state.rows.contains_key(&5));
    h.frame(vec![button(point, false)]);
    assert_eq!(h.app.state.selected_row, Some(0));
    h.frame(vec![egui::Event::PointerMoved(h.point(6)?)]);
    assert!(h.app.state.rows.contains_key(&5));
    h.frame(vec![egui::Event::PointerMoved(point)]);
    h.frame(vec![button(point, true)]);
    h.frame(vec![button(point, false)]);
    assert_eq!(h.app.state.selected_row, Some(5));
    h.publish(100)?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.details.as_ref().map(|c| c.message.as_str()),
        Some("Commit 5\n\nFull body 5")
    );
    Ok(())
}

#[test]
fn scrollbar_drag_freezes_extent_and_release_preserves_offset() -> Result {
    let mut h = Harness::new(1000);
    let point = egui::pos2(
        h.app.history_rect.right() - 4.0,
        h.app.history_rect.top() + history_view::ROW_HEIGHT + 8.0,
    );
    h.frame(vec![egui::Event::PointerMoved(point)]);
    h.frame(vec![button(point, true)]);
    let moved = point + egui::vec2(0.0, 80.0);
    h.frame(vec![egui::Event::PointerMoved(moved)]);
    let offset = h.app.scroll_offset;
    assert!(
        offset > 100.0,
        "The input must actually drag egui's scrollbar: {offset}"
    );
    h.publish(100_000)?;
    h.frame(vec![]);
    assert_eq!(h.app.state.count, 1000);
    assert!((h.app.scroll_offset - offset).abs() < 0.1);
    h.frame(vec![button(moved, false)]);
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.state.count, 1000);
    h.frame(vec![egui::Event::PointerGone]);
    assert_eq!(h.app.state.count, 100_000);
    assert!((h.app.scroll_offset - offset).abs() < 0.1);
    Ok(())
}

#[test]
fn keyboard_navigation_rejects_late_details_and_loads_an_evicted_row() -> Result {
    let mut h = Harness::new(100_000);
    h.app.state.select_row(5000);
    h.app.scroll_to_selection = true;
    h.frame(vec![]);
    let old = update(h.driver.requested()?, 100_000);
    h.frame(vec![key(egui::Key::ArrowDown)]);
    assert_eq!(h.app.state.selected_row, Some(5001));
    h.driver.publish(Ok(old))?;
    h.frame(vec![]);
    assert_eq!(h.app.state.selected_row, Some(5001));
    assert!(h.app.state.details.is_none());
    h.publish(100_000)?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.selected.as_deref(),
        Some(commit(5001).id.as_str())
    );
    assert_eq!(
        h.app.state.details.as_ref().map(|c| c.message.as_str()),
        Some("Commit 5001\n\nFull body 5001")
    );
    assert!(h.app.row_rects.iter().any(|(row, _)| *row == 5001));
    let copied = h.frame(vec![egui::Event::Key {
        key: egui::Key::C,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    }]);
    assert!(copied.platform_output.commands.iter().any(
        |command| matches!(command, egui::OutputCommand::CopyText(id) if id == &commit(5001).id)
    ));
    Ok(())
}

#[test]
fn replacement_waits_for_pressed_row_and_retired_worker_cannot_overwrite_it() -> Result {
    let mut h = Harness::new(100);
    let point = h.point(5)?;
    h.frame(vec![egui::Event::PointerMoved(point)]);
    h.frame(vec![button(point, true)]);
    let (pending, next) = stream::Worker::controlled();
    h.app.pending = Some(pending);
    let mut replacement = update(stream::Request::default(), 20);
    replacement.identity.path = "other-repo".into();
    next.publish(Ok(replacement))?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.history.as_ref().map(|h| h.path.as_path()),
        Some(std::path::Path::new("repo"))
    );
    h.frame(vec![button(point, false)]);
    assert_eq!(h.app.state.selected_row, Some(5));
    h.frame(vec![egui::Event::PointerGone]);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    assert_eq!(
        h.app.state.history.as_ref().map(|h| h.path.as_path()),
        Some(std::path::Path::new("other-repo"))
    );
    h.publish(500)?;
    h.frame(vec![]);
    assert_eq!(h.app.state.count, 20);
    Ok(())
}

fn edit_filter(h: &mut Harness, value: &str) {
    let command = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    };
    h.frame(vec![command(egui::Key::F)]);
    h.frame(vec![command(egui::Key::A), egui::Event::Text(value.into())]);
}

#[test]
fn rapid_filters_reject_superseded_results_even_when_query_text_repeats() -> Result {
    let mut h = Harness::new(100);
    edit_filter(&mut h, "alpha");
    assert_eq!(h.app.state.query, "alpha");
    let old = update(h.driver.requested()?, 31);
    edit_filter(&mut h, "beta");
    edit_filter(&mut h, "alpha");
    h.driver.publish(Ok(old))?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.count, 0,
        "A previous alpha search belongs to an older edit"
    );
    h.publish(7)?;
    h.frame(vec![]);
    assert_eq!(h.app.state.count, 7);
    Ok(())
}

#[test]
fn refresh_preserves_commit_anchor_and_fractional_row_offset() -> Result {
    let mut h = Harness::new(100);
    h.app.restore_offset = Some(40.0 * history_view::ROW_HEIGHT + 7.0);
    h.frame(vec![]);
    let previous = h.app.scroll_offset;
    let (pending, next) = stream::Worker::controlled();
    h.app.pending = Some(pending);
    let request = stream::Request {
        range: 35..100,
        anchor_id: Some(commit(40).id),
        selected_id: h.app.state.selected.clone(),
        ..Default::default()
    };
    let mut replacement = update(request, 103);
    replacement.anchor_row = Some(43);
    replacement.rows = (35..100).map(|row| (row, commit(row - 3))).collect();
    replacement.selection = Some((3, commit(0)));
    next.publish(Ok(replacement))?;
    h.frame(vec![]);
    assert!((h.app.scroll_offset - previous - 3.0 * history_view::ROW_HEIGHT).abs() < 0.1);
    assert_eq!(
        h.app.state.rows.get(&43).map(|c| &c.id),
        Some(&commit(40).id)
    );
    assert_eq!(h.app.state.selected.as_deref(), Some(commit(0).id.as_str()));
    Ok(())
}

#[test]
fn failed_switch_keeps_displayed_identity_and_retry_adopts_new_repository() -> Result {
    let mut h = Harness::new(100);
    let (pending, failed) = stream::Worker::controlled();
    h.app.state.begin("other".into());
    h.app.pending = Some(pending);
    failed.publish(Err("Could not open other".into()))?;
    h.publish(120)?;
    h.frame(vec![]);
    assert_eq!(h.app.state.error.as_deref(), Some("Could not open other"));
    assert_eq!(
        h.app.state.history.as_ref().map(|h| h.path.as_path()),
        Some(std::path::Path::new("repo"))
    );
    let (pending, retry) = stream::Worker::controlled();
    h.app.state.begin("other".into());
    h.app.pending = Some(pending);
    let mut replacement = update(stream::Request::default(), 20);
    replacement.identity.path = "other".into();
    retry.publish(Ok(replacement))?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.history.as_ref().map(|h| h.path.as_path()),
        Some(std::path::Path::new("other"))
    );
    assert!(h.app.state.error.is_none());
    assert!(h.app.state.requested.is_none());
    Ok(())
}

#[test]
fn filter_edit_during_refresh_survives_late_replacement() -> Result {
    let mut h = Harness::new(100);
    let (pending, next) = stream::Worker::controlled();
    h.app.pending = Some(pending);
    let stale = update(stream::Request::default(), 100);
    edit_filter(&mut h, "new query");
    next.publish(Ok(stale))?;
    h.frame(vec![]);
    assert_eq!(h.app.state.query, "new query");
    assert_eq!(h.app.state.count, 0);
    let fresh = update(next.requested()?, 3);
    next.publish(Ok(fresh))?;
    h.frame(vec![]);
    assert_eq!(h.app.state.query, "new query");
    assert_eq!(h.app.state.count, 3);
    Ok(())
}

#[test]
fn scrollbar_release_followed_by_pointer_motion_in_one_frame_keeps_release_position() -> Result {
    let mut h = Harness::new(1000);
    let point = egui::pos2(
        h.app.history_rect.right() - 4.0,
        h.app.history_rect.top() + history_view::ROW_HEIGHT + 8.0,
    );
    h.frame(vec![egui::Event::PointerMoved(point)]);
    h.frame(vec![button(point, true)]);
    let moved = point + egui::vec2(0.0, 80.0);
    h.frame(vec![egui::Event::PointerMoved(moved)]);
    let offset = h.app.scroll_offset;
    assert!(offset > 100.0);
    h.frame(vec![
        button(moved, false),
        egui::Event::PointerMoved(egui::pos2(1000.0, 20.0)),
    ]);
    assert!(
        (h.app.scroll_offset - offset).abs() < 0.1,
        "Release offset changed from {offset} to {}",
        h.app.scroll_offset
    );
    h.frame(vec![]);
    assert!((h.app.scroll_offset - offset).abs() < 0.1);
    assert_eq!(
        h.ctx.input(|i| i.pointer.hover_pos()),
        Some(egui::pos2(1000.0, 20.0))
    );
    Ok(())
}

#[test]
fn keyboard_on_visible_rows_preserves_viewport() -> Result {
    // Selection changes should preserve spatial context when both rows already fit.
    let mut h = Harness::new(100_000);
    h.app.restore_offset = Some(4000.0);
    h.frame(vec![]);
    h.publish(100_000)?;
    h.frame(vec![]);
    h.app.state.select_row(156);
    let before = h.point(160)?;
    for _ in 0..8 {
        h.frame(vec![key(egui::Key::ArrowDown)]);
        assert_eq!(h.point(160)?, before);
        h.frame(vec![]);
        assert_eq!(h.point(160)?, before);
    }
    Ok(())
}

#[test]
fn keyboard_last_page_has_stable_geometry_between_frames() -> Result {
    // Inspect painted geometry: scroll state alone can hide a transient overshoot.
    let mut h = Harness::new(100_000);
    h.app.state.select_row(99_999);
    h.app.scroll_to_selection = true;
    h.frame(vec![]);
    h.publish(100_000)?;
    h.frame(vec![]);
    let before = h.point(99_999)?;
    let offset = h.app.scroll_offset;
    for direction in [egui::Key::ArrowUp, egui::Key::ArrowDown].repeat(4) {
        h.frame(vec![key(direction)]);
        assert_eq!(h.point(99_999)?, before);
        assert_eq!(h.app.scroll_offset, offset);
        for _ in 0..3 {
            h.frame(vec![]);
            assert_eq!(h.point(99_999)?, before);
            assert_eq!(h.app.scroll_offset, offset);
        }
    }
    Ok(())
}

#[test]
fn keyboard_details_show_current_summary_until_message_arrives() -> Result {
    let mut h = Harness::new(100);
    h.app.state.rows.get_mut(&1).ok_or("missing row")?.message = "Commit 1".into();
    h.frame(vec![key(egui::Key::ArrowDown)]);
    assert!(h.app.state.details.is_none());
    assert_eq!(
        h.app.state.displayed_details().map(|c| c.id.clone()),
        Some(commit(1).id)
    );
    assert_eq!(
        h.app.state.displayed_details().map(|c| c.message.as_str()),
        Some("Commit 1")
    );
    h.publish(100)?;
    h.frame(vec![]);
    assert_eq!(
        h.app.state.displayed_details().map(|c| c.message.as_str()),
        Some("Commit 1\n\nFull body 1")
    );
    Ok(())
}

fn modified_key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn navigation_bindings_share_page_size_bounds_and_stable_reveal() -> Result {
    // All bindings use the same selection contract, including pages beyond cached rows.
    for (down, up, modifiers, paged) in [
        (egui::Key::J, egui::Key::K, egui::Modifiers::NONE, false),
        (egui::Key::N, egui::Key::P, egui::Modifiers::CTRL, false),
        (
            egui::Key::PageDown,
            egui::Key::PageUp,
            egui::Modifiers::NONE,
            true,
        ),
        (
            egui::Key::PageDown,
            egui::Key::V,
            egui::Modifiers::NONE,
            true,
        ),
    ] {
        let mut h = Harness::new(100_000);
        let step = if paged {
            h.app.page_rows.saturating_sub(1).max(1)
        } else {
            1
        };
        h.frame(vec![modified_key(down, modifiers)]);
        assert_eq!(h.app.state.selected_row, Some(step));
        let up_modifiers = if up == egui::Key::V {
            egui::Modifiers::ALT
        } else {
            modifiers
        };
        h.frame(vec![modified_key(up, up_modifiers)]);
        assert_eq!(h.app.state.selected_row, Some(0));
        h.frame(vec![modified_key(up, up_modifiers)]);
        assert_eq!(h.app.state.selected_row, Some(0));
        h.app.state.select_row(99_998);
        h.frame(vec![modified_key(down, modifiers)]);
        assert_eq!(h.app.state.selected_row, Some(99_999));
        let position = h.point(99_999)?;
        h.frame(vec![]);
        assert_eq!(h.point(99_999)?, position);
        h.frame(vec![modified_key(down, modifiers)]);
        assert_eq!(h.app.state.selected_row, Some(99_999));
        assert_eq!(h.point(99_999)?, position);
    }
    Ok(())
}

#[test]
fn navigation_bindings_preserve_text_focus_and_empty_selection() {
    for count in [0, 100] {
        let mut h = Harness::new(count);
        if count > 0 {
            h.ctx
                .memory_mut(|m| m.request_focus(egui::Id::unique("history-filter")));
            h.frame(vec![]);
        }
        let selected = h.app.state.selected_row;
        for (key, modifiers) in [
            (egui::Key::J, egui::Modifiers::NONE),
            (egui::Key::K, egui::Modifiers::NONE),
            (egui::Key::N, egui::Modifiers::CTRL),
            (egui::Key::P, egui::Modifiers::CTRL),
            (egui::Key::V, egui::Modifiers::CTRL),
            (egui::Key::V, egui::Modifiers::ALT),
            (egui::Key::PageDown, egui::Modifiers::NONE),
            (egui::Key::PageUp, egui::Modifiers::NONE),
        ] {
            h.frame(vec![modified_key(key, modifiers)]);
            assert_eq!(h.app.state.selected_row, selected);
        }
    }
}

#[test]
fn narrow_details_navigation_preserves_hidden_history_and_back_anchor() -> Result {
    let mut h = Harness::new(1000);
    h.app.restore_offset = Some(4007.0);
    h.frame(vec![]);
    h.publish(1000)?;
    h.frame(vec![]);
    h.app.state.select_row(160);
    h.publish(1000)?;
    h.frame(vec![]);
    let frame = |h: &mut Harness, events| {
        h.time += 0.1;
        let mut input = super::tests::frame_input(720.0, 480.0, events);
        input.time = Some(h.time);
        h.app.prepare_input(&mut input);
        crate::tests::run_ui(&h.ctx, input, |ctx| h.app.draw_ui(ctx))
    };
    frame(&mut h, vec![]);
    let offset = h.app.scroll_offset;
    let selected = h.app.state.selected.clone();
    let query = h.app.state.query.clone();
    frame(&mut h, vec![key(egui::Key::Space)]);
    assert!(h.app.narrow_details);
    for event in [
        key(egui::Key::ArrowDown),
        key(egui::Key::J),
        key(egui::Key::PageDown),
        modified_key(egui::Key::N, egui::Modifiers::CTRL),
    ] {
        frame(&mut h, vec![event]);
        assert_eq!(
            h.app.state.selected, selected,
            "details navigation changed hidden history"
        );
    }
    frame(&mut h, vec![key(egui::Key::Escape)]);
    assert!(!h.app.narrow_details);
    assert_eq!(h.app.state.selected, selected);
    assert_eq!(h.app.state.query, query);
    assert_eq!(h.app.scroll_offset, offset);
    Ok(())
}

#[test]
fn partial_history_error_retains_identity_rows_and_navigation() -> Result {
    let mut h = Harness::new(64);
    let mut failed = update(h.driver.requested()?, 64);
    failed.complete = false;
    failed.error = Some("Could not read repo: missing commit".into());
    h.driver.publish(Ok(failed))?;
    h.frame(vec![]);
    assert!(!h.app.state.finished);
    assert_eq!(h.app.state.count, 64);
    assert!(
        h.app
            .state
            .error
            .as_deref()
            .is_some_and(|e| e.contains("missing commit"))
    );
    assert_eq!(
        h.app.state.history.as_ref().map(|r| r.path.as_path()),
        Some(std::path::Path::new("repo"))
    );
    h.frame(vec![key(egui::Key::ArrowDown)]);
    assert_eq!(h.app.state.selected, Some(commit(1).id));
    assert!(h.app.state.displayed_details().is_some());
    assert!(!h.app.state.finished);
    Ok(())
}

#[test]
fn review_navigation_and_late_history_preserve_history_until_back() -> Result {
    let mut h = Harness::new(1000);
    h.app.restore_offset = Some(4007.0);
    h.frame(vec![]);
    h.publish(1000)?;
    h.frame(vec![]);
    h.app.state.select_row(160);
    h.publish(1000)?;
    h.frame(vec![]);
    let selected = h.app.state.selected.clone();
    let offset = h.app.scroll_offset;
    let query = h.app.state.query.clone();
    h.app.review = Some(crate::review_view::Review::fixture(&h.ctx));
    h.frame(vec![]);
    h.publish(2000)?;
    h.frame(vec![key(egui::Key::J)]);
    assert_eq!(h.app.review.as_ref().and_then(|r| r.selected), Some(1));
    assert_eq!(h.app.state.selected, selected);
    assert_eq!(h.app.scroll_offset, offset);
    assert_eq!(h.app.state.count, 1000);
    h.frame(vec![key(egui::Key::Escape)]);
    assert!(h.app.review.is_none());
    h.frame(vec![]);
    assert_eq!(h.app.state.selected, selected);
    assert_eq!(h.app.scroll_offset, offset);
    assert_eq!(h.app.state.query, query);
    assert_eq!(h.app.state.count, 2000);
    Ok(())
}

#[test]
fn commit_draft_survives_history_navigation() -> Result {
    let mut h = Harness::new(100);
    h.app.open_working(&h.ctx);
    h.app
        .working
        .as_mut()
        .ok_or("missing working view")?
        .set_draft("Subject line\n\nBody paragraph".into());
    h.app.close_to_history(&h.ctx);
    assert!(h.app.working.is_none());
    h.app.open_working(&h.ctx);
    assert_eq!(
        h.app
            .working
            .as_ref()
            .ok_or("missing working view")?
            .draft(),
        "Subject line\n\nBody paragraph"
    );
    Ok(())
}

#[test]
fn commit_draft_does_not_cross_worktrees() -> Result {
    let mut h = Harness::new(100);
    h.app.state.history.as_mut().ok_or("missing history")?.path = "worktree-a".into();
    h.app.open_working(&h.ctx);
    h.app
        .working
        .as_mut()
        .ok_or("missing working view")?
        .set_draft("draft for a".into());
    h.app.close_to_history(&h.ctx);

    h.app.state.history.as_mut().ok_or("missing history")?.path = "worktree-b".into();
    h.app.open_working(&h.ctx);
    assert_eq!(
        h.app
            .working
            .as_ref()
            .ok_or("missing working view")?
            .draft(),
        ""
    );
    h.app.close_to_history(&h.ctx);

    h.app.state.history.as_mut().ok_or("missing history")?.path = "worktree-a".into();
    h.app.open_working(&h.ctx);
    assert_eq!(
        h.app
            .working
            .as_ref()
            .ok_or("missing working view")?
            .draft(),
        "draft for a"
    );
    Ok(())
}

#[test]
fn status_back_preserves_history_selection_and_anchor() {
    let mut h = Harness::new(1000);
    h.app.state.select_row(900);
    h.app.scroll_to_selection = true;
    h.frame(vec![]);
    let selected = h.app.state.selected.clone();
    let offset = h.app.scroll_offset;
    h.app.working = Some(status_view::StatusView::new(
        "missing-status-fixture".into(),
        &h.ctx,
    ));
    h.frame(vec![]);
    assert_eq!(h.app.state.selected, selected);
    h.frame(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert!(h.app.working.is_none());
    h.frame(vec![]);
    assert_eq!(h.app.state.selected, selected);
    assert!((h.app.scroll_offset - offset).abs() < 1.0);
}
