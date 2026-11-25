//! Screens and HUD while connected: menu titles, respawn, loading, the connect screen, status.

use super::*;

impl Game<'_> {
    /// The type of the menu a server opened (`MenuType`'s id).
    pub(super) fn remote_menu_type_body(&self) -> Option<&str> {
        Some(self.remote.as_ref()?.menu.as_ref()?.1.as_str())
    }
    /// The title the server gave the menu it opened.
    pub(super) fn remote_menu_title_body(&self) -> Option<horizon_client::text::Text> {
        self.remote.as_ref()?.menu_title.clone()
    }
    /// A language change reaches the server-sent components too: their titles
    /// are resolved when the packet arrives, with the session's language.
    pub(super) fn refresh_remote_language_body(&mut self) {
        if let Some(lang) = horizon_client::lang::current()
            && let Some(r) = self.remote.as_mut()
        {
            r.lang = Arc::new(lang.map().clone());
        }
    }
    /// A server's chest-like menu (`MenuType.GENERIC_9xN`): its rows and title.
    pub(super) fn remote_chest_menu_body(&self) -> Option<(usize, horizon_client::text::Text)> {
        let r = self.remote.as_ref()?;
        let rows = r
            .menu
            .as_ref()?
            .1
            .strip_prefix("minecraft:generic_9x")?
            .parse()
            .ok()?;
        Some((
            rows,
            r.menu_title
                .clone()
                .unwrap_or_else(horizon_client::text::Text::empty),
        ))
    }
    /// ServerboundPlayerCommandPacket STOP_SLEEPING.
    pub(super) fn remote_stop_sleeping_body(&mut self) {
        let id = self.remote.as_ref().map_or(0, |r| r.entity_id);
        self.send(S::PlayerCommand(play::PlayerCommand {
            id,
            action: 0,
            data: 0,
        }));
    }
    pub(super) fn remote_respawn_body(&mut self) {
        self.send(S::ClientCommand(play::ClientCommand { action: 0 }));
    }

    /// A connection's screen is over the world: connecting, reconfiguring, or
    /// the level loading screen, a portal's included (`remote_overlay`).
    pub(super) fn remote_screen_up_body(&self) -> bool {
        !self.replay.playing()
            && self
                .remote
                .as_ref()
                .is_some_and(|r| self.player.is_none() || r.level_load.is_active())
    }
    /// Connecting / loading screens; true when it covered the frame.
    pub(super) fn remote_overlay_body(&mut self, batch: &mut HudBatch) -> bool {
        if !self.remote_screen_up() {
            return false;
        }
        let Some(r) = &self.remote else { return false };
        let loading = r.level_load.is_active();
        if self.player.is_some() && !loading {
            return false;
        }
        let (w, h) = (batch.width, batch.height);
        if let Some(since) = r.reconfiguring {
            // ServerReconfigScreen narrates its title.
            horizon_client::widgets::begin_screen(|| {
                horizon_client::widgets::ScreenInfo::new(
                    "reconfiguring",
                    super::super::screen_info::tr("connect.reconfiguring", &[]),
                )
            });
            self.draw_reconfiguring(batch, since);
            return true;
        }
        if r.joined || loading {
            // LevelLoadingScreen: the world's terrain comes in. A server's
            // chunk status view and progress are the integrated server's alone
            // (`LevelLoadTracker.statusView`, `hasProgress`), and so is what
            // the narrator says of it.
            let progress = self
                .integrated
                .as_ref()
                .map(|i| i.load_view())
                .filter(|v| v.has_progress())
                .map(|v| v.server_progress());
            horizon_client::widgets::begin_screen(|| {
                super::super::screen_info::level_loading(progress, &super::super::screen_info::tr)
            });
            if self.draw_dimension_transition_screen(
                batch,
                &r.level_load,
                progress.map(|_| self.loading_progress),
            ) {
                return true;
            }
            self.draw_level_loading_background(batch, r.level_load.reason);
            let atlas = &self.gui.atlas;
            match self.integrated.as_ref().map(|i| i.load_view()) {
                Some(view) => {
                    let status = |x: i32, z: i32| view.status(x, z);
                    let progress = view.has_progress().then_some(self.loading_progress);
                    horizon_client::level_loading::draw(batch, atlas, Some(&status), progress);
                }
                None => horizon_client::level_loading::draw(batch, atlas, None, None),
            }
            return true;
        }
        let cancel = self.connect_cancel((w, h)).is_some();
        // ConnectScreen.extractRenderState: "Joining the world" every 2 s (a
        // server's: single player's isn't joined through ConnectScreen).
        if self.integrated.is_none()
            && let Some(r) = self.remote.as_mut()
            && r.joining_narrated
                .is_none_or(|t| t.elapsed() > Duration::from_millis(2000))
        {
            r.joining_narrated = Some(Instant::now());
            self.say_system_now(&horizon_client::text::Text::translate(
                "narrator.joining",
                &[],
            ));
        }
        let Some(r) = &self.remote else { return false };
        // ConnectScreen: `GameNarrator.NO_TITLE`, then its Cancel; single
        // player's server is joined under its LevelLoadingScreen.
        let single_player = self.integrated.is_some();
        horizon_client::widgets::begin_screen(|| {
            if single_player {
                super::super::screen_info::level_loading(None, &super::super::screen_info::tr)
            } else {
                horizon_client::widgets::ScreenInfo::new("connect", "")
            }
        });
        let atlas = &self.gui.atlas;
        let status = horizon_client::text::Text::untrusted(r.status.as_str());
        horizon_client::level_loading::draw_connecting(batch, atlas, &status, self.cursor, cancel);
        true
    }
    /// Connected or connecting with no player yet: `ConnectScreen`, then
    /// `LevelLoadingScreen`, over the panorama with its blur.
    pub(super) fn remote_loading_body(&self) -> bool {
        if self.replay.playing() {
            return false;
        }
        use horizon_client::level_loading::Reason;
        self.remote.as_ref().is_some_and(|r| {
            if r.level_load.is_active() {
                // The portal screens have no panorama, so no blur.
                r.level_load.reason == Reason::Other
            } else {
                self.player.is_none()
            }
        })
    }
    /// `ServerReconfigScreen`: a vertical layout of cells padded by 10, the
    /// title (9 high) over Disconnect (150 x 20), centred. The title's and the
    /// button's top left corners.
    pub(super) fn reconfigure_layout_body(&self, (w, h): (f32, f32)) -> ((f32, f32), (f32, f32)) {
        let title = horizon_client::text::Text::translate("connect.reconfiguring", &[]);
        let title_w = self.gui.atlas.font.width_text(&title).ceil() as i32;
        let width = (title_w + 20).max(170);
        let left = ((w as i32 - width) as f32 * 0.5) as i32;
        let top = ((h as i32 - 69) as f32 * 0.5) as i32;
        let across = |child: i32| (10.0 + ((width - child - 10) as f32 - 10.0) * 0.5) as i32;
        (
            ((left + across(title_w)) as f32, (top + 10) as f32),
            ((left + across(150)) as f32, (top + 39) as f32),
        )
    }
    /// `ServerReconfigScreen` over the panorama; Disconnect wakes up after
    /// `DISCONNECT_TIME` (600 ticks).
    pub(super) fn draw_reconfiguring_body(&self, batch: &mut HudBatch, since: u64) {
        let atlas = &self.gui.atlas;
        let (w, h) = (batch.width, batch.height);
        horizon_client::ui::menu_background(batch, atlas, 0.0, 0.0, w, h, false);
        let title = horizon_client::text::Text::translate("connect.reconfiguring", &[]);
        let ((tx, ty), (bx, by)) = self.reconfigure_layout((w, h));
        batch.text_styled(atlas, &title, tx, ty, u32::MAX, true);
        // CommonComponents.GUI_DISCONNECT
        let label = horizon_client::text::Text::translate("menu.disconnect", &[]);
        let active = self.ticks.saturating_sub(since) >= 600;
        let hover = horizon_client::ui::hovered(self.cursor, bx, by, 150.0, 20.0);
        horizon_client::ui::button(batch, atlas, bx, by, 150.0, 20.0, &label, active, hover);
    }
    /// `ConnectScreen`'s Cancel: a server's (single player's server isn't
    /// joined through `ConnectScreen`), until the login is done.
    pub(super) fn connect_cancel_body(&self, (w, h): (f32, f32)) -> Option<(f32, f32)> {
        let r = self.remote.as_ref()?;
        (self.player.is_none() && !r.joined && self.integrated.is_none())
            .then(|| horizon_client::level_loading::connect_cancel(w, h))
    }
    /// A click on `ConnectScreen`: Cancel aborts the connection
    /// (`connect.aborted`) and goes back. Escape does nothing there
    /// (`shouldCloseOnEsc` is false).
    pub(super) fn connect_screen_click_body(&mut self) -> Result<()> {
        // ServerReconfigScreen's Disconnect, once it is active.
        if let Some(since) = self.remote.as_ref().and_then(|r| r.reconfiguring) {
            let (_, (x, y)) = self.reconfigure_layout(self.gui_size());
            if self.ticks.saturating_sub(since) >= 600
                && horizon_client::ui::hovered(self.cursor, x, y, 150.0, 20.0)
            {
                self.audio.play_button_click();
                self.leave_server(None)?;
            }
            return Ok(());
        }
        let Some((x, y)) = self.connect_cancel(self.gui_size()) else {
            return Ok(());
        };
        if horizon_client::ui::hovered(self.cursor, x, y, 200.0, 20.0) {
            self.audio.play_button_click();
            self.leave_server(None)?;
        }
        Ok(())
    }
    /// The server's death message; chat uses the shared ChatComponent.
    pub(super) fn remote_hud_body(&self, batch: &mut HudBatch) {
        let Some(r) = &self.remote else { return };
        let atlas = &self.gui.atlas;
        let w = batch.width;
        if self.screen == Screen::Dead
            && let Some(message) = &r.death_message
        {
            // DeathScreen draws the styled cause component.
            let tw = atlas.font.width_text(message);
            batch.text_styled(
                atlas,
                message,
                ((w - tw) / 2.0).floor(),
                85.0,
                u32::MAX,
                true,
            );
        }
    }
    /// Shared flag 6 of an entity on a server's client (`None` without a server).
    pub(super) fn remote_glowing_body(&self, id: u32) -> Option<bool> {
        let r = self.remote.as_ref()?;
        Some(r.glowing.contains(&(id as i32)))
    }
    /// The profile name of another player the server sent, by entity id.
    pub(super) fn remote_player_name_body(&self, id: u32) -> Option<&str> {
        let r = self.remote.as_ref()?;
        r.level_player_name(id as i32, &r.player_uuid(id)?)
    }
    pub(super) fn remote_player_uuid_body(&self, id: u32) -> Option<u128> {
        let remote = self.remote.as_ref()?;
        if self.player_model.as_ref().is_some_and(|m| m.id == id) {
            Some(remote.profile)
        } else {
            remote.level_players.get(&(id as i32)).copied()
        }
    }
    /// Status for `horizon-ctl status`.
    /// The other players' entities in the level and their names (their
    /// profiles', which outlive the player list's entries).
    pub(super) fn remote_player_names_body(&self) -> Vec<(u32, String)> {
        let Some(r) = self.remote.as_ref() else {
            return Vec::new();
        };
        r.level_players
            .iter()
            .filter(|(_, uuid)| **uuid != r.profile)
            .filter_map(|(id, uuid)| Some((*id as u32, r.level_player_name(*id, uuid)?.to_owned())))
            .collect()
    }
    /// `ClientClockManager.tick`: the clock moves on by the game time that
    /// passed since its last tick, at its rate. A game time the server sets
    /// forward (its sync every second) moves the clock with it.
    pub(super) fn remote_clock_tick_body(&mut self) {
        let Some(r) = self.remote.as_mut() else {
            return;
        };
        let delta = self.world.game_time - r.clock_game_time;
        r.clock_game_time = self.world.game_time;
        for clock in r.clocks.values_mut() {
            let _ = client_level_tick::advance_clock(clock, delta);
        }
        self.world.day_time = r.clock_time(r.default_clock);
    }
}
