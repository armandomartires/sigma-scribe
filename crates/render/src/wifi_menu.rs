//! The Wi-Fi button and its menu (v1.18), after the drawer menu: one
//! title-bar icon, a small framed menu under it, a tap anywhere else closes
//! it. From the top the menu holds
//!
//! - the Wi-Fi switch;
//! - the state: the network joined and its address, or what is going on, or
//!   what went wrong;
//! - the networks found by the last scan, strongest first, each with its
//!   signal, a lock when it needs a password and, for a saved one, a cross
//!   that forgets it;
//! - Scan again.
//!
//! The menu knows nothing about radios: the app hands it a [`WifiModel`].
//! The password of a network is typed into the title bar (the rename
//! field), with an eye to show it; [`EditorScreen::draw_wifi_field`] adds
//! the eye and a hint to that field.

use crate::canvas::{Canvas, Rect};
use crate::editor::{ellipsize, EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::switch;
use crate::theme::Theme;

/// What the title-bar icon shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiState {
    /// Radio off.
    Off,
    /// Radio on, no network joined.
    On,
    /// A network joined.
    Connected,
}

impl WifiState {
    /// The icon: crossed out, a dot and one arc, all three arcs.
    #[must_use]
    pub fn icon(self) -> Icon {
        match self {
            Self::Off => Icon::WifiOff,
            Self::On => Icon::WifiLow,
            Self::Connected => Icon::Wifi,
        }
    }
}

/// What the radio is busy with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Nothing.
    Idle,
    /// Turning on.
    TurningOn,
    /// Turning off.
    TurningOff,
    /// Scanning.
    Scanning,
    /// Joining a network.
    Joining,
}

/// One network in the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetRow {
    /// Its name.
    pub ssid: String,
    /// Signal, 0 (out of range) to 4 bars.
    pub bars: u8,
    /// It needs a password (or cannot be joined).
    pub locked: bool,
    /// It is saved: it joins without a password and can be forgotten.
    pub saved: bool,
    /// It is the network joined.
    pub connected: bool,
}

/// Everything the menu shows.
#[derive(Clone, Copy, Debug)]
pub struct WifiModel<'a> {
    /// The switch.
    pub powered: bool,
    /// What is going on.
    pub activity: Activity,
    /// The network joined.
    pub ssid: Option<&'a str>,
    /// Its address.
    pub ip: Option<&'a str>,
    /// What went wrong last (English; translated here).
    pub note: Option<&'a str>,
    /// The networks, in the order to list them.
    pub networks: &'a [NetRow],
}

/// A tap in the open menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiHit {
    /// The Wi-Fi switch row.
    Switch,
    /// A network row (index into the model's networks).
    Net(usize),
    /// The cross of a saved network.
    Forget(usize),
    /// Scan again.
    Rescan,
    /// The state block, or a hint: nothing happens, the menu stays.
    Inside,
    /// The Wi-Fi button: close the menu.
    Button,
    /// Anywhere else: close the menu (the tap goes no further).
    Outside,
}

