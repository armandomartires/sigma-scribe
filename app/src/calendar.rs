//! The Calendar pane of the side drawer: which month it shows, what a tap
//! on a day, on Today or on the entry buttons does, and the cached list of
//! journal entries behind the marks.

use crate::App;
use sigma_core::date::{self, Date};
use sigma_core::journal;
use sigma_device::Refresh;
use sigma_render::calendar::{long_date, year_page, CalField, CalPicker, CalendarModel};

/// The month shown when nothing says which: no open entry and no clock.
const FALLBACK_MONTH: Date = Date {
    y: 2020,
    m: 1,
    d: 1,
};

impl App {
    /// Reads the journal folder again for the Calendar's marks. Not done
    /// every frame: `Library::list` reads every file of a folder.
    pub(crate) fn refresh_journal_marks(&mut self) {
        let entries = journal::entries(&self.library, self.settings.journal_layout);
        self.journal_marks = entries.keys().copied().collect();
        self.journal_entries = entries;
    }

    /// Today by the clock and the UTC offset setting (`SIGMA_TODAY` wins).
    #[must_use]
    pub fn today(&self) -> Option<Date> {
        date::today(i32::from(self.settings.utc_offset))
    }

    /// The date of the open note when it is a journal entry.
    #[must_use]
    pub fn open_entry_date(&self) -> Option<Date> {
        self.journal_entries
            .iter()
            .find(|(_, p)| **p == self.current)
            .map(|(d, _)| *d)
    }

    /// What the Calendar pane shows.
    pub(crate) fn calendar_model(&self) -> CalendarModel<'_> {
        let today = self.today();
        let open = self.open_entry_date();
        let month = self
            .calendar_month
            .or(open)
            .or(today)
            .or_else(|| self.journal_marks.last().copied())
            .unwrap_or(FALLBACK_MONTH)
            .first_of_month();
        CalendarModel {
            month,
            today,
            marked: &self.journal_marks,
            open,
            week_start: self.settings.week_start,
            picker: self.calendar_picker,
        }
    }

    /// Opens the note of `d`, creating it first when there is none, and
    /// keeps the drawer as it is. A note open in preview stays in preview.
    pub fn calendar_open(&mut self, d: Date) -> Option<Refresh> {
        self.scratch_focus = false;
        let layout = self.settings.journal_layout;
        let heading = long_date(d);
        let (p, created) = match journal::open_or_create(&self.library, d, layout, &heading) {
            Ok(r) => r,
            Err(e) => {
                sigma_core::error!("journal", "cannot open {d}: {e}");
                return Some(Refresh::Partial);
            }
        };
        if created {
            sigma_core::info!("journal", "created {}", p.as_str());
        }
        self.refresh_journal_marks();
        if p == self.current {
            return Some(Refresh::Partial);
        }
        self.push_history();
        let preview = self.preview.is_some();
        if preview {
            self.exit_preview();
        }
        self.open(&p);
        if preview {
            self.enter_preview();
        }
        Some(Refresh::Full)
    }

    /// Today: opens today's note and shows its month. With the clock not
    /// set there is no today: the pane says so and nothing else happens.
    pub fn calendar_today(&mut self) -> Option<Refresh> {
        let Some(t) = self.today() else {
            sigma_core::warn!("journal", "the clock is not set: no today");
            return Some(Refresh::Partial);
        };
        self.calendar_month = Some(t.first_of_month());
        self.calendar_picker = None;
        self.calendar_open(t)
    }

    /// A tap on a field of the date line: opens its picker, closes it when
    /// it is the open one, switches from another picker.
    pub fn calendar_field(&mut self, f: CalField) -> Option<Refresh> {
        let shown = self.calendar_model().month;
        self.calendar_picker = match (self.calendar_picker, f) {
            (Some(p), f) if p.field() == f => None,
            (_, CalField::Day) => Some(CalPicker::Day),
            (_, CalField::Month) => Some(CalPicker::Month),
            (_, CalField::Year) => Some(CalPicker::Year {
                first_year: year_page(shown.y),
            }),
        };
        Some(Refresh::Partial)
    }

    /// A tap on a cell of the open picker: a year or a month sets the shown
    /// month, a day opens or creates its note. Either closes the picker.
    pub fn calendar_pick(&mut self, v: u16) -> Option<Refresh> {
        let shown = self.calendar_model().month;
        let picker = self.calendar_picker.take()?;
        match picker {
            CalPicker::Year { .. } => {
                self.calendar_month = Date::new(i32::from(v), shown.m, 1);
                Some(Refresh::Partial)
            }
            CalPicker::Month => {
                self.calendar_month = u8::try_from(v).ok().and_then(|m| Date::new(shown.y, m, 1));
                Some(Refresh::Partial)
            }
            CalPicker::Day => {
                let day = u8::try_from(v)
                    .ok()
                    .and_then(|d| Date::new(shown.y, shown.m, d));
                match day {
                    Some(d) => self.calendar_open(d),
                    None => Some(Refresh::Partial),
                }
            }
        }
    }

    /// The previous or next month button; with the year picker open it
    /// pages the twelve years instead.
    pub fn calendar_step_month(&mut self, next: bool) -> Option<Refresh> {
        if let Some(CalPicker::Year { first_year }) = self.calendar_picker {
            let first_year = if next {
                first_year + 12
            } else {
                (first_year - 12).max(1)
            };
            self.calendar_picker = Some(CalPicker::Year { first_year });
            return Some(Refresh::Partial);
        }
        let shown = self.calendar_model().month;
        self.calendar_month = Some(if next {
            shown.next_month()
        } else {
            shown.prev_month()
        });
        Some(Refresh::Partial)
    }

    /// The previous or next entry before or after the open one.
    pub fn calendar_step_entry(&mut self, next: bool) -> Option<Refresh> {
        let m = self.calendar_model();
        let to = if next { m.next_entry() } else { m.prev_entry() };
        let Some(to) = to else {
            return Some(Refresh::Partial);
        };
        self.calendar_month = Some(to.first_of_month());
        self.calendar_open(to)
    }
}
