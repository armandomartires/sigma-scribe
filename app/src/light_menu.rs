//! The front-light button's menu: Backlight and Warmth sliders in the title
//! bar. They drive the same state as Settings → Display, through the same
//! changes, so the device, the saved warmth and the settings rows agree.

use crate::App;
use sigma_core::settings::Change;
use sigma_device::{Phase, Refresh};
use sigma_render::light_menu::{LightHit, LightRow};

impl App {
    /// Copies the light's state to the screen: the button is there only
    /// with a front light, and Warmth only with a mixer.
    pub(crate) fn sync_light_screen(&mut self) {
        self.screen.light_on = self.light_have;
        self.screen.light_level = self.light_step;
        self.screen.warmth_level = self.warmth_have.then_some(self.warmth_step);
        if !self.light_have {
            self.screen.light_menu = false;
        }
    }

    /// The light button: opens or closes the light menu (and the drawer
    /// menu makes way for it).
    pub(crate) fn toggle_light_menu(&mut self) -> Refresh {
        self.screen.light_menu = !self.screen.light_menu;
        self.screen.drawer_menu = false;
        self.screen.wifi_menu = false;
        self.light_touch = None;
        self.sel_ui = None;
        Refresh::Partial
    }

    /// Sets a slider to `n`, as Settings → Display does. The same step
    /// again (every drag event repeats it) does nothing.
    fn light_set(&mut self, row: LightRow, n: u8) -> Option<Refresh> {
        let c = match row {
            LightRow::Brightness if n == self.light_step => return None,
            LightRow::Brightness => Change::Backlight(n),
            // Against the saved step, so the first touch after a start with
            // nothing chosen is kept (as in Settings).
            LightRow::Warmth if Some(n) == self.settings.warmth => return None,
            LightRow::Warmth => Change::Warmth(n),
        };
        self.change(c);
        Some(Refresh::Partial)
    }

    fn light_layout(&self) -> sigma_render::editor::Layout {
        let keys = self.preview.is_none() && self.keys_visible();
        self.screen.layout(keys, &self.keyboard)
    }

    /// A tap while the menu is open: a slider sets its step and the menu
    /// stays; the button or any other place closes it, and the tap does
    /// nothing else.
    pub(crate) fn light_menu_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.light_layout();
        match self.screen.light_menu_hit(&l, x, y) {
            LightHit::Slider(r, n) => self.light_set(r, n),
            LightHit::Frame => None,
            LightHit::Button | LightHit::Outside => {
                self.screen.light_menu = false;
                Some(Refresh::Partial)
            }
        }
    }

    /// Touch while the menu is open. A finger down on a slider moves it and
    /// follows the finger until it lifts; `None` leaves any other touch to
    /// the gesture reader (a tap elsewhere closes the menu).
    pub(crate) fn light_menu_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
    ) -> Option<Option<Refresh>> {
        let l = self.light_layout();
        match phase {
            Phase::Down => {
                let LightHit::Slider(r, n) = self.screen.light_menu_hit(&l, x, y) else {
                    return None;
                };
                self.light_touch = Some(r);
                Some(self.light_set(r, n))
            }
            Phase::Move | Phase::Up => {
                let r = self.light_touch?;
                if phase == Phase::Up {
                    self.light_touch = None;
                }
                let n = self.screen.light_menu_drag(&l, r, x)?;
                Some(self.light_set(r, n))
            }
        }
    }

    /// Draws the light menu when it is open. Leaving write mode and preview
    /// closes it.
    pub(crate) fn draw_light_menu(&mut self, c: &mut sigma_render::Canvas) {
        if !self.drawer_allowed() {
            self.screen.light_menu = false;
        }
        let l = self.light_layout();
        self.screen.draw_light_menu(c, &l, self.theme());
    }
}
