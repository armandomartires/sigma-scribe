//! Replace in all notes (S24.1): the review sheet, the worker that plans,
//! applies and undoes, and the notice band with its Undo.
//!
//! With the Find bar in All notes, the All key plans the replace on a worker
//! (`sigma_core::replace::plan`, uncapped) and then shows the review sheet in
//! place of the results list. Apply and Undo run on the same worker, one
//! note at a time, and report progress ("Replacing… 2 of 3 notes"); while any
//! job runs every tap and key is swallowed. The open note is never written
//! by the worker: its new text comes back and goes into the buffer as one
//! undo step, then is saved.
//!
//! The sheet is cancelled by its Cancel button, Esc and Back; the results
//! list, query, scope and page are untouched underneath.

use crate::App;
use sigma_core::replace::{self, Applied, Outcome, Plan, Selection, Snapshot, Undone};
use sigma_core::{Library, NotePath};
use sigma_device::Refresh;
use sigma_render::i18n::tr;
use sigma_render::review::{note_tick_targets, review_line_matches};
use sigma_render::{ReviewHit, ReviewLayout, ReviewModel, Scope};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;

/// The review sheet as the Find bar holds it (S24.1).
#[derive(Clone, Debug)]
pub struct ReviewState {
    /// What the replace would change, scanned without a cap.
    pub plan: Arc<Plan>,
    /// Which matches are ticked.
    pub selection: Selection,
    /// The shown page of the sheet, 0-based.
    pub page: usize,
    /// `Some((done, total))` while the apply runs.
    pub progress: Option<(usize, usize)>,
}

/// What the notice band says after a replace or an undo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// A replace ran: matches and notes written, notes refused because they
    /// changed since the review, notes that could not be written.
    Replaced {
        /// Matches replaced.
        matches: usize,
        /// Notes written (the open note included).
        notes: usize,
        /// Notes refused: edited since the review.
        changed: usize,
        /// Notes whose write failed.
        failed: usize,
    },
    /// An undo ran: notes put back, notes left alone.
    Restored {
        /// Notes put back.
        notes: usize,
        /// Notes left as they are (edited since).
        left: usize,
    },
    /// The snapshot could not be made: nothing was touched.
    NotReplaced,
    /// The undo could not read its snapshot: nothing was touched.
    NotRestored,
}

impl Notice {
    /// The band's text in the current language: parts joined with " · ",
    /// each translated on its own.
    #[must_use]
    pub fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        match self {
            Self::Replaced {
                matches,
                notes,
                changed,
                failed,
            } => {
                let one = if *notes == 1 { "note" } else { "notes" };
                parts.push(tr(&format!("Replaced {matches} in {notes} {one}")).into_owned());
                if *changed > 0 {
                    parts.push(tr(&format!("{changed} changed since the review")).into_owned());
                }
                if *failed > 0 {
                    parts.push(tr(&format!("{failed} could not be written")).into_owned());
                }
            }
            Self::Restored { notes, left } => {
                let one = if *notes == 1 { "note" } else { "notes" };
                parts.push(tr(&format!("Restored {notes} {one}")).into_owned());
                if *left > 0 {
                    parts.push(tr(&format!("{left} left alone")).into_owned());
                }
            }
            Self::NotReplaced => parts.push(tr("Nothing was replaced").into_owned()),
            Self::NotRestored => parts.push(tr("Could not undo the replace").into_owned()),
        }
        parts.join(" \u{b7} ")
    }
}

/// Work for the [`Replacer`] thread.
enum Job {
    Plan {
        root: PathBuf,
        query: String,
        case: bool,
        with: String,
        open: Option<(NotePath, String)>,
    },
    Apply {
        plan: Arc<Plan>,
        selection: Selection,
        open: Option<(NotePath, String)>,
    },
    Undo {
        root: PathBuf,
        snapshot: Snapshot,
        open: Option<(NotePath, String)>,
    },
}

/// What the worker reports.
enum Msg {
    Planned(Plan),
    Progress(usize, usize),
    Applied(Applied),
    Undone(Undone),
    /// The worker thread died; it has been replaced.
    Lost,
}

