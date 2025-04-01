use async_trait::async_trait;
use crate::entity::{Entity, EntityBase, NBTStorage};
use crate::server::Server;
use pumpkin_data::entity::EntityType;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::client::play::{CSetEntityMetadata, MetaDataType, Metadata};
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::text::TextComponent;
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;
use tokio::sync::Mutex;
use crossbeam::atomic::AtomicCell;

/// Represents a Text Display entity in the Minecraft world.
///
/// Text Display entities show floating text in the world and were
/// added in Minecraft 1.19.4.
pub struct TextDisplayEntity {
    /// The underlying entity object.
    pub entity: Entity,
    /// The text content to be displayed.
    pub text: Mutex<TextComponent>,
    /// The background color of the display (ARGB format).
    pub background_color: AtomicCell<i32>,
    /// The opacity of the text (0-255).
    pub text_opacity: AtomicCell<u8>,
    /// The display width of the text.
    pub width: AtomicCell<f32>,
    /// The alignment of the text (0: center, 1: left, 2: right).
    pub alignment: AtomicCell<u8>,
    /// Whether the text should be shown with a shadow.
    pub has_shadow: AtomicCell<bool>,
    /// Whether the text should be shown with a see-through background.
    pub see_through: AtomicCell<bool>,
    /// Whether the text should be shown with the default background.
    pub has_default_background: AtomicCell<bool>,
    /// The line height for the text.
    pub line_height: AtomicCell<i32>,
    /// The transformation applied (billboard mode).
    pub billboard_mode: AtomicCell<BillboardMode>,
}

/// The billboard mode for the display entity (how it rotates relative to the player).
#[derive(Clone, Copy)]
pub enum BillboardMode {
    /// The display does not rotate with the player.
    Fixed = 0,
    /// The display always faces the player but is vertical.
    Vertical = 1,
    /// The display always faces the player but is horizontal.
    Horizontal = 2,
    /// The display always faces the player.
    Center = 3,
}

impl TextDisplayEntity {
    /// Creates a new [`TextDisplayEntity`].
    pub fn new(entity: Entity, text: TextComponent) -> Self {
        Self {
            entity,
            text: Mutex::new(text),
            background_color: AtomicCell::new(0x40000000), // Default semi-transparent black
            text_opacity: AtomicCell::new(255),            // Fully opaque
            width: AtomicCell::new(200.0),                   // Default width (0 = auto)
            alignment: AtomicCell::new(0),                 // Center
            has_shadow: AtomicCell::new(false),
            see_through: AtomicCell::new(false),
            has_default_background: AtomicCell::new(true), // Use default background
            line_height: AtomicCell::new(-1),              // Default line height
            billboard_mode: AtomicCell::new(BillboardMode::Vertical), // Face towards player
        }
    }

    /// Creates a new [`TextDisplayEntity`] at the specified position.
    pub async fn create(text: TextComponent, position: Vector3<f64>, world: &Arc<crate::world::World>) -> Arc<Self> {
        let entity = Entity::new(
            uuid::Uuid::new_v4(),
            world.clone(),
            position,
            EntityType::TEXT_DISPLAY,
            false, // Not invulnerable by default
        );
        
        let text_display = Arc::new(Self::new(entity, text));
        text_display.send_metadata().await;
        text_display
    }

    /// Sets the text to be displayed.
    pub async fn set_text(&self, text: TextComponent) {
        {
            let mut text_lock = self.text.lock().await;
            *text_lock = text.clone();
        }
        self.entity
            .send_meta_data(&[Metadata::new(23, MetaDataType::TextComponent, text)])
            .await;
    }

    /// Sets the background color of the display (ARGB format).
    pub async fn set_background_color(&self, color: i32) {
        self.background_color.store(color);
        self.entity
            .send_meta_data(&[Metadata::new(25, MetaDataType::Integer, color)])
            .await;
    }

    /// Sets the text opacity (0-255).
    pub async fn set_text_opacity(&self, opacity: u8) {
        self.text_opacity.store(opacity);
        self.entity
            .send_meta_data(&[Metadata::new(26, MetaDataType::Byte, opacity as i8)])
            .await;
    }

    /// Sets the display width of the text.
    pub async fn set_width(&self, width: f32) {
        self.width.store(width);
        // Convert width from f32 to i32 as required by the metadata
        let width_int = width as i32;
        self.entity
            .send_meta_data(&[Metadata::new(24, MetaDataType::Integer, VarInt(width_int))])
            .await;
    }

