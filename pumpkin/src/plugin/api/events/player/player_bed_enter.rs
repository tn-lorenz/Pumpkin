use crate::entity::player::Player;
use crate::plugin::player::PlayerEvent;
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// Called when a player attempts to enter a bed.
///
/// This event can be cancelled to prevent the player from entering the bed.
///
/// As there are no event `Result`s implemented yet, I won't implement the methods to manipulate them for this event.
#[cancellable]
#[derive(Event, Clone)]
pub struct PlayerBedEnterEvent {
    /// The player who is attempting to enter the bed.
    pub player: Arc<Player>,

    /// The bed block being entered.
    pub bed: Block,

    /// Represents the outcome of this event.
    pub bed_enter_result: BedEnterResult,
}

impl PlayerBedEnterEvent {
    /// Creates a new `PlayerBedEnterEvent`.
    ///
    /// # Arguments
    ///
    /// * `player` - The player entering the bed.
    /// * `bed` - The block representing the bed.
    /// * `bed_enter_result` - Describes whether the attempt was successful or not and why.
    pub fn new(player: Arc<Player>, bed: Block, bed_enter_result: BedEnterResult) -> Self {
        Self {
            player,
            bed,
            bed_enter_result,
            cancelled: false,
        }
    }

    /// Returns the bed block the player is leaving.
    pub fn get_bed(&self) -> Block {
        self.bed
    }

    /// Returns `true` if the player's spawn location should be set to the bed's position.
    // TODO
    pub fn get_bed_enter_result(&self) -> BedEnterResult {
        self.bed_enter_result
    }

    pub fn use_bed(&self) {
        // todo()!
    }

    pub fn set_use_bed(&self) {
        // todo()!
    }
}

impl PlayerEvent for PlayerBedEnterEvent {
    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }
}

#[derive(Clone)]
pub enum BedEnterResult {
    Ok,
    NotPossibleHere,
    NotPossibleNow,
    TooFarAway,
    Obstructed,
    NotSafe,
    OtherProblem,
}