/// Where everything of the open menu is.
#[derive(Clone, Debug)]
pub struct WifiLayout {
    /// The frame.
    pub frame: Rect,
    /// The switch row.
    pub switch_row: Rect,
    /// The state block under it (two lines).
    pub status: Rect,
    /// The networks shown: model index, row, and the cross of a saved one.
    pub nets: Vec<(usize, Rect, Option<Rect>)>,
    /// The line standing in for an empty list, when there is one.
    pub hint: Option<(Rect, &'static str)>,
    /// Scan again, when the radio is on.
    pub rescan: Option<Rect>,
}

impl EditorScreen {
    /// The menu's frame and rows, right-aligned under the Wi-Fi button.
    #[must_use]
    pub fn wifi_menu_layout(&self, l: &Layout, m: &WifiModel<'_>) -> WifiLayout {
        let (row_h, edge) = (self.px(56.0), self.px(4.0));
        let status_h = self.px(76.0);
        let w = self
            .px(440.0)
            .min(self.width as i32 - self.px(16.0))
            .max(self.px(220.0));
        let b = l.wifi_button;
        let x = (b.x + b.w - w).clamp(self.px(8.0), (self.width as i32 - w).max(0));
        let top = l.title.h;
        let fixed = 2 * edge + row_h + status_h;
        let (rows_on, rescan_on) = (m.powered, m.powered);
        // Room for the list: what is left of the screen under the menu's
        // fixed rows and the Scan row, in whole rows; never more than 8.
        let room = (self.height as i32 - top - fixed - if rescan_on { row_h } else { 0 }) / row_h;
        let max = room.clamp(1, 8) as usize;
        let shown: Vec<usize> = if rows_on {
            (0..m.networks.len()).take(max).collect()
        } else {
            Vec::new()
        };
        let hint_text: Option<&'static str> = (rows_on && m.networks.is_empty()).then_some(
            if m.activity == Activity::Scanning || m.activity == Activity::TurningOn {
                "Scanning\u{2026}"
            } else {
                "No networks found"
            },
        );
        let lines = shown.len() as i32 + i32::from(hint_text.is_some());
        let h = fixed + lines * row_h + if rescan_on { row_h } else { 0 };
        let frame = Rect::new(x, top, w, h);
        let switch_row = Rect::new(x, top + edge, w, row_h);
        let status = Rect::new(x, switch_row.y + row_h, w, status_h);
        let mut y = status.y + status_h;
        let cross = self.px(56.0);
        let nets = shown
            .into_iter()
            .map(|i| {
                let row = Rect::new(x, y, w, row_h);
                y += row_h;
                let forget = m.networks[i]
                    .saved
                    .then(|| Rect::new(x + w - cross, row.y, cross, row_h));
                (i, row, forget)
            })
            .collect();
        let hint = hint_text.map(|t| {
            let r = Rect::new(x, y, w, row_h);
            y += row_h;
            (r, t)
        });
        let rescan = rescan_on.then(|| Rect::new(x, y, w, row_h));
        WifiLayout {
            frame,
            switch_row,
            status,
            nets,
            hint,
            rescan,
        }
    }

    /// What a tap at (`x`, `y`) does while the menu is open.
    #[must_use]
    pub fn wifi_menu_hit(&self, l: &Layout, m: &WifiModel<'_>, x: i32, y: i32) -> WifiHit {
        if l.wifi_button.inset(-self.px(4.0)).contains(x, y) {
            return WifiHit::Button;
        }
        let g = self.wifi_menu_layout(l, m);
        if !g.frame.contains(x, y) {
            return WifiHit::Outside;
        }
        if g.switch_row.contains(x, y) {
            return WifiHit::Switch;
        }
        for (i, row, forget) in &g.nets {
            if forget.is_some_and(|f| f.contains(x, y)) {
                return WifiHit::Forget(*i);
            }
            if row.contains(x, y) {
                return WifiHit::Net(*i);
            }
        }
        if g.rescan.is_some_and(|r| r.contains(x, y)) {
            return WifiHit::Rescan;
        }
        WifiHit::Inside
    }

    /// The Wi-Fi button: off, on or connected, active while the menu is open.
    pub fn draw_wifi_button(&self, c: &mut Canvas, l: &Layout, t: Theme) {
        if !self.wifi_on {
            return;
        }
        let r = l.wifi_button;
        if self.wifi_menu {
            c.fill(r, t.wash);
            c.stroke(r, 2, t.ink);
        }
        draw_icon(c, self.wifi_state.icon(), r, t.ink);
    }

    /// Four bars, `lit` of them in ink and the rest light, in a box of
    /// `h` high whose bottom left corner is at (`x`, `bottom`).
    fn draw_signal(&self, c: &mut Canvas, x: i32, bottom: i32, lit: u8, t: Theme) {
        let (bw, gap) = (self.px(5.0).max(3), self.px(3.0).max(2));
        for i in 0..4 {
            let h = self.px(6.0 + 4.0 * i as f32);
            let r = Rect::new(x + i * (bw + gap), bottom - h, bw, h);
            c.fill(r, if i < i32::from(lit) { t.ink } else { t.rule });
        }
    }

