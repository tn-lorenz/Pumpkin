use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use sha2::digest::block_buffer::Lazy;
use tokio::sync::{Mutex, RwLock};

use pumpkin_data::screen::WindowType;
use pumpkin_world::item::ItemStack;
use pumpkin_inventory::{container_click::{Click, ClickType, MouseClick}, Container, InventoryError, OpenContainer};
use pumpkin_util::{math::position::BlockPos, text::TextComponent};
use pumpkin_data::block::Block;
use pumpkin_protocol::{client::play::COpenScreen, codec::var_int::VarInt};

use crate::entity::player::Player;
use crate::plugin::api::Context;
use crate::server::Server;
    
/// Represents a click event in a container
pub struct ContainerClickEvent<'a> {
    /// The player who clicked in the container
    pub player: Arc<Player>,
    /// The click that was performed
    pub click: &'a Click,
    /// The container being clicked in
    pub container: Arc<Mutex<Box<dyn Container>>>,
    /// Whether the event is cancelled
    pub cancelled: bool,
}

/// Trait for handling container click events
#[async_trait]
pub trait ContainerClickListener: Send + Sync + 'static {
    /// Called when a player clicks in the container
    async fn on_click(&self, event: &mut ContainerClickEvent<'_>);
}

/// A custom container implementation that wraps a base container and tracks click events
pub struct CustomContainer<T: Container + 'static> {
    /// The base container being wrapped
    pub inner_container: T,
    /// The click listener for this container
    pub click_listener: Option<Arc<dyn ContainerClickListener>>,
    /// Container ID for tracking
    container_id: u64,
    window_type_override: Option<WindowType>,
}

impl<T: Container + 'static> CustomContainer<T> {
    /// Creates a new custom container
    pub fn new(inner_container: T, container_id: u64) -> Self {
        Self {
            inner_container,
            click_listener: None,
            container_id,
            window_type_override: None,
        }
    }

    /// Sets the click listener for this container
    pub fn set_click_listener(&mut self, listener: Arc<dyn ContainerClickListener>) {
        self.click_listener = Some(listener);
    }

     /// Sets the window type, overriding the inner container's window type
    pub fn set_window_type(&mut self, window_type: WindowType) {
        self.window_type_override = Some(window_type);
    }

    /// Gets the container ID
    pub fn get_container_id(&self) -> u64 {
        self.container_id
    }

    /// Gets a reference to the click listener if one exists
    pub fn get_click_listener(&self) -> Option<Arc<dyn ContainerClickListener>> {
        self.click_listener.clone()
    }
}

impl<T: Container + 'static> Container for CustomContainer<T> {
    fn window_type(&self) -> &'static WindowType {
        // This will require matching to get a static reference
        match self.window_type_override {
            Some(WindowType::Generic9x1) => &WindowType::Generic9x1,
            Some(WindowType::Generic9x2) => &WindowType::Generic9x2,
            Some(WindowType::Generic9x3) => &WindowType::Generic9x3,
            Some(WindowType::Generic9x4) => &WindowType::Generic9x4,
            Some(WindowType::Generic9x5) => &WindowType::Generic9x5,
            Some(WindowType::Generic9x6) => &WindowType::Generic9x6,
            _ => self.inner_container.window_type(),
        }
    }

    fn window_name(&self) -> &'static str {
        self.inner_container.window_name()
    }

    fn all_slots(&mut self) -> Box<[&mut Option<ItemStack>]> {
        self.inner_container.all_slots()
    }

    fn all_slots_ref(&self) -> Box<[Option<&ItemStack>]> {
        self.inner_container.all_slots_ref()
    }

    fn all_combinable_slots(&self) -> Box<[Option<&ItemStack>]> {
        self.inner_container.all_combinable_slots()
    }

    fn all_combinable_slots_mut(&mut self) -> Box<[&mut Option<ItemStack>]> {
        self.inner_container.all_combinable_slots_mut()
    }

    fn internal_pumpkin_id(&self) -> u64 {
        self.container_id
    }

    fn craft(&mut self) -> bool {
        self.inner_container.craft()
    }

    fn crafting_output_slot(&self) -> Option<usize> {
        self.inner_container.crafting_output_slot()
    }

    fn slot_in_crafting_input_slots(&self, slot: &usize) -> bool {
        self.inner_container.slot_in_crafting_input_slots(slot)
    }
}

