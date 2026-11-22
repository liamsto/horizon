//! Typed text for the open screen's focused text box (`charTyped`).
use super::{Game, Screen};
use horizon_client::Result;

impl Game<'_> {
    /// Characters the open screen's focused text box takes: a key press's
    /// surviving characters (`charTyped`).
    /// Nothing here is a key — the caller drops control characters. `Err` when
    /// no text box of the screen takes them.
    pub(super) fn type_chars_into_screen(&mut self, text: &str) -> Result<()> {
        if self.book.is_some() {
            self.book_type(text);
            return Ok(());
        }
        if self.sign_edit.is_some() {
            self.sign_input(None, Some(text));
            return Ok(());
        }
        // The title's screens: the world name, the seed, a server's address.
        // A screen opened over the title (Options, say) keeps its own text.
        if title_owns_text(&self.screen, self.title.is_some())
            && let Some(title) = &mut self.title
        {
            title.type_text(text);
            return Ok(());
        }
        match self.screen {
            Screen::Chat => self.chat_type(text),
            Screen::CommandBlock => self.command_block_type(text),
            Screen::Social => self.social_type(text),
            Screen::BlockEditor => self.block_editor_type(text),
            Screen::DebugOptions => self.debug_options_type(text),
            Screen::Options => self.ui.options.type_text(&self.gui.atlas, text),
            // The port, or the game rules' search and number boxes.
            Screen::WorldOptions => self.world_options_text(Some(text)),
            Screen::Inventory | Screen::Creative if self.mod_browser_typing() => {
                self.mod_browser_text(Some(text), false)
            }
            Screen::Inventory if self.recipe_search_focused() => {
                self.recipe_text(Some(text), false)
            }
            // The anvil's rename box, while it has an input to name.
            Screen::Inventory if self.station_typing() => self.station_text(Some(text), false),
            Screen::Creative if self.creative_screen.tab().search => {
                // CreativeModeInventoryScreen.charTyped: the chat key that
                // opened the tab does not type its letter.
                if std::mem::take(&mut self.creative_ignore_text) {
                    return Ok(());
                }
                // As typed: replaces a selected query, keeps the length limit.
                self.creative_search_edit(Some(text))
            }
            _ => {
                return Err(
                    "typing needs the chat screen (key t tap), a sign or book, the language search, the creative search, the anvil or World Options".into(),
                );
            }
        }
        Ok(())
    }
}

/// The title's own screens (not one opened over it, as Options is) take the
/// text: the world name, the seed, a server's address.
pub(super) fn title_owns_text(screen: &Screen, title_up: bool) -> bool {
    title_up && *screen == Screen::Playing
}