    /// Sets the line height for the text.
    pub async fn set_line_height(&self, height: i32) {
        self.line_height.store(height);
        // Note: Line height is not part of vanilla metadata
    }

    /// Sets the billboard mode (transformation).
    pub async fn set_billboard_mode(&self, mode: BillboardMode) {
        self.billboard_mode.store(mode);
        // Billboard mode is in the base Display class (inherited), typically at index 15
        self.entity
            .send_meta_data(&[Metadata::new(
                15, 
                MetaDataType::Byte, 
                mode as i8
            )])
            .await;
    }

    /// Updates display flags (alignment, shadow, see-through, background).
    pub async fn update_flags(&self) {
        let has_shadow = self.has_shadow.load();
        let see_through = self.see_through.load();
        let has_default_background = self.has_default_background.load();
        let alignment = self.alignment.load();
        
        // Flags are packed into a byte:
        // - Bit 0 (0x01): Has shadow
        // - Bit 1 (0x02): See through
        // - Bit 2 (0x04): Use default background
        // - Bits 3-4 (0x18): Alignment (0: center, 1: left (0x08), 2: right (0x10))
        let mut flags: i8 = 0;
        if has_shadow { flags |= 0x01; }
        if see_through { flags |= 0x02; }
        if has_default_background { flags |= 0x04; }
        
        // Handle alignment
        match alignment {
            0 => {}, // Center, no flags needed
            1 => flags |= 0x08, // Left alignment
            2 => flags |= 0x10, // Right alignment
            _ => {}, // Invalid alignment, default to center
        }
        
        self.entity
            .send_meta_data(&[Metadata::new(4, MetaDataType::Byte, flags)])
            .await;
    }

    /// Sets the alignment of the text.
    pub async fn set_alignment(&self, alignment: u8) {
        if alignment > 2 { return; } // Only 0, 1, 2 are valid
        self.alignment.store(alignment);
        self.update_flags().await;
    }

    /// Sets whether the text should have a shadow.
    pub async fn set_shadow(&self, has_shadow: bool) {
        self.has_shadow.store(has_shadow);
        self.update_flags().await;
    }

    /// Sets whether the display is see-through.
    pub async fn set_see_through(&self, see_through: bool) {
        self.see_through.store(see_through);
        self.update_flags().await;
    }

    /// Sets whether the display has the default background.
    pub async fn set_default_background(&self, has_default_background: bool) {
        self.has_default_background.store(has_default_background);
        self.update_flags().await;
    }

    /// Sends all metadata for this text display entity to clients.
    pub async fn send_metadata(&self) {
        // Send text value
        {
            let text = self.text.lock().await.clone();
            self.entity
                .send_meta_data(&[Metadata::new(23, MetaDataType::TextComponent, text)])
                .await;
        }
        
        // Send line width
        self.entity
            .send_meta_data(&[Metadata::new(24, MetaDataType::Integer, VarInt(self.width.load() as i32))])
            .await;
        
        // // Send background color
        self.entity
            .send_meta_data(&[Metadata::new(25, MetaDataType::Integer, self.background_color.load())])
            .await;
        
        // // Send text opacity
        self.entity
            .send_meta_data(&[Metadata::new(26, MetaDataType::Byte, self.text_opacity.load() as i8)])
            .await;
        
        // // Send billboard mode (part of the base Display class)
        self.entity
            .send_meta_data(&[Metadata::new(15, MetaDataType::Byte, self.billboard_mode.load() as i8)])
            .await;
        
        // // Send display flags
        // self.update_flags().await;
    }

    /// Sets the Y offset of the display.
    pub async fn set_y_offset(&self, offset: f32) {
        // Translation offset Y is at index 12 for the Display base entity
        self.entity
            .send_meta_data(&[Metadata::new(12, MetaDataType::Float, offset)])
            .await;
    }
}

#[async_trait]
impl EntityBase for TextDisplayEntity {
    async fn tick(&self, server: &Server) {
        self.entity.tick(server).await;
    }

    fn get_entity(&self) -> &Entity {
        &self.entity
    }

    fn get_living_entity(&self) -> Option<&crate::entity::living::LivingEntity> {
        None
    }
}

#[async_trait]
impl NBTStorage for TextDisplayEntity {
    async fn write_nbt(&self, nbt: &mut NbtCompound) {
        self.entity.write_nbt(nbt).await;
        
        // TODO: Store text display specific properties
        // This would include text content, background color, etc.
    }

    async fn read_nbt(&mut self, nbt: &mut NbtCompound) {
        self.entity.read_nbt(nbt).await;
        
        // TODO: Read text display specific properties
    }
}