/// ContainerManager to create and manage containers
pub struct ContainerManager {
    server: Arc<Server>,
}

impl ContainerManager {
    /// Creates a new ContainerManager
    pub fn new(server: Arc<Server>) -> Self {
        Self { server }
    }

    /// Creates a new container of the specified type
    pub async fn create_container<C: Container + Default + 'static>(
        &self, 
        window_type: WindowType  // Changed from &'static WindowType to WindowType
    ) -> CustomContainer<C> {
        let container_id = self.server.new_container_id() as u64;
        let inner_container = C::default();
        let mut custom_container = CustomContainer::new(inner_container, container_id);
        custom_container.set_window_type(window_type);  // No need for & anymore
        custom_container
    }

    /// Opens a container for a player
    pub async fn open_container<T: Container + 'static>(&self, player: Arc<Player>, container: CustomContainer<T>, location: Option<BlockPos>, block: Option<Block>) -> Result<u64, InventoryError> {
        let container_id = container.get_container_id();
        
        let window_type = *container.window_type();
        
        // Check if container has a click listener
        let has_listener = container.click_listener.is_some();
        
        let click_listener = container.get_click_listener();
        
        // Register container
        {
            let boxed_container: Box<dyn Container> = Box::new(container);
            let mut open_containers = self.server.open_containers.write().await;
            
            let open_container = OpenContainer {
                players: vec![player.entity_id()],
                container: Arc::new(Mutex::new(boxed_container)),
                location,
                block,
            };
            
            open_containers.insert(container_id, open_container);
            
            // Register click listener
            if let Some(listener) = click_listener {
                self.server.container_listeners.write().await.insert(container_id, listener);
                
                let verify = self.server.container_listeners.read().await;
            }
        }
        
        player.open_container.store(Some(container_id));
        
        // Open UI
        self.open_container_ui(player.clone(), window_type, container_id).await?;
        
        Ok(container_id)
    }
    
    // New helper function to open container UI
    async fn open_container_ui(&self, player: Arc<Player>, window_type: WindowType, container_id: u64) -> Result<(), InventoryError> {
        // Step 1: Set up the container UI
        {
            let mut inventory = player.inventory().lock().await;
            inventory.increment_state_id();
            inventory.total_opened_containers += 1;
            
            // Send the open screen packet directly
            let title = TextComponent::text("Container");
            player.client
                .enqueue_packet(&COpenScreen::new(
                    inventory.total_opened_containers.into(),
                    VarInt(window_type as i32),
                    &title,
                ))
                .await;
        } // Release inventory lock
        
        // Step 2: Set the container content
        {
            let container = player.get_open_container(&self.server).await;
            if let Some(container) = container {
                let mut locked_container = container.lock().await;
                player.set_container_content(Some(&mut *locked_container)).await;
            }
        }
        
        Ok(())
    }
    
    /// Closes a container for a player
    pub async fn close_container(&self, player: Arc<Player>) -> Result<(), InventoryError> {
        if let Some(container_id) = player.open_container.load() {
            // Remove player from container
            let mut open_containers = self.server.open_containers.write().await;
            if let Some(container) = open_containers.get_mut(&container_id) {
                container.remove_player(player.entity_id());
                
                // If no players are using the container, remove it
                if container.get_number_of_players() == 0 {
                    open_containers.remove(&container_id);
                    // Also remove from listener registry
                    self.server.container_listeners.write().await.remove(&container_id);
                }
            }
            
            // Clear player's open container reference
            player.open_container.store(None);
            
            // Close container in client
            player.close_container().await;
        }
        
        Ok(())
    }
    
    /// Registers a click handler for a container
    pub async fn register_container_click_handler(&self, container_id: u64, listener: Arc<dyn ContainerClickListener>) -> Result<(), InventoryError> {
        self.server.container_listeners.write().await.insert(container_id, listener);
        Ok(())
    }
}

