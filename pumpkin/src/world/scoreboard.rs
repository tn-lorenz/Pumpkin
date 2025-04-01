/// A wrapper around RenderType to make it Copy-able
#[derive(Clone, Copy)]
pub enum RenderTypeWrapper {
    Integer,
    Hearts,
}

impl From<RenderTypeWrapper> for RenderType {
    fn from(wrapper: RenderTypeWrapper) -> Self {
        match wrapper {
            RenderTypeWrapper::Integer => RenderType::Integer,
            RenderTypeWrapper::Hearts => RenderType::Hearts,
        }
    }
}

impl From<RenderType> for RenderTypeWrapper {
    fn from(rt: RenderType) -> Self {
        match rt {
            RenderType::Integer => RenderTypeWrapper::Integer,
            RenderType::Hearts => RenderTypeWrapper::Hearts,
        }
    }
}use std::collections::HashMap;

use pumpkin_data::scoreboard::ScoreboardDisplaySlot;
use pumpkin_protocol::{
    client::play::{CDisplayObjective, CUpdateObjectives, CUpdateScore, RenderType, Mode},
    codec::var_int::VarInt,
};
use pumpkin_util::text::TextComponent;

use crate::world::World;

/// Represents a scoreboard entry with text and score
pub struct ScoreboardEntry {
    pub text: TextComponent,
    pub score: i32,
}

/// Builder for creating a scoreboard
pub struct ScoreboardBuilder {
    name: String,
    display_name: TextComponent,
    entries: Vec<ScoreboardEntry>,
    display_slot: ScoreboardDisplaySlot,
    render_type: RenderTypeWrapper,
}

impl ScoreboardBuilder {
    /// Create a new scoreboard builder with the given name and display name
    pub fn new(name: impl Into<String>, display_name: TextComponent) -> Self {
        Self {
            name: name.into(),
            display_name,
            entries: Vec::new(),
            display_slot: ScoreboardDisplaySlot::Sidebar,
            render_type: RenderTypeWrapper::Integer,
        }
    }

    /// Set the display slot for the scoreboard
    pub fn display_slot(mut self, slot: ScoreboardDisplaySlot) -> Self {
        self.display_slot = slot;
        self
    }

    /// Set the render type for the scoreboard
    pub fn render_type(mut self, render_type: RenderType) -> Self {
        self.render_type = RenderTypeWrapper::from(render_type);
        self
    }

    /// Add an entry to the scoreboard
    pub fn add_entry(mut self, text: TextComponent, score: i32) -> Self {
        self.entries.push(ScoreboardEntry {
            text,
            score,
        });
        self
    }
    
    /// Add a blank line to the scoreboard (uses empty text with a score)
    pub fn add_blank_line(self, score: i32) -> Self {
        self.add_entry(TextComponent::text(""), score)
    }

    /// Build and display the scoreboard to all players in the world
    pub async fn build_and_display(self, world: &World) -> Scoreboard {
        let mut scoreboard = Scoreboard::new();
        
        // Create the objective
        let objective = ScoreboardObjective {
            name: &self.name,
            display_name: self.display_name.clone(),
            render_type: self.render_type,
        };
        
        // Add the objective to the scoreboard
        scoreboard.add_objective(world, &objective).await;

        // Set the objective to display in the specified slot
        world.broadcast_packet_all(&CDisplayObjective::new(
            self.display_slot,
            self.name.clone(),
        )).await;
        
        // Create and update each score
        for entry in self.entries {
            let entity_name = format!("entry_{}", entry.score);
            let score = ScoreboardScore {
                entity_name: &entity_name,
                objective_name: &self.name,
                value: VarInt(entry.score),
                display_name: Some(entry.text.clone()),
            };
            
            scoreboard.update_score(world, &score).await;
        }
        
        scoreboard
    }
}

/// External version of ScoreboardObjective
#[derive(Clone)]
pub struct ScoreboardObjective<'a> {
    pub name: &'a str,
    pub display_name: TextComponent,
    pub render_type: RenderTypeWrapper,
}

/// External version of ScoreboardScore
#[derive(Clone)]
pub struct ScoreboardScore<'a> {
    pub entity_name: &'a str,
    pub objective_name: &'a str,
    pub value: VarInt,
    pub display_name: Option<TextComponent>,
}

