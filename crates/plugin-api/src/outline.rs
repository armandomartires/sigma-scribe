//! Stable outline helpers for plugins.
//!
//! Today this is the to-dos pane [`Filter`]. The rest of the outline index
//! (headings, to-do scan, the background indexer) stays in `sigma-core`.

/// To-dos pane filter: the button cycles All → Open → Done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every to-do.
    #[default]
    All,
    /// Unchecked to-dos only.
    Open,
    /// Checked to-dos only.
    Done,
}

impl Filter {
    /// The filter the button moves to next.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Open,
            Self::Open => Self::Done,
            Self::Done => Self::All,
        }
    }

    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Done => "Done",
        }
    }

    /// Whether a to-do with this `done` state is shown.
    #[must_use]
    pub fn keeps(self, done: bool) -> bool {
        match self {
            Self::All => true,
            Self::Open => !done,
            Self::Done => done,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_cycles() {
        let f = Filter::default();
        assert_eq!(
            (f.next(), f.next().next(), f.next().next().next()),
            (Filter::Open, Filter::Done, Filter::All)
        );
        assert!(Filter::Open.keeps(false) && !Filter::Open.keeps(true));
        assert!(Filter::Done.keeps(true) && Filter::All.keeps(false));
        assert_eq!(Filter::All.label(), "All");
        assert_eq!(Filter::Open.label(), "Open");
        assert_eq!(Filter::Done.label(), "Done");
    }
}
