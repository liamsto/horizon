//! The old `ServerPlayer` retained across `restoreFrom`, independently of its replacement.
use super::Game;
use mod_api::__private::game::Fields;
use mod_api::{Identifier, LevelRef, OriginalPlayerRef};
use std::sync::atomic::{AtomicU64, Ordering};

/// Per-player attachment state and the stack of originals retained during clone hooks.
#[derive(Default)]
pub(super) struct PlayerState {
    pub(super) attachments: horizon_mod_runtime::attachments::ResidentAttachments,
    pub(super) originals: Vec<OriginalPlayerSnapshot>,
}

pub(super) struct OriginalPlayerSnapshot {
    token: u64,
    uuid: u128,
    position: [f64; 3],
    dimension: String,
    level: Option<LevelRef>,
    inventory: Vec<Option<horizon_items::items::ItemStack>>,
    armor: [Option<horizon_items::items::ItemStack>; 4],
    offhand: Option<horizon_items::items::ItemStack>,
    health: f32,
    experience: mod_api::PlayerExperience,
    effects: Vec<mod_api::ActiveEffect>,
    pub(super) attachments: mod_api::Attachments,
}

impl OriginalPlayerSnapshot {
    pub(super) fn token(&self) -> u64 {
        self.token
    }
    pub(super) fn uuid(&self) -> u128 {
        self.uuid
    }
}

impl Game<'_> {
    /// `ServerPlayer.restoreFrom`: retain the old object before replacing any data.
    pub(in crate::game) fn capture_original_player(
        &self,
        conn: u32,
    ) -> Option<OriginalPlayerSnapshot> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        if !mod_api::__private::listened(mod_api::HookId::PlayerClone) {
            return None;
        }
        let guest = self.lan.as_ref()?.hub.guest(conn)?;
        let token = NEXT
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .expect("original player identity exhausted");
        Some(OriginalPlayerSnapshot {
            token,
            uuid: guest.uuid,
            position: guest.position,
            dimension: guest.dimension.clone(),
            level: self.level(&guest.dimension),
            inventory: guest.inventory.clone(),
            armor: guest.armor.clone(),
            offhand: guest.offhand.clone(),
            health: guest.health,
            experience: mod_api::PlayerExperience::from(
                mod_api::__private::built::PlayerExperience {
                    level: guest.xp_level,
                    progress: guest.xp_progress,
                    total: guest.xp_total,
                },
            ),
            effects: guest
                .effects
                .list
                .iter()
                .map(super::native_living_effects::active_effect)
                .collect(),
            attachments: (*guest.attachments).clone(),
        })
    }

    pub(in crate::game) fn with_original_player(
        &mut self,
        original: OriginalPlayerSnapshot,
        call: impl FnOnce(&mut Self, OriginalPlayerRef),
    ) {
        let handle = mod_api::__private::game::original_player_ref(original.token);
        self.player_state.originals.push(original);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(self, handle)));
        self.player_state.originals.pop();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    fn original_player_snapshot(
        &self,
        original: OriginalPlayerRef,
    ) -> Option<&OriginalPlayerSnapshot> {
        self.player_state
            .originals
            .iter()
            .rev()
            .find(|snapshot| snapshot.token == original.fields())
    }

    pub(super) fn original_player_uuid(&self, original_player: OriginalPlayerRef) -> Option<u128> {
        Some(self.original_player_snapshot(original_player)?.uuid)
    }
    pub(super) fn original_player_position(
        &self,
        original_player: OriginalPlayerRef,
    ) -> Option<[f64; 3]> {
        Some(self.original_player_snapshot(original_player)?.position)
    }
    pub(super) fn original_player_dimension(
        &self,
        original_player: OriginalPlayerRef,
    ) -> Option<Identifier> {
        Identifier::parse(&self.original_player_snapshot(original_player)?.dimension).ok()
    }
    pub(super) fn original_player_level(
        &self,
        original_player: OriginalPlayerRef,
    ) -> Option<LevelRef> {
        self.original_player_snapshot(original_player)?.level
    }
    pub(super) fn original_player_health(&self, original_player: OriginalPlayerRef) -> Option<f32> {
        Some(self.original_player_snapshot(original_player)?.health)
    }
    pub(super) fn original_player_experience(
        &self,
        original_player: OriginalPlayerRef,
    ) -> Option<mod_api::PlayerExperience> {
        Some(self.original_player_snapshot(original_player)?.experience)
    }
    pub(super) fn original_player_effects(
        &self,
        original_player: OriginalPlayerRef,
    ) -> Option<Vec<mod_api::ActiveEffect>> {
        Some(
            self.original_player_snapshot(original_player)?
                .effects
                .clone(),
        )
    }
    pub(super) fn original_player_inventory_slot(
        &self,
        original_player: OriginalPlayerRef,
        slot: i32,
    ) -> Option<horizon_items::items::ItemStack> {
        let snapshot = self.original_player_snapshot(original_player)?;
        let item = match slot {
            0..=35 => snapshot.inventory.get(slot as usize)?.as_ref(),
            100..=103 => snapshot.armor[(103 - slot) as usize].as_ref(),
            -106 => snapshot.offhand.as_ref(),
            _ => None,
        }?;
        Some(item.clone())
    }
}

#[cfg(test)]
mod tests {
    
    use crate::game::native_guest_turn_tests::with_guests;
    
    

    #[test]
    #[ignore = "daily"]
    fn unsubscribed_clone_does_not_capture() {
        mod_platform::testing::with_mods(Vec::<mod_api::ModEntry>::new().into(), || {
            with_guests(|game| assert!(game.capture_original_player(12).is_none()));
        });
    }
}
