//! The Calendar pane of the side drawer (journal): a tap on a day opens or
//! creates that day's note under `journal/`, Today and the entry buttons
//! move between notes, and the drawer stays open.

use sigma_core::date::{Date, WeekStart};
use sigma_core::{Change, NotePath};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_scribe::App;

/// The date every test here calls today (a Monday).
const TODAY: &str = "2026-11-02";

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        // The same value in every test, so the parallel tests agree.
        std::env::set_var("SIGMA_TODAY", TODAY);
        let d = std::env::temp_dir().join(format!("sigma-cal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("n.md"), "# Plain note\n").unwrap();
        let mut app = App::new(1440, 1920, d.clone());
        app.set_builtin_panes(true);
        app.open(&NotePath::parse("n.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        let mut r = Self { app, dev, dir: d };
        r.tap_on("Panels");
        r.tap_on("Calendar");
        r.tap_on("Panels");
        r
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        self.send(Event::Tap { x, y })
    }
    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.dir.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn path(s: &str) -> NotePath {
    NotePath::parse(s).unwrap()
}

#[test]
fn tapping_a_day_creates_its_note_with_a_dated_heading_and_a_second_tap_only_opens_it() {
    let mut r = Rig::new("day");
    assert!(r.app.drawer[3], "the Calendar pane is open");
    assert!(!r.dir.join("journal").exists());
    r.tap_on("Day:2026-11-02");
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    assert_eq!(
        r.read("journal/2026-11-02.md"),
        "# Monday, 2 November 2026\n\n"
    );
    assert!(
        r.app.drawer[3] && r.app.drawer_visible(),
        "drawer stays open"
    );
    assert_eq!(r.app.open_entry_date(), Date::new(2026, 11, 2));

    // Write something, leave, come back: the note is opened, not rewritten.
    r.app.buffer.set_caret(r.app.buffer.text().len());
    r.app.buffer.insert("kept");
    r.app.open(&path("n.md"));
    r.tap_on("Day:2026-11-02");
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    assert_eq!(
        r.read("journal/2026-11-02.md"),
        "# Monday, 2 November 2026\n\nkept"
    );
    // A day on another weekday gets its own heading.
    r.tap_on("Day:2026-11-04");
    assert_eq!(
        r.read("journal/2026-11-04.md"),
        "# Wednesday, 4 November 2026\n\n"
    );
    // Back goes to the note before.
    assert_eq!(r.app.current, path("journal/2026-11-04.md"));
    r.app.go_back();
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
}

#[test]
fn a_day_opens_in_preview_when_the_drawer_is_used_there() {
    let mut r = Rig::new("preview");
    r.tap_on("Preview");
    assert!(r.app.preview.is_some());
    r.tap_on("Day:2026-11-03");
    assert!(r.app.preview.is_some(), "still in preview");
    assert_eq!(r.app.current, path("journal/2026-11-03.md"));
    assert!(r.dir.join("journal/2026-11-03.md").exists());
    assert!(r.app.drawer_visible());
}

#[test]
fn today_and_the_month_buttons_move_the_month() {
    let mut r = Rig::new("today");
    let shown = |r: &Rig| r.app.control_centre("Day:2026-10-15").is_some();
    assert!(
        r.app.control_centre("Day:2026-11-15").is_some(),
        "today's month"
    );
    r.tap_on("CalPrevMonth");
    assert!(shown(&r) && r.app.control_centre("Day:2026-11-15").is_none());
    r.tap_on("CalNextMonth");
    r.tap_on("CalNextMonth");
    assert!(r.app.control_centre("Day:2026-12-15").is_some());
    r.tap_on("CalToday");
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    assert!(
        r.app.control_centre("Day:2026-11-15").is_some(),
        "back to November"
    );
    assert_eq!(r.app.calendar_month, Date::new(2026, 11, 1));
    assert!(r.dir.join("journal/2026-11-02.md").exists());
    // The year rolls over both ways.
    r.app.calendar_month = Date::new(2026, 12, 1);
    r.tap_on("CalNextMonth");
    assert!(r.app.control_centre("Day:2027-01-15").is_some());
    r.tap_on("CalPrevMonth");
    r.tap_on("CalPrevMonth");
    assert!(r.app.control_centre("Day:2026-11-15").is_some());
}

#[test]
fn previous_and_next_entry_walk_the_existing_notes() {
    let mut r = Rig::new("walk");
    std::fs::create_dir_all(r.dir.join("journal")).unwrap();
    for d in ["2026-10-20", "2026-11-02", "2026-11-05", "2026-12-24"] {
        std::fs::write(r.dir.join(format!("journal/{d}.md")), format!("# {d}\n")).unwrap();
    }
    // The pane reads the folder when it opens; a note made behind its back
    // is found when a day is opened.
    r.tap_on("Day:2026-11-05");
    assert!(
        r.app.control_centre("CalPrevEntry").is_some(),
        "the footer shows for an open entry"
    );
    r.tap_on("CalPrevEntry");
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    r.tap_on("CalPrevEntry");
    assert_eq!(r.app.current, path("journal/2026-10-20.md"));
    assert_eq!(
        r.app.calendar_month,
        Date::new(2026, 10, 1),
        "the month follows"
    );
    // No earlier entry: nothing happens.
    r.tap_on("CalPrevEntry");
    assert_eq!(r.app.current, path("journal/2026-10-20.md"));
    r.tap_on("CalNextEntry");
    r.tap_on("CalNextEntry");
    assert_eq!(r.app.current, path("journal/2026-11-05.md"));
    r.tap_on("CalNextEntry");
    assert_eq!(r.app.current, path("journal/2026-12-24.md"));
    r.tap_on("CalNextEntry");
    assert_eq!(r.app.current, path("journal/2026-12-24.md"));
    assert_eq!(
        r.read("journal/2026-12-24.md"),
        "# 2026-12-24\n",
        "never rewritten"
    );
    // A note that is not an entry has no footer.
    r.app.open(&path("n.md"));
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert!(r.app.control_centre("CalPrevEntry").is_none());
}

#[test]
fn the_layout_and_week_start_settings_apply_to_the_calendar() {
    let mut r = Rig::new("settings");
    let x_of = |r: &Rig, d: &str| r.app.control_centre(&format!("Day:{d}")).unwrap().0;
    // 1 November 2026 is a Sunday.
    let (mon1, mon2) = (x_of(&r, "2026-11-01"), x_of(&r, "2026-11-02"));
    assert!(mon2 < mon1, "Monday weeks: Sunday last");
    r.app.change(Change::WeekStart(WeekStart::Sunday));
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let (sun1, sun2) = (x_of(&r, "2026-11-01"), x_of(&r, "2026-11-02"));
    assert!(sun1 < sun2, "Sunday weeks: Sunday first");
    assert_eq!(sun1, mon2);

    // A new note lands where the layout says, and is marked afterwards.
    r.app.change(Change::JournalLayout(
        sigma_core::journal::JournalLayout::YearWeek,
    ));
    r.tap_on("Day:2026-11-02");
    assert!(r.dir.join("journal/2026/W45/2026-11-02.md").exists());
    assert!(!r.dir.join("journal/2026-11-02.md").exists());
    // Reorganising keeps the marks right: back to one folder.
    r.app.change(Change::JournalLayout(
        sigma_core::journal::JournalLayout::Flat,
    ));
    assert!(r.dir.join("journal/2026-11-02.md").exists());
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    assert_eq!(r.app.open_entry_date(), Date::new(2026, 11, 2));
}

#[test]
fn the_date_line_fields_open_close_and_switch_their_pickers() {
    use sigma_render::calendar::CalPicker;
    let mut r = Rig::new("fields");
    assert!(
        r.app.control_centre("CalPickDay").is_some(),
        "the line shows"
    );
    assert!(r.app.calendar_picker.is_none());
    r.tap_on("CalPickYear");
    assert!(matches!(
        r.app.calendar_picker,
        Some(CalPicker::Year { first_year: 2016 })
    ));
    assert!(
        r.app.control_centre("Day:2026-11-15").is_none(),
        "the grid gives way"
    );
    assert!(r.app.control_centre("Pick:2026").is_some());
    // The same field again closes it.
    r.tap_on("CalPickYear");
    assert!(r.app.calendar_picker.is_none());
    assert!(r.app.control_centre("Day:2026-11-15").is_some());
    // Another field switches picker.
    r.tap_on("CalPickMonth");
    assert_eq!(r.app.calendar_picker, Some(CalPicker::Month));
    r.tap_on("CalPickDay");
    assert_eq!(r.app.calendar_picker, Some(CalPicker::Day));
    assert!(r.app.control_centre("Pick:30").is_some());
    assert!(r.app.control_centre("Pick:31").is_none(), "November has 30");
    // Today closes the picker and opens today's note.
    r.tap_on("CalToday");
    assert!(r.app.calendar_picker.is_none());
    assert_eq!(r.app.current, path("journal/2026-11-02.md"));
    // Closing the drawer or switching the pane off forgets it.
    r.tap_on("CalPickMonth");
    r.app.close_drawer();
    assert!(r.app.calendar_picker.is_none());
}

#[test]
fn picking_a_year_or_a_month_changes_the_shown_month_and_closes_the_picker() {
    let mut r = Rig::new("pickym");
    r.tap_on("CalPickYear");
    r.tap_on("Pick:2024");
    assert!(r.app.calendar_picker.is_none());
    assert_eq!(r.app.calendar_month, Date::new(2024, 11, 1), "month kept");
    assert!(r.app.control_centre("Day:2024-11-15").is_some());
    assert!(!r.dir.join("journal").exists(), "nothing is created");
    r.tap_on("CalPickMonth");
    r.tap_on("Pick:2");
    assert!(r.app.calendar_picker.is_none());
    assert_eq!(r.app.calendar_month, Date::new(2024, 2, 1), "year kept");
    assert!(
        r.app.control_centre("Day:2024-02-29").is_some(),
        "leap year"
    );
    // The year picker pages by twelve with the arrows.
    r.tap_on("CalPickYear");
    assert!(r.app.control_centre("Pick:2016").is_some());
    r.tap_on("CalNextMonth");
    assert!(r.app.control_centre("Pick:2028").is_some());
    assert!(r.app.control_centre("Pick:2024").is_none());
    assert_eq!(
        r.app.calendar_month,
        Date::new(2024, 2, 1),
        "month unchanged"
    );
    r.tap_on("CalPrevMonth");
    r.tap_on("CalPrevMonth");
    assert!(r.app.control_centre("Pick:2015").is_some());
    r.tap_on("Pick:2015");
    assert_eq!(r.app.calendar_month, Date::new(2015, 2, 1));
}

#[test]
fn picking_a_day_opens_or_creates_that_note_and_closes_the_picker() {
    let mut r = Rig::new("pickday");
    r.tap_on("CalPickDay");
    r.tap_on("Pick:17");
    assert!(r.app.calendar_picker.is_none());
    assert_eq!(r.app.current, path("journal/2026-11-17.md"));
    assert_eq!(
        r.read("journal/2026-11-17.md"),
        "# Tuesday, 17 November 2026\n\n"
    );
    assert!(r.app.drawer_visible());
    // The line now shows the open entry's day; picking another month and a
    // day goes to that month.
    r.tap_on("CalPickMonth");
    r.tap_on("Pick:12");
    r.tap_on("CalPickDay");
    r.tap_on("Pick:24");
    assert_eq!(r.app.current, path("journal/2026-12-24.md"));
    assert!(r.app.calendar_picker.is_none());
}
