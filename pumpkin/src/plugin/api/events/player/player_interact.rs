use super::PlayerEvent;
use crate::entity::player::Player;
use pumpkin_data::item::Item;
use pumpkin_macros::{Event, cancellable};
use pumpkin_world::item::ItemStack;
use std::sync::Arc;

/// An event that occurs when a `Player` interacts with a `Block` using their hand.
/// This event does not consider interactions through block movement, eg pressure plates, tripwire hooks, sculk sensors etc.
///
/// If the event is cancelled, the interaction  will not happen.
///
/// This event contains information about the player, the type of interaction (including whether the player is sneaking or not), the `Block` they are interacting with,
/// the `ItemStack` they are interacting using, the block face (`BlockDirection`) they are interacting with, and the `BlockPos` of the interaction.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerInteractEvent {
    /// The player who attempted to interact.
    pub player: Arc<Player>,

    /// The ItemStack the player is interacting using
    pub item_stack: ItemStack,
}

impl PlayerInteractEvent {
    /// Creates a new instance of `PlayerInteractEvent`.
    ///
    /// # Arguments
    /// - `player`: A reference to the player who interacted.
    /// - `action`: The type of interaction performed.
    /// - `block`: The block the player is interacting with.
    /// - `block_face`: The face of the block the player is interacting with.
    /// - `item`: The `ItemStack` the player is interacting using.
    /// - `position`: The position of the block being interacted with.
    /// - `cancelled`: A boolean indicating whether the interaction should be cancelled.
    ///
    /// # Returns
    /// A new instance of `PlayerInteractEvent`.
    pub fn new(
        player: Arc<Player>,
        item_stack: ItemStack,
        cancelled: bool,
    ) -> Self {
        Self {
            player,
            item_stack,
            cancelled,
        }
    }

    /// Gets the item being dropped.
    ///
    /// # Returns
    /// A reference to the `Item` being dropped.
    #[must_use]
    pub fn get_item(&self) -> &Item {
        &self.item_stack.item
    }
}

impl PlayerEvent for PlayerInteractEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}