/// Plans, applies and undoes a replace on a worker thread, one job at a
/// time, so the UI thread never walks the library or writes the notes.
pub struct Replacer {
    tx: Sender<Job>,
    rx: Receiver<Msg>,
    pending: bool,
    /// The open note and the hash of the text the last job was given: what
    /// the buffer must still hold for the job's answer to go into it.
    base: Option<(NotePath, u64)>,
}

impl Default for Replacer {
    fn default() -> Self {
        Self::new()
    }
}

/// The open note's text as a borrowed pair.
fn borrowed(open: &Option<(NotePath, String)>) -> Option<(&NotePath, &str)> {
    open.as_ref().map(|(p, t)| (p, t.as_str()))
}

impl Replacer {
    /// Starts the worker thread. No job runs until the app asks for one.
    ///
    /// # Panics
    /// If the OS cannot create the thread (from [`std::thread::spawn`]).
    #[must_use]
    pub fn new() -> Self {
        let (tx, jobs) = channel::<Job>();
        let (done, rx) = channel::<Msg>();
        thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let sent = match job {
                    Job::Plan {
                        root,
                        query,
                        case,
                        with,
                        open,
                    } => {
                        let lib = Library::new(&root);
                        let plan = replace::plan(&lib, &query, case, &with, borrowed(&open));
                        done.send(Msg::Planned(plan)).is_ok()
                    }
                    Job::Apply {
                        plan,
                        selection,
                        open,
                    } => {
                        let applied = replace::apply_with_progress(
                            &plan,
                            &selection,
                            borrowed(&open),
                            |d, t| {
                                let _ = done.send(Msg::Progress(d, t));
                            },
                        );
                        done.send(Msg::Applied(applied)).is_ok()
                    }
                    Job::Undo {
                        root,
                        snapshot,
                        open,
                    } => {
                        let lib = Library::new(&root);
                        let undone = replace::undo_with_open(&lib, &snapshot, borrowed(&open));
                        done.send(Msg::Undone(undone)).is_ok()
                    }
                };
                if !sent {
                    break;
                }
            }
        });
        Self {
            tx,
            rx,
            pending: false,
            base: None,
        }
    }

    /// Queues a job. While one is pending the app takes no input, so jobs
    /// never pile up. The open note's text, as the job gets it, is noted: the
    /// buffer can still change while the job runs (a sync reloads the note),
    /// and then its answer must not go into the buffer.
    fn request(&mut self, job: Job) {
        let open = match &job {
            Job::Plan { open, .. } | Job::Apply { open, .. } | Job::Undo { open, .. } => open,
        };
        self.base = open.as_ref().map(|(p, t)| (p.clone(), replace::hash(t)));
        self.pending = true;
        if self.tx.send(job).is_err() {
            // The worker is gone: start another and let `poll` say so.
            *self = Self::new();
            self.pending = false;
        }
    }

    /// A job has been asked for and has not finished.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Takes note of a message: a final answer ends the job.
    fn take(&mut self, m: Msg, out: &mut Vec<Msg>) {
        match m {
            Msg::Progress(..) => {
                // Only the newest progress matters.
                if matches!(out.last(), Some(Msg::Progress(..))) {
                    out.pop();
                }
            }
            _ => self.pending = false,
        }
        out.push(m);
    }

    /// What the worker has said since the last call, without blocking.
    fn poll(&mut self) -> Vec<Msg> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(m) => self.take(m, &mut out),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.pending {
                        *self = Self::new();
                        out.push(Msg::Lost);
                    }
                    break;
                }
            }
        }
        out
    }

    /// Blocks until the running job has answered (tests, emulator).
    fn wait(&mut self) -> Vec<Msg> {
        let mut out = self.poll();
        while self.pending {
            match self.rx.recv() {
                Ok(m) => self.take(m, &mut out),
                Err(_) => {
                    *self = Self::new();
                    out.push(Msg::Lost);
                }
            }
        }
        out
    }
}