    /// The width the signal bars take.
    fn signal_w(&self) -> i32 {
        4 * self.px(5.0).max(3) + 3 * self.px(3.0).max(2)
    }

    /// Draws the open menu over whatever is below it.
    pub fn draw_wifi_menu(&self, c: &mut Canvas, l: &Layout, m: &WifiModel<'_>, t: Theme) {
        if !self.wifi_menu {
            return;
        }
        let g = self.wifi_menu_layout(l, m);
        let line = self.px(1.5).max(2);
        let (ui, pad) = (self.ui_px(), self.px(16.0));
        let rule = Rect::new(0, 0, 0, 1.max(self.px(1.0)));
        c.fill(g.frame, t.paper);
        c.stroke(g.frame, line, t.ink);
        let baseline = |r: Rect, size: f32, face: Face| {
            r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6
        };

        // The switch row: the icon of the state, the word, the switch.
        let r = g.switch_row;
        let icon = Rect::new(
            r.x + pad,
            r.y + (r.h - self.px(44.0)) / 2,
            self.px(44.0),
            self.px(44.0),
        );
        draw_icon(c, self.wifi_state_of(m).icon(), icon, t.ink);
        let x = (icon.x + icon.w + self.px(8.0)) as f32;
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            ui,
            x,
            baseline(r, ui, Face::AtkinsonBold),
            &tr("Wi-Fi"),
            t.ink,
        );
        let band = Rect::new(r.x, r.y, r.w - pad, r.h);
        let target = switch::target(band, self.scale(), &self.fonts, ui);
        switch::draw(c, self.scale(), &self.fonts, ui, target, m.powered, t);