/// Internal version for storage
#[derive(Clone)]
struct StoredObjective {
    name: String,
    display_name: TextComponent,
    render_type: RenderTypeWrapper,
}

/// Internal version for storage
#[derive(Clone)]
struct StoredScore {
    entity_name: String,
    objective_name: String,
    value: VarInt,
    display_name: Option<TextComponent>,
}

/// Enhanced Scoreboard implementation with better management capabilities
#[derive(Default)]
pub struct Scoreboard {
    objectives: HashMap<String, StoredObjective>,
    scores: HashMap<String, HashMap<String, StoredScore>>,
}

impl Scoreboard {
    #[must_use]
    pub fn new() -> Self {
        Self {
            objectives: HashMap::new(),
            scores: HashMap::new(),
        }
    }

    /// Create a new builder for easy scoreboard creation
    pub fn builder(name: impl Into<String>, display_name: TextComponent) -> ScoreboardBuilder {
        ScoreboardBuilder::new(name, display_name)
    }

    pub async fn add_objective(&mut self, world: &World, objective: &ScoreboardObjective<'_>) {
        if self.objectives.contains_key(objective.name) {
            log::warn!(
                "Tried to create an objective which already exists: {}",
                objective.name
            );
            return;
        }
        
        // Store the objective
        self.objectives.insert(
            objective.name.to_string(),
            StoredObjective {
                name: objective.name.to_string(),
                display_name: objective.display_name.clone(),
                render_type: objective.render_type,
            }
        );
        
        // Create empty scores map for this objective
        self.scores.insert(objective.name.to_string(), HashMap::new());
        
        // Send the packet to create the objective
        world
            .broadcast_packet_all(&CUpdateObjectives::new(
                objective.name.to_string(),
                Mode::Add,
                objective.display_name.clone(),
                RenderType::from(objective.render_type),
                None,
            ))
            .await;
    }

    pub async fn update_score(&mut self, world: &World, score: &ScoreboardScore<'_>) {
        if !self.objectives.contains_key(score.objective_name) {
            log::warn!(
                "Tried to place a score into an objective which does not exist: {}",
                score.objective_name
            );
            return;
        }
        
        // Store the score
        if let Some(scores) = self.scores.get_mut(score.objective_name) {
            scores.insert(
                score.entity_name.to_string(),
                StoredScore {
                    entity_name: score.entity_name.to_string(),
                    objective_name: score.objective_name.to_string(),
                    value: score.value,
                    display_name: score.display_name.clone(),
                }
            );
        }
        
        // Send the packet to update the score
        world
            .broadcast_packet_all(&CUpdateScore::new(
                score.entity_name.to_string(),
                score.objective_name.to_string(),
                score.value,
                score.display_name.clone(),
                None,
            ))
            .await;
    }
    
    /// Remove a scoreboard objective and all associated scores
    pub async fn remove_objective(&mut self, world: &World, objective_name: &str) {
        if !self.objectives.contains_key(objective_name) {
            log::warn!("Tried to remove objective that doesn't exist: {}", objective_name);
            return;
        }
        
        // Send the packet to remove the objective
        world
            .broadcast_packet_all(&CUpdateObjectives::new(
                objective_name.to_string(),
                Mode::Remove,
                TextComponent::text(""), // Not used when removing
                RenderType::Integer,    // Not used when removing
                None,                   // Not used when removing
            ))
            .await;
        
        // Remove from our local storage
        self.objectives.remove(objective_name);
        self.scores.remove(objective_name);
    }
    
    /// Update an objective's display name
    pub async fn update_objective_display_name(&mut self, world: &World, objective_name: &str, display_name: TextComponent) {
        if let Some(objective) = self.objectives.get_mut(objective_name) {
            objective.display_name = display_name.clone();
            
            // Send packet to update the display name
            world
                .broadcast_packet_all(&CUpdateObjectives::new(
                    objective_name.to_string(),
                    Mode::Update,
                    display_name,
                    RenderType::from(objective.render_type),
                    None,
                ))
                .await;
        }
    }
    
    /// Update a player's score
    pub async fn update_player_score(&mut self, world: &World, objective_name: &str, entity_name: &str, value: i32, display_name: Option<TextComponent>) {
        let score = ScoreboardScore {
            entity_name,
            objective_name,
            value: VarInt(value),
            display_name,
        };
        
        self.update_score(world, &score).await;
    }
}