/// Where the caret goes in the open note after `plan`'s ticked matches were
/// replaced: matches wholly before it shift it, a caret inside a match moves
/// to where that match starts.
fn caret_after(plan: &Plan, selection: &Selection, path: &NotePath, caret: usize) -> usize {
    let Some(i) = plan.notes.iter().position(|n| n.path == *path) else {
        return caret;
    };
    let with = plan.with.len() as isize;
    let mut delta: isize = 0;
    for (k, m) in plan.notes[i].matches.iter().enumerate() {
        if !selection.is_ticked(i, k) {
            continue;
        }
        if m.to <= caret {
            delta += with - (m.to - m.from) as isize;
        } else {
            if m.from < caret {
                return (m.from as isize + delta).max(0) as usize;
            }
            break;
        }
    }
    (caret as isize + delta).max(0) as usize
}

impl App {
    /// The review sheet is up in the editor.
    #[must_use]
    pub fn review_shown(&self) -> bool {
        self.in_editor() && self.find.as_ref().is_some_and(|f| f.review.is_some())
    }

    /// A plan, apply or undo is running: nothing answers a tap or a key.
    #[must_use]
    pub fn replace_busy(&self) -> bool {
        self.replacer.pending()
    }

    /// The open note and its live text for the worker; `None` when the note
    /// could not be read (the app never writes it).
    fn open_note_text(&self) -> Option<(NotePath, String)> {
        (!self.read_only).then(|| (self.current.clone(), self.buffer.text().to_owned()))
    }

    /// Whether the open note is still `path` with the text the running (or
    /// last) job was given. If not, the note was reloaded or edited while the
    /// job ran, and the job's text for it is stale: it must not go into the
    /// buffer.
    fn open_note_unchanged(&self, path: &NotePath) -> bool {
        self.current == *path
            && self
                .replacer
                .base
                .as_ref()
                .is_some_and(|(p, h)| p == path && *h == replace::hash(self.buffer.text()))
    }

    /// The All key in All notes (S24-D1): plan the replace on the worker;
    /// the sheet opens when the plan arrives.
    pub(crate) fn review_open(&mut self) -> Option<Refresh> {
        if self.replace_busy() {
            return None;
        }
        let f = self.find.as_ref().filter(|f| f.scope == Scope::All)?;
        if f.query.text().is_empty() || f.review.is_some() {
            return None;
        }
        let job = Job::Plan {
            root: self.library.root().to_path_buf(),
            query: f.query.text().to_owned(),
            case: f.case,
            with: f.replace.text().to_owned(),
            open: self.open_note_text(),
        };
        self.replacer.request(job);
        None
    }

    /// The model of the sheet for `r`.
    pub(crate) fn review_model<'a>(&'a self, r: &'a ReviewState) -> ReviewModel<'a> {
        ReviewModel {
            plan: &r.plan,
            selection: &r.selection,
            page: r.page,
            progress: r.progress,
            theme: self.theme(),
        }
    }

    /// The sheet's layout, while it is up.
    #[must_use]
    pub fn review_layout_now(&self) -> Option<ReviewLayout> {
        let r = self.find.as_ref()?.review.as_ref()?;
        Some(
            self.screen
                .review_layout(self.page_rect(), &self.review_model(r)),
        )
    }

    /// A tap on the sheet: ticks, page arrows, Apply, Cancel.
    pub(crate) fn review_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.review_layout_now()?;
        match self.screen.review_hit(&l, x, y) {
            ReviewHit::NoteTick(i) => {
                let r = self.find.as_mut()?.review.as_mut()?;
                let ids = note_tick_targets(r.plan.notes.get(i)?);
                let on = !ids.iter().any(|&m| r.selection.is_ticked(i, m));
                r.selection.set_note(&r.plan, i, on);
                Some(Refresh::Partial)
            }
            ReviewHit::MatchTick(i, first) => {
                let r = self.find.as_mut()?.review.as_mut()?;
                let ids = review_line_matches(r.plan.notes.get(i)?, first);
                let on = !ids.iter().all(|&m| r.selection.is_ticked(i, m));
                for m in ids {
                    r.selection.set(i, m, on);
                }
                Some(Refresh::Partial)
            }
            ReviewHit::PagePrev => self.review_page(false),
            ReviewHit::PageNext => self.review_page(true),
            ReviewHit::Apply => self.review_apply(),
            ReviewHit::Cancel => self.review_cancel(),
            ReviewHit::None => None,
        }
    }

    /// Turns a page of the sheet (page arrows and keys).
    pub(crate) fn review_page(&mut self, forward: bool) -> Option<Refresh> {
        let l = self.review_layout_now()?;
        let next = if forward {
            (l.index + 1).min(l.pages.saturating_sub(1))
        } else {
            l.index.saturating_sub(1)
        };
        if next == l.index {
            return None;
        }
        self.find.as_mut()?.review.as_mut()?.page = next;
        Some(Refresh::Partial)
    }

    /// Apply: the worker rewrites the ticked notes one at a time.
    fn review_apply(&mut self) -> Option<Refresh> {
        if self.replace_busy() {
            return None;
        }
        let open = self.open_note_text();
        let r = self.find.as_mut()?.review.as_mut()?;
        if r.selection.total() == 0 {
            return None;
        }
        r.progress = Some((0, r.selection.notes()));
        let job = Job::Apply {
            plan: r.plan.clone(),
            selection: r.selection.clone(),
            open,
        };
        self.replacer.request(job);
        Some(Refresh::Partial)
    }

    /// Cancel, Esc and Back: the sheet goes, the results list stays as it
    /// was. Refused while the apply runs.
    pub(crate) fn review_cancel(&mut self) -> Option<Refresh> {
        if self.replace_busy() {
            return None;
        }
        let f = self.find.as_mut()?;
        f.review.take()?;
        Some(Refresh::Full)
    }

    /// What the notice band says and its button's label, while it is shown:
    /// in All notes with no sheet up, after a replace or an undo, and for as
    /// long as the last replace can be undone.
    #[must_use]
    pub(crate) fn notice_view(&self) -> Option<(String, Option<String>)> {
        let f = self
            .find
            .as_ref()
            .filter(|f| f.scope == Scope::All && f.review.is_none())?;
        match (&f.notice, &f.snapshot) {
            (Some(n), snap) => {
                let undo = snap.is_some() && matches!(n, Notice::Replaced { .. });
                Some((n.text(), undo.then(|| tr("Undo").into_owned())))
            }
            (None, Some(_)) => Some((String::new(), Some(tr("Undo last replace").into_owned()))),
            (None, None) => None,
        }
    }

    /// `page` with the notice band's room taken off its top, when the band
    /// is shown: where the results list goes.
    #[must_use]
    pub(crate) fn search_area(&self, page: sigma_render::Rect) -> sigma_render::Rect {
        if self.notice_view().is_none() {
            return page;
        }
        let h = self.screen.notice_h().min(page.h.max(0));
        sigma_render::Rect::new(page.x, page.y + h, page.w, page.h - h)
    }

    /// The band's layout, text and button label, when it is shown.
    #[must_use]
    pub(crate) fn notice_layout_now(
        &self,
    ) -> Option<(sigma_render::findbar::NoticeLayout, String, Option<String>)> {
        if !self.search_list_shown() {
            return None;
        }
        let (text, button) = self.notice_view()?;
        let l = self
            .screen
            .notice_layout(self.page_rect(), &text, button.as_deref());
        Some((l, text, button))
    }

    /// A tap on the notice band: its button undoes the last replace.
    /// Returns `Some(answer)` when the tap was on the band.
    pub(crate) fn notice_tap(&mut self, x: i32, y: i32) -> Option<Option<Refresh>> {
        let (l, _, button) = self.notice_layout_now()?;
        if !l.band.contains(x, y) {
            return None;
        }
        if button.is_some() && self.screen.notice_hit(&l, x, y) {
            return Some(self.replace_undo());
        }
        Some(None)
    }

    /// Undo last replace: the worker puts back the notes of the newest
    /// snapshot that still are what the replace wrote.
    pub(crate) fn replace_undo(&mut self) -> Option<Refresh> {
        if self.replace_busy() {
            return None;
        }
        let snapshot = self.find.as_ref()?.snapshot.clone()?;
        let job = Job::Undo {
            root: self.library.root().to_path_buf(),
            snapshot,
            open: self.open_note_text(),
        };
        self.replacer.request(job);
        None
    }

    /// Looks for the newest snapshot that can still be undone (a few
    /// directory reads); called when the bar enters All notes.
    pub(crate) fn find_snapshot(&self) -> Option<Snapshot> {
        replace::latest_snapshot(&self.library)
    }

    /// Takes what the worker has said (the idle tick).
    pub(crate) fn replace_poll(&mut self) -> Option<Refresh> {
        let msgs = self.replacer.poll();
        self.replace_messages(msgs)
    }

    /// Blocks until the running job is done (emulator, tests). Returns
    /// whether the screen changed.
    pub fn settle_replace(&mut self) -> bool {
        let msgs = self.replacer.wait();
        self.replace_messages(msgs).is_some()
    }

    fn replace_messages(&mut self, msgs: Vec<Msg>) -> Option<Refresh> {
        let mut out = None;
        for m in msgs {
            out = crate::merge(out, self.replace_message(m));
        }
        out
    }

    fn replace_message(&mut self, m: Msg) -> Option<Refresh> {
        match m {
            Msg::Planned(plan) => self.review_planned(plan),
            Msg::Progress(done, total) => {
                let r = self.find.as_mut()?.review.as_mut()?;
                r.progress = Some((done, total));
                Some(Refresh::Partial)
            }
            Msg::Applied(a) => self.replace_applied(&a),
            Msg::Undone(u) => self.replace_undone(u),
            Msg::Lost => {
                sigma_core::warn!("replace", "the worker stopped");
                if let Some(f) = &mut self.find {
                    if let Some(r) = &mut f.review {
                        r.progress = None;
                    }
                }
                Some(Refresh::Full)
            }
        }
    }

    /// A plan came back: the sheet opens, unless the fields moved on.
    fn review_planned(&mut self, plan: Plan) -> Option<Refresh> {
        let f = self
            .find
            .as_mut()
            .filter(|f| f.scope == Scope::All && f.review.is_none())?;
        if plan.query != f.query.text() || plan.case != f.case || plan.with != f.replace.text() {
            return None;
        }
        sigma_core::info!(
            "replace",
            "plan: {} chars, {} matches in {} notes ({} read, {} skipped): {} ms",
            plan.query.chars().count(),
            plan.total_matches(),
            plan.notes.len(),
            plan.scanned,
            plan.skipped,
            plan.elapsed_ms
        );
        let selection = Selection::defaults(&plan);
        f.notice = None;
        f.review = Some(ReviewState {
            plan: Arc::new(plan),
            selection,
            page: 0,
            progress: None,
        });
        Some(Refresh::Full)
    }

    /// The results, the Graph, the backlinks and the to-do index follow the
    /// notes that changed (what a rename does).
    fn replace_invalidate(&mut self) {
        self.clear_map_cache();
        self.reindex();
        self.search_after_replace();
    }

    /// Searches the query again from the first page.
    fn search_after_replace(&mut self) {
        if let Some(f) = &mut self.find {
            f.first = 0;
            f.selected = None;
        }
        self.search_request();
    }

    /// The apply finished: the open note goes through the buffer, the sheet
    /// closes and the band tells what happened.
    fn replace_applied(&mut self, a: &Applied) -> Option<Refresh> {
        let review = self.find.as_mut().and_then(|f| f.review.take());
        let (mut matches, mut notes, mut changed) = (a.matches, a.notes, a.changed());
        if let Some((path, text)) = &a.open_text {
            if self.open_note_unchanged(path) {
                let caret = match &review {
                    Some(r) => caret_after(&r.plan, &r.selection, path, self.buffer.caret()),
                    None => self.buffer.caret(),
                };
                // One undo step, then saved byte for byte.
                self.buffer.replace_text(text.clone(), caret);
                self.save();
                self.screen.follow_caret(&self.buffer, self.page_rect());
            } else {
                // The note was reloaded or edited while the worker ran: its
                // text is newer than the review, so it is not touched (it
                // was never written) and counts as changed since the review.
                sigma_core::warn!(
                    "replace",
                    "{} changed while the replace ran: left as it is",
                    path.as_str()
                );
                let ticked = a
                    .results
                    .iter()
                    .find(|r| r.path == *path && r.outcome == Outcome::Open)
                    .map_or(0, |r| r.matches);
                matches = matches.saturating_sub(ticked);
                notes = notes.saturating_sub(1);
                changed += 1;
            }
        }
        sigma_core::info!(
            "replace",
            "applied: {} matches in {} notes, {} changed, {} failed: {} ms",
            matches,
            notes,
            changed,
            a.failed(),
            a.elapsed_ms
        );
        let notice = if a.error.is_some() {
            Notice::NotReplaced
        } else {
            Notice::Replaced {
                matches,
                notes,
                changed,
                failed: a.failed(),
            }
        };
        if let Some(f) = &mut self.find {
            f.notice = Some(notice);
            // The band's Undo is for this replace's snapshot, and for none
            // if it made none: never an older one.
            f.snapshot.clone_from(&a.snapshot);
        }
        self.replace_invalidate();
        Some(Refresh::Full)
    }

    /// The undo finished: as above, for the notes put back.
    fn replace_undone(&mut self, u: Undone) -> Option<Refresh> {
        let mut notes = u.restored.len();
        let mut left = u.skipped.len();
        if let Some((path, text)) = u.open_text {
            if self.open_note_unchanged(&path) {
                let caret = self.buffer.caret().min(text.len());
                self.buffer.replace_text(text, caret);
                self.save();
                self.screen.follow_caret(&self.buffer, self.page_rect());
                notes += 1;
            } else {
                // Reloaded or edited while the worker ran: left as it is.
                sigma_core::warn!(
                    "replace",
                    "{} changed while the undo ran: left as it is",
                    path.as_str()
                );
                left += 1;
            }
        }
        sigma_core::info!(
            "replace",
            "undone: {notes} restored, {left} left alone: {} ms",
            u.elapsed_ms
        );
        let notice = if u.error.is_some() {
            Notice::NotRestored
        } else {
            Notice::Restored { notes, left }
        };
        if let Some(f) = &mut self.find {
            f.notice = Some(notice);
            f.snapshot = None;
        }
        self.replace_invalidate();
        Some(Refresh::Full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::replace::{Match, NotePlan};

    fn plan(with: &str, matches: &[(usize, usize)]) -> Plan {
        Plan {
            root: PathBuf::new(),
            query: "ab".into(),
            case: false,
            with: with.into(),
            notes: vec![NotePlan {
                path: NotePath::parse("a.md").unwrap(),
                matches: matches
                    .iter()
                    .map(|&(from, to)| Match {
                        from,
                        to,
                        in_link: false,
                    })
                    .collect(),
                lines: Vec::new(),
                open: true,
            }],
            skipped: 0,
            scanned: 1,
            elapsed_ms: 0,
        }
    }

    /// Matches before the caret shift it; one under it moves it to the
    /// match's start; unticked matches shift nothing.
    #[test]
    fn caret_follows_the_replaced_text() {
        let a = NotePath::parse("a.md").unwrap();
        let p = plan("xyz", &[(0, 2), (10, 12), (20, 22)]);
        let all = Selection::all(&p);
        // After the first match only: +1.
        assert_eq!(caret_after(&p, &all, &a, 5), 6);
        // After two matches: +2.
        assert_eq!(caret_after(&p, &all, &a, 15), 17);
        // Inside the second match: at its new start.
        assert_eq!(caret_after(&p, &all, &a, 11), 11);
        // Unticked: nothing moves.
        let none = Selection::none(&p);
        assert_eq!(caret_after(&p, &none, &a, 15), 15);
        // A note that was not in the plan.
        let b = NotePath::parse("b.md").unwrap();
        assert_eq!(caret_after(&p, &all, &b, 15), 15);
        // Deleting shrinks.
        let d = plan("", &[(0, 2)]);
        assert_eq!(caret_after(&d, &Selection::all(&d), &a, 5), 3);
    }

    #[test]
    fn notice_text_joins_the_counts() {
        let n = Notice::Replaced {
            matches: 52,
            notes: 3,
            changed: 0,
            failed: 0,
        };
        assert_eq!(n.text(), "Replaced 52 in 3 notes");
        let n = Notice::Replaced {
            matches: 1,
            notes: 1,
            changed: 2,
            failed: 1,
        };
        assert_eq!(
            n.text(),
            "Replaced 1 in 1 note \u{b7} 2 changed since the review \u{b7} 1 could not be written"
        );
        assert_eq!(
            Notice::Restored { notes: 2, left: 1 }.text(),
            "Restored 2 notes \u{b7} 1 left alone"
        );
    }
}