        // The state: two lines.
        let s = g.status;
        c.fill(Rect::new(s.x + pad, s.y, s.w - 2 * pad, rule.h), t.rule);
        let room = (s.w - 2 * pad) as f32;
        let (first, second): (String, String) = match (m.activity, m.ssid) {
            (Activity::TurningOn, _) => (tr("Turning on\u{2026}").into_owned(), String::new()),
            (Activity::TurningOff, _) => (tr("Turning off\u{2026}").into_owned(), String::new()),
            (Activity::Joining, _) => (tr("Connecting\u{2026}").into_owned(), String::new()),
            (_, Some(ssid)) => (
                ssid.to_owned(),
                match (m.note, m.ip) {
                    (Some(n), _) => tr(n).into_owned(),
                    (None, Some(ip)) => format!("{} \u{b7} {ip}", tr("Connected")),
                    (None, None) => tr("Connected").into_owned(),
                },
            ),
            (_, None) => (
                tr(if m.powered { "Not connected" } else { "Off" }).into_owned(),
                m.note.map(|n| tr(n).into_owned()).unwrap_or_default(),
            ),
        };
        let a = Rect::new(s.x, s.y + self.px(8.0), s.w, s.h / 2 - self.px(8.0));
        let b = Rect::new(s.x, s.y + s.h / 2, s.w, s.h / 2 - self.px(8.0));
        let first = ellipsize(&self.fonts, Face::AtkinsonBold, ui, &first, room);
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            ui,
            (s.x + pad) as f32,
            baseline(a, ui, Face::AtkinsonBold),
            &first,
            t.ink,
        );
        let small = ui * 0.85;
        let second = ellipsize(&self.fonts, Face::Atkinson, small, &second, room);
        self.fonts.draw(
            c,
            Face::Atkinson,
            small,
            (s.x + pad) as f32,
            baseline(b, small, Face::Atkinson),
            &second,
            t.mute,
        );

        // The networks.
        for (i, row, forget) in &g.nets {
            let n = &m.networks[*i];
            c.fill(
                Rect::new(row.x + pad, row.y, row.w - 2 * pad, rule.h),
                t.rule,
            );
            if n.connected {
                c.fill(
                    Rect::new(row.x + 2, row.y + 1 + rule.h, row.w - 4, row.h - 1 - rule.h),
                    t.wash,
                );
            }
            self.draw_signal(c, row.x + pad, row.y + row.h / 2 + self.px(10.0), n.bars, t);
            let right = forget.map_or(row.x + row.w, |f| f.x);
            let lock = Rect::new(
                right - self.px(44.0) - self.px(4.0),
                row.y + (row.h - self.px(44.0)) / 2,
                self.px(44.0),
                self.px(44.0),
            );
            let name_x = row.x + pad + self.signal_w() + self.px(14.0);
            let name_w = (lock.x - name_x) as f32;
            let face = if n.connected {
                Face::AtkinsonBold
            } else {
                Face::Atkinson
            };
            let name = ellipsize(&self.fonts, face, ui, &n.ssid, name_w);
            self.fonts.draw(
                c,
                face,
                ui,
                name_x as f32,
                baseline(*row, ui, face),
                &name,
                t.ink,
            );
            if n.locked {
                draw_icon(c, Icon::Lock, lock, t.ink);
            }
            if let Some(f) = forget {
                draw_icon(c, Icon::Close, *f, t.mute);
            }
        }
        if let Some((r, text)) = &g.hint {
            c.fill(Rect::new(r.x + pad, r.y, r.w - 2 * pad, rule.h), t.rule);
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                (r.x + pad) as f32,
                baseline(*r, ui, Face::Atkinson),
                &tr(text),
                t.mute,
            );
        }
        if let Some(r) = g.rescan {
            c.fill(Rect::new(r.x + pad, r.y, r.w - 2 * pad, rule.h), t.rule);
            let icon = Rect::new(
                r.x + pad - self.px(8.0),
                r.y + (r.h - self.px(44.0)) / 2,
                self.px(44.0),
                self.px(44.0),
            );
            draw_icon(c, Icon::Refresh, icon, t.ink);
            let busy = m.activity == Activity::Scanning;
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                (icon.x + icon.w + self.px(8.0)) as f32,
                baseline(r, ui, Face::Atkinson),
                &tr(if busy {
                    "Scanning\u{2026}"
                } else {
                    "Scan again"
                }),
                if busy { t.mute } else { t.ink },
            );
        }
    }

    /// The state the model stands for.
    fn wifi_state_of(&self, m: &WifiModel<'_>) -> WifiState {
        match (m.powered, m.ssid) {
            (false, _) => WifiState::Off,
            (true, None) => WifiState::On,
            (true, Some(_)) => WifiState::Connected,
        }
    }

    /// The eye inside the right end of the title-bar field, while a
    /// password is typed.
    #[must_use]
    pub fn wifi_eye(&self, l: &Layout) -> Rect {
        let field_right = l.cancel_rect.x - self.px(24.0) + self.px(12.0);
        Rect::new(
            field_right - l.ok_rect.h,
            l.ok_rect.y,
            l.ok_rect.h,
            l.ok_rect.h,
        )
    }

    /// Adds to the title-bar field, while a password is typed: the eye
    /// (pressed while the password is shown), and, while the field is
    /// empty, a hint naming the network.
    pub fn draw_wifi_field(
        &self,
        c: &mut Canvas,
        l: &Layout,
        t: Theme,
        ssid: &str,
        empty: bool,
        shown: bool,
    ) {
        let ui = self.ui_px();
        let eye = self.wifi_eye(l);
        let field_x = self.px(12.0);
        // The field's own right end, under the eye, is made clear first.
        c.fill(Rect::new(eye.x, eye.y + 1, eye.w, eye.h - 2), t.wash);
        if shown {
            c.stroke(eye.inset(self.px(2.0)), 2, t.ink);
        }
        draw_icon(c, Icon::Eye, eye, t.ink);
        if empty {
            let base = l.title.y as f32
                + l.title.h as f32 / 2.0
                + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
            let x = (field_x + self.px(10.0) + 6) as f32;
            let hint = format!("{}: {ssid}", tr("Password"));
            let room = (eye.x - x as i32 - self.px(8.0)) as f32;
            let hint = ellipsize(&self.fonts, Face::Atkinson, ui, &hint, room);
            self.fonts
                .draw(c, Face::Atkinson, ui, x, base, &hint, t.mute);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::Keyboard;

    fn get(c: &Canvas, x: i32, y: i32) -> u8 {
        c.pixels[(y * c.width as i32 + x) as usize]
    }

    fn rows() -> Vec<NetRow> {
        let n = |ssid: &str, bars, locked, saved, connected| NetRow {
            ssid: ssid.to_owned(),
            bars,
            locked,
            saved,
            connected,
        };
        vec![
            n("Home", 4, true, true, true),
            n("Café Central", 3, false, false, false),
            n("Office-5G", 2, true, false, false),
            n(
                "A rather long network name that has to be cut short",
                1,
                true,
                true,
                false,
            ),
            n("Holiday house", 0, false, true, false),
        ]
    }

    fn model<'a>(nets: &'a [NetRow], powered: bool) -> WifiModel<'a> {
        WifiModel {
            powered,
            activity: Activity::Idle,
            ssid: powered.then_some("Home"),
            ip: Some("192.168.1.42"),
            note: None,
            networks: nets,
        }
    }

    fn screens() -> Vec<EditorScreen> {
        let mut out = Vec::new();
        for (w, h) in [(1440, 1920), (1920, 1440)] {
            for ui in [100u16, 150, 200] {
                let mut e = EditorScreen::new(w, h);
                e.cfg.ui_scale = ui;
                e.resize(w, h);
                e.wifi_on = true;
                // The front-light button is there on the Forma too.
                e.light_on = true;
                out.push(e);
            }
        }
        out
    }

    #[test]
    fn the_button_has_a_place_and_the_title_bar_does_not_overflow() {
        let k = Keyboard::default();
        for mut e in screens() {
            let off = e.layout(true, &k);
            e.wifi_on = false;
            let without = e.layout(true, &k);
            e.wifi_on = true;
            let l = e.layout(true, &k);
            assert_eq!(without.wifi_button, Rect::default());
            let b = l.wifi_button;
            assert!(b.w >= 44 && b.h >= 44, "a full touch target");
            // Between Graph's place and Options, both flush right as before.
            assert_eq!(l.options_button, without.options_button);
            assert!(b.x + b.w <= l.options_button.x && b.x >= 0);
            assert!(l.files_button.x > l.name_rect.x + l.name_rect.w);
            // Nothing overlaps and everything is inside the bar.
            let all = [
                l.back_button,
                l.files_button,
                l.preview_button,
                l.find_button,
                l.drawer_button,
                l.light_button,
                l.wifi_button,
                l.options_button,
            ];
            // With Wi-Fi and a light, the light button is third from the
            // right: Light, Wi-Fi, Settings.
            assert!(l.light_button.x + l.light_button.w <= l.wifi_button.x);
            assert_eq!(
                l.wifi_button.x - l.light_button.x,
                l.options_button.x - l.wifi_button.x
            );
            for (i, a) in all.iter().enumerate() {
                assert!(a.x >= 0 && a.x + a.w <= e.width as i32, "{a:?}");
                assert!(a.y >= 0 && a.y + a.h <= l.title.h);
                for o in &all[i + 1..] {
                    assert!(a.x + a.w <= o.x || o.x + o.w <= a.x, "{a:?} {o:?}");
                }
            }
            // The name keeps room: the icons shrink before it disappears.
            assert!(l.name_rect.w >= e.px(100.0), "{:?}", l.name_rect);
            // The other icons moved left to make room.
            assert!(off.options_button.x == l.options_button.x);
            assert!(l.find_button.x < without.options_button.x);
        }
    }

    #[test]
    fn a_tap_on_the_button_is_wifi_in_write_mode_and_preview() {
        let k = Keyboard::default();
        let b = sigma_core::Buffer::new("x");
        let st = sigma_core::SaveState::Saved;
        for e in screens() {
            let l = e.layout(true, &k);
            let m = crate::editor::Model {
                file_name: "a.md",
                buffer: &b,
                keyboard: &k,
                keys_on: true,
                save: &st,
                theme: Theme::DAY,
                sel_ui: None,
                rename: None,
            };
            let c = (
                l.wifi_button.x + l.wifi_button.w / 2,
                l.wifi_button.y + l.wifi_button.h / 2,
            );
            assert_eq!(e.hit(&m, c.0, c.1), crate::Hit::Wifi);
            assert_eq!(e.preview_hit(&k, c.0, c.1), crate::Hit::Wifi);
            // Every other title button still answers as itself.
            let o = l.options_button;
            assert_eq!(e.hit(&m, o.x + o.w / 2, o.y + o.h / 2), crate::Hit::Options);
        }
    }

    #[test]
    fn the_menu_sits_under_the_button_inside_the_screen_and_every_row_answers() {
        let k = Keyboard::default();
        let nets = rows();
        for e in screens() {
            let l = e.layout(true, &k);
            let m = model(&nets, true);
            let g = e.wifi_menu_layout(&l, &m);
            let (w, h) = (e.width as i32, e.height as i32);
            assert_eq!(g.frame.y, l.title.h, "just under the title bar");
            assert!(
                g.frame.x >= 0 && g.frame.x + g.frame.w <= w,
                "{:?}",
                g.frame
            );
            assert!(
                g.frame.y + g.frame.h <= h,
                "fits the screen: {:?} in {h}",
                g.frame
            );
            assert!(g.switch_row.h >= e.px(44.0));
            let centre = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
            let (x, y) = centre(g.switch_row);
            assert_eq!(e.wifi_menu_hit(&l, &m, x, y), WifiHit::Switch);
            let (x, y) = centre(g.status);
            assert_eq!(e.wifi_menu_hit(&l, &m, x, y), WifiHit::Inside);
            assert!(!g.nets.is_empty());
            let mut last = g.status.y + g.status.h;
            for (i, row, forget) in &g.nets {
                assert!(row.h >= e.px(44.0));
                assert_eq!(row.y, last, "rows follow each other");
                last = row.y + row.h;
                // The left of the row joins; a saved row has a cross.
                assert_eq!(
                    e.wifi_menu_hit(&l, &m, row.x + row.w / 3, row.y + row.h / 2),
                    WifiHit::Net(*i)
                );
                assert_eq!(forget.is_some(), nets[*i].saved);
                if let Some(f) = forget {
                    assert!(f.w >= e.px(44.0) && f.h >= e.px(44.0));
                    assert_eq!(
                        e.wifi_menu_hit(&l, &m, f.x + f.w / 2, f.y + f.h / 2),
                        WifiHit::Forget(*i)
                    );
                }
            }
            let r = g.rescan.expect("the scan row while on");
            assert_eq!(r.y, last);
            let (x, y) = centre(r);
            assert_eq!(e.wifi_menu_hit(&l, &m, x, y), WifiHit::Rescan);
            let b = l.wifi_button;
            assert_eq!(
                e.wifi_menu_hit(&l, &m, b.x + b.w / 2, b.y + b.h / 2),
                WifiHit::Button
            );
            assert_eq!(e.wifi_menu_hit(&l, &m, 5, h - 5), WifiHit::Outside);
            assert_eq!(
                e.wifi_menu_hit(&l, &m, g.frame.x + g.frame.w / 2, g.frame.y + g.frame.h + 6),
                WifiHit::Outside,
                "just below the menu"
            );
        }
    }

    #[test]
    fn with_the_radio_off_only_the_switch_and_the_state_show() {
        let k = Keyboard::default();
        let nets = rows();
        for e in screens() {
            let l = e.layout(true, &k);
            let m = model(&nets, false);
            let g = e.wifi_menu_layout(&l, &m);
            assert!(g.nets.is_empty() && g.rescan.is_none() && g.hint.is_none());
            assert_eq!(g.frame.h, 2 * e.px(4.0) + e.px(56.0) + e.px(76.0));
        }
    }

    #[test]
    fn a_long_list_is_cut_to_what_fits_the_screen() {
        let k = Keyboard::default();
        let many: Vec<NetRow> = (0..30)
            .map(|i| NetRow {
                ssid: format!("net {i}"),
                bars: 2,
                locked: true,
                saved: false,
                connected: false,
            })
            .collect();
        for e in screens() {
            let l = e.layout(true, &k);
            let g = e.wifi_menu_layout(&l, &model(&many, true));
            assert!(g.nets.len() <= 8 && !g.nets.is_empty());
            assert!(g.frame.y + g.frame.h <= e.height as i32);
        }
    }

    #[test]
    fn an_empty_list_says_why() {
        let k = Keyboard::default();
        for e in screens() {
            let l = e.layout(true, &k);
            let mut m = model(&[], true);
            let g = e.wifi_menu_layout(&l, &m);
            assert_eq!(g.hint.map(|h| h.1), Some("No networks found"));
            m.activity = Activity::Scanning;
            let g = e.wifi_menu_layout(&l, &m);
            assert_eq!(g.hint.map(|h| h.1), Some("Scanning\u{2026}"));
            let (r, _) = g.hint.unwrap();
            assert_eq!(
                e.wifi_menu_hit(&l, &m, r.x + 20, r.y + r.h / 2),
                WifiHit::Inside
            );
        }
    }

    #[test]
    fn the_menu_draws_framed_the_button_follows_the_state_and_nothing_draws_while_closed() {
        let k = Keyboard::default();
        let nets = rows();
        let t = Theme::DAY;
        let mut e = EditorScreen::new(1440, 1920);
        e.wifi_on = true;
        let l = e.layout(true, &k);
        let m = model(&nets, true);
        let g = e.wifi_menu_layout(&l, &m);
        let draw = |e: &EditorScreen| {
            let mut c = Canvas::new(1440, 1920, t.paper);
            e.draw_wifi_button(&mut c, &l, t);
            e.draw_wifi_menu(&mut c, &l, &m, t);
            c
        };
        let closed = draw(&e);
        assert_eq!(get(&closed, g.frame.x, g.frame.y + g.frame.h / 2), t.paper);
        e.wifi_menu = true;
        let open = draw(&e);
        assert_eq!(
            get(&open, g.frame.x, g.frame.y + g.frame.h / 2),
            t.ink,
            "framed"
        );
        // The button's three states are three different pictures.
        let b = l.wifi_button;
        let mut picture = |s: WifiState| {
            e.wifi_state = s;
            e.wifi_menu = false;
            let c = draw(&e);
            (b.y..b.y + b.h)
                .flat_map(|y| (b.x..b.x + b.w).map(move |x| (x, y)))
                .map(|(x, y)| get(&c, x, y))
                .collect::<Vec<_>>()
        };
        let (off, on, conn) = (
            picture(WifiState::Off),
            picture(WifiState::On),
            picture(WifiState::Connected),
        );
        assert_ne!(off, on);
        assert_ne!(on, conn);
        assert_ne!(off, conn);
        assert!(off.iter().any(|&p| p < 100), "the icon has ink");
        // The joined row is washed, the others are not.
        e.wifi_menu = true;
        let open = draw(&e);
        let (_, home, _) = g.nets[0];
        let (_, cafe, _) = g.nets[1];
        let probe = |r: Rect| get(&open, r.x + r.w - e.px(100.0), r.y + r.h - 3);
        assert_eq!(probe(home), t.wash);
        assert_eq!(probe(cafe), t.paper);
    }

    #[test]
    fn the_password_field_gets_an_eye_inside_its_right_end_and_a_hint_while_empty() {
        let k = Keyboard::default();
        let t = Theme::DAY;
        for e in screens() {
            let l = e.layout(true, &k);
            let eye = e.wifi_eye(&l);
            assert!(eye.w >= 44 && eye.x + eye.w <= l.cancel_rect.x, "{eye:?}");
            assert!(eye.x > e.px(12.0) + e.px(200.0), "the field keeps room");
            let draw = |empty, shown| {
                let mut c = Canvas::new(e.width, e.height, t.paper);
                e.draw_title(&mut c, &l, "", Some(&sigma_core::Buffer::new("")), t, true);
                e.draw_wifi_field(&mut c, &l, t, "Home", empty, shown);
                c
            };
            let (hidden, shown) = (draw(false, false), draw(false, true));
            assert_ne!(hidden.pixels, shown.pixels, "the eye shows when pressed");
            let hinted = draw(true, false);
            assert_ne!(hidden.pixels, hinted.pixels, "a hint while empty");
        }
    }
}
