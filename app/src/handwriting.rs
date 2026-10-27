//! The handwriting recogniser and where its samples live.

/// The user's handwriting samples, inside the notes folder (hidden).
#[must_use]
pub fn samples_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".handwriting").join("samples.txt")
}

/// Samples learned from corrections: the ink of a letter the user fixed
/// by picking a candidate, labelled with the pick. Same format as
/// [`samples_path`], at most [`LEARN_PER_CHAR`] per character, oldest
/// first.
#[must_use]
pub fn learned_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".handwriting").join("learned.txt")
}

/// Learned samples kept per character: the newest replace the oldest, so
/// a correction made by mistake soon ages out.
pub const LEARN_PER_CHAR: usize = 3;

/// The enrolled and the learned samples.
#[must_use]
pub fn all_samples(root: &std::path::Path) -> Vec<sigma_hwr::Sample> {
    [samples_path(root), learned_path(root)]
        .iter()
        .flat_map(|p| {
            std::fs::read_to_string(p)
                .map(|t| sigma_hwr::samples_from_text(&t))
                .unwrap_or_default()
        })
        .collect()
}

/// The recogniser and its background fine-tuning (v1.5.2).
#[derive(Default)]
pub struct Hwr {
    pub rec: Option<sigma_hwr::Recogniser>,
    job: Option<std::thread::JoinHandle<sigma_hwr::Recogniser>>,
    /// Enrolled and learned samples.
    pub samples: usize,
    /// A refit was asked for while one was running: run one more after it.
    again: Option<std::path::PathBuf>,
}

impl Hwr {
    /// Loads the model and the samples; fine-tuning on the samples runs
    /// on a thread, the plain model answers meanwhile.
    pub fn load(&mut self, root: &std::path::Path) {
        self.rec = Some(Self::base());
        self.job = None;
        self.refit(root);
    }

    fn base() -> sigma_hwr::Recogniser {
        sigma_hwr::Recogniser::new(sigma_hwr::Model::default_model())
            .with_pieces(sigma_hwr::Model::pieces_model())
    }

    /// Fine-tunes again on the samples now on disk (after a correction was
    /// learned), on a thread; the current recogniser answers meanwhile. If
    /// a fit is already running, one more follows it.
    pub fn refit(&mut self, root: &std::path::Path) {
        if self.job.is_some() {
            self.again = Some(root.to_path_buf());
            return;
        }
        let samples = all_samples(root);
        self.samples = samples.len();
        if samples.is_empty() {
            self.rec = Some(Self::base());
            return;
        }
        let mut tuned = Self::base();
        self.job = Some(std::thread::spawn(move || {
            tuned.set_samples(&samples);
            tuned
        }));
    }

    /// Takes the fine-tuned recogniser once ready (`wait`: block for it).
    pub fn poll(&mut self, wait: bool) {
        if self.job.as_ref().is_some_and(|j| wait || j.is_finished()) {
            if let Some(Ok(r)) = self.job.take().map(std::thread::JoinHandle::join) {
                self.rec = Some(r);
            }
            if let Some(root) = self.again.take() {
                self.refit(&root);
                if wait {
                    self.poll(true);
                }
            }
        }
    }
}