/// Extension trait for Context to easily access the container manager
#[async_trait]
pub trait ContainerContextExtension {
    /// Gets or creates a container manager for this context
    async fn container_manager(&self) -> ContainerManager;
    
    /// Creates a container of the specified type
    async fn create_container<C: Container + Default + 'static>(&self, window_type: WindowType) -> CustomContainer<C>;
    
    /// Opens a container for a player
    async fn open_container<T: Container + 'static>(&self, player: Arc<Player>, container: CustomContainer<T>, location: Option<BlockPos>, block: Option<Block>) -> Result<u64, InventoryError>;
}

#[async_trait]
impl ContainerContextExtension for Context {
    async fn container_manager(&self) -> ContainerManager {
        ContainerManager::new(self.server.clone())
    }
    
    async fn create_container<C: Container + Default + 'static>(&self, window_type: WindowType) -> CustomContainer<C> {
        self.container_manager().await.create_container::<C>(window_type).await
    }
    
    async fn open_container<T: Container + 'static>(&self, player: Arc<Player>, container: CustomContainer<T>, location: Option<BlockPos>, block: Option<Block>) -> Result<u64, InventoryError> {
        self.container_manager().await.open_container(player, container, location, block).await
    }
}

/// Implementation for a basic chest container
pub struct ChestContainer {
    slots: Vec<Option<ItemStack>>,
    title: &'static str,
    window_type: &'static WindowType,
}

impl Default for ChestContainer {
    fn default() -> Self {
        Self {
            slots: vec![None; 27],  // 9x3 = 27 slots
            title: "Chest",
            window_type: &WindowType::Generic9x3,
        }
    }
}

impl ChestContainer {
    pub fn new(title: &'static str, window_type: &'static WindowType) -> Self {
        let slot_count = match window_type {
            WindowType::Generic9x1 => 9,
            WindowType::Generic9x2 => 18,
            WindowType::Generic9x3 => 27,
            WindowType::Generic9x4 => 36,
            WindowType::Generic9x5 => 45,
            WindowType::Generic9x6 => 54,
            _ => 27, // Default to 9x3 for unsupported types
        };

        Self {
            slots: vec![None; slot_count],
            title,
            window_type,
        }
    }
    
    pub fn set_item(&mut self, slot: usize, item: Option<ItemStack>) -> Result<(), InventoryError> {
        if slot < self.slots.len() {
            self.slots[slot] = item;
            Ok(())
        } else {
            Err(InventoryError::InvalidSlot)
        }
    }
}

impl Container for ChestContainer {
    fn window_type(&self) -> &'static WindowType {
        self.window_type
    }

    fn window_name(&self) -> &'static str {
        self.title
    }

    fn all_slots(&mut self) -> Box<[&mut Option<ItemStack>]> {
        self.slots.iter_mut().collect()
    }

    fn all_slots_ref(&self) -> Box<[Option<&ItemStack>]> {
        self.slots.iter().map(|slot| slot.as_ref()).collect()
    }
}

// Process a container click event
// Call this function from the Player::handle_click_container method
pub async fn process_container_click(player: Arc<Player>, click: &Click, container_id: u64, server: Arc<Server>) -> bool {
    
    // Check for listeners
    let listener = {
        let listeners = server.container_listeners.read().await;
        
        if let Some(l) = listeners.get(&container_id) {
            l.clone()
        } else {
            return false;
        }
    };
    
    // Get container
    if let Some(container) = player.get_open_container(&server).await {
        
        // Create event
        let mut event = ContainerClickEvent {
            player: player.clone(),
            click,
            container,
            cancelled: false,
        };
        
        // Call listener
        listener.on_click(&mut event).await;
        
        return event.cancelled;
    } else {
        log::info!("Failed to get container reference for player");
    }
    
    false
}