use dashmap::DashMap;
use pumpkin_config::advanced_config;
use pumpkin_data::{
    particle::Particle,
    sound::{Sound, SoundCategory},
};
use pumpkin_protocol::{codec::var_int::VarInt, java::client::play::CEntityVelocity};
use pumpkin_util::math::square;
use pumpkin_util::math::vector3::Vector3;
use rand::Rng;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, LazyLock};
use uuid::Uuid;

use crate::{
    entity::{Entity, player::Player},
    world::World,
};

#[derive(Debug, Clone, Copy)]
pub enum AttackType {
    Knockback,
    Critical,
    Sweeping,
    Strong,
    Weak,
}

impl AttackType {
    pub async fn new(player: &Player, attack_cooldown_progress: f32) -> Self {
        let entity = &player.living_entity.entity;

        let sprinting = entity.sprinting.load(Relaxed);
        let on_ground = entity.on_ground.load(Relaxed);
        let fall_distance = player.living_entity.fall_distance.load();
        let sword = player.inventory().held_item().lock().await.is_sword();

        let is_strong = attack_cooldown_progress > 0.9;
        if sprinting && is_strong {
            return Self::Knockback;
        }

        // TODO: even more checks
        if is_strong && !on_ground && fall_distance > 0.0 {
            // !sprinting omitted
            return Self::Critical;
        }

        // TODO: movement speed check
        if sword && is_strong {
            // !is_crit, !is_knockback_hit, on_ground omitted
            return Self::Sweeping;
        }

        if is_strong { Self::Strong } else { Self::Weak }
    }
}

pub async fn handle_knockback(attacker: &Entity, world: &World, victim: &Entity, strength: f64) {
    let yaw = attacker.yaw.load();

    let saved_velo = victim.velocity.load();
    victim.knockback(
        strength * 0.5,
        f64::from((yaw.to_radians()).sin()),
        f64::from(-(yaw.to_radians()).cos()),
    );

    let entity_id = VarInt(victim.entity_id);
    let victim_velocity = victim.velocity.load();

    let packet = CEntityVelocity::new(entity_id, victim_velocity);
    let velocity = attacker.velocity.load();
    attacker.velocity.store(velocity.multiply(0.6, 1.0, 0.6));

    victim.velocity.store(saved_velo);
    world.broadcast_packet_all(&packet).await;
}

pub async fn spawn_sweep_particle(attacker_entity: &Entity, world: &World, pos: &Vector3<f64>) {
    let yaw = attacker_entity.yaw.load();
    let d = -f64::from((yaw.to_radians()).sin());
    let e = f64::from((yaw.to_radians()).cos());

    let scale = 0.5;
    let body_y = pos.y + f64::from(attacker_entity.height()) * scale;

    world
        .spawn_particle(
            Vector3::new(pos.x + d, body_y, pos.z + e),
            Vector3::new(0.0, 0.0, 0.0),
            0.0,
            0,
            Particle::SweepAttack,
        )
        .await;
}

pub async fn player_attack_sound(pos: &Vector3<f64>, world: &World, attack_type: AttackType) {
    match attack_type {
        AttackType::Knockback => {
            world
                .play_sound(
                    Sound::EntityPlayerAttackKnockback,
                    SoundCategory::Players,
                    pos,
                )
                .await;
        }
        AttackType::Critical => {
            world
                .play_sound(Sound::EntityPlayerAttackCrit, SoundCategory::Players, pos)
                .await;
        }
        AttackType::Sweeping => {
            world
                .play_sound(Sound::EntityPlayerAttackSweep, SoundCategory::Players, pos)
                .await;
        }
        AttackType::Strong => {
            world
                .play_sound(Sound::EntityPlayerAttackStrong, SoundCategory::Players, pos)
                .await;
        }
        AttackType::Weak => {
            world
                .play_sound(Sound::EntityPlayerAttackWeak, SoundCategory::Players, pos)
                .await;
        }
    }
}

/// This will only be used by plugins who override per-player `CombatProfile`s
pub static COMBAT_PROFILES: LazyLock<DashMap<Uuid, Arc<dyn CombatProfile + Send + Sync>>> =
    LazyLock::new(DashMap::new);

/// This is a global in-memory cache that is initialised once on pumpkin start. It holds the current knockback configuration.
pub static GLOBAL_COMBAT_PROFILE: LazyLock<Arc<dyn CombatProfile + Send + Sync>> = LazyLock::new(
    || {
        let config = &advanced_config().pvp;

        match config.combat_type.to_lowercase().as_str() {
            "classic" => Arc::new(ClassicProfile {
                friction: config.friction,
                horizontal_kb: config.horizontal_kb,
                vertical_kb: config.vertical_kb,
                vertical_limit: config.vertical_limit,
                extra_horizontal_kb: config.extra_horizontal_kb,
                extra_vertical_kb: config.extra_vertical_kb,
            }),
            "modern" => Arc::new(ModernProfile {
                friction: config.friction,
                horizontal_kb: config.horizontal_kb,
                vertical_kb: config.vertical_kb,
                vertical_limit: config.vertical_limit,
                extra_horizontal_kb: config.extra_horizontal_kb,
                extra_vertical_kb: config.extra_vertical_kb,
            }),
            unknown => {
                log::warn!(
                    "Combat Profile '{unknown}' does not exist! Falling back to Modern Combat Profile instead."
                );
                Arc::new(ModernProfile {
                    friction: config.friction,
                    horizontal_kb: config.horizontal_kb,
                    vertical_kb: config.vertical_kb,
                    vertical_limit: config.vertical_limit,
                    extra_horizontal_kb: config.extra_horizontal_kb,
                    extra_vertical_kb: config.extra_vertical_kb,
                })
            }
        }
    },
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatType {
    Legacy,
    Classic,
    Modern,
}

#[allow(dead_code)]
pub trait CombatProfile: Send + Sync {
    fn apply_attack_knockback(&self, attacker: Arc<Player>, target: Arc<Entity>, strength: f64);
    fn receive_knockback(&self, entity: Arc<Entity>, knockback_x: f64, knockback_z: f64);
    fn combat_type(&self) -> CombatType;
    fn friction(&self) -> f64;
    fn horizontal_kb(&self) -> f64;
    fn vertical_kb(&self) -> f64;
    fn vertical_limit(&self) -> f64;
    fn extra_horizontal_kb(&self) -> f64;
    fn extra_vertical_kb(&self) -> f64;
}

pub struct ClassicProfile {
    pub friction: f64,
    pub horizontal_kb: f64,
    pub vertical_kb: f64,
    pub vertical_limit: f64,
    pub extra_horizontal_kb: f64,
    pub extra_vertical_kb: f64,
}

impl CombatProfile for ClassicProfile {
    // TODO: send update packet, but maybe do that when this fn is called
    /// Getting called from an attacker, when attacking an entity
    fn apply_attack_knockback(&self, attacker: Arc<Player>, target: Arc<Entity>, strength: f64) {
        // TODO: Velocity changed flag? + critical hit flag?
        let yaw: f64 = f64::from(target.yaw.load());
        let yaw_rad = yaw.to_radians();

        // The `extra_horizontal_kb` is 0.5 and `extra_vertical_kb` 0.1 by default in java mc 1.8
        let knockback_x = -yaw_rad.sin() * strength * self.extra_horizontal_kb;
        let knockback_z = yaw_rad.cos() * strength * self.extra_horizontal_kb;
        let knockback_y = self.extra_vertical_kb;

        let mut velocity = target.velocity.load();

        velocity.x += knockback_x;
        velocity.y += knockback_y;
        velocity.z += knockback_z;

        let mut attacker_velocity = attacker.living_entity.entity.velocity.load();
        attacker_velocity.x *= 0.6;
        attacker_velocity.z *= 0.6;

        // TODO: ADD not STORE the velocity ? Lune I'm confused
        target.velocity.store(velocity);

        attacker
            .living_entity
            .entity
            .sprinting
            .store(false, Relaxed);
    }

    /// Getting called on a target, when being attacked
    // the `float p_70653_2_` from java is dead code, so I removed it
    fn receive_knockback(&self, target: Arc<Entity>, knockback_x: f64, knockback_z: f64) {
        let mut rng = rand::rng();
        // TODO: Use the actual knockback_resistance when this field get's added (issue already created)
        let knockback_resistance = 0.5;

        if rng.random::<f64>() >= knockback_resistance {
            // TODO: Use this
            let _ = !target.on_ground.load(Relaxed);

            let magnitude = (square(knockback_x) + square(knockback_z)).sqrt();
            let mut velocity = target.velocity.load();

            // `friction` is 2.0 by default in java mc 1.8
            velocity.x /= self.friction;
            velocity.y /= self.friction;
            velocity.z /= self.friction;

            // `horizontal_kb` and `vertical_kb` are 0.4 by default in java mc 1.8
            velocity.x -= knockback_x / magnitude * self.horizontal_kb;
            velocity.y += self.vertical_kb;
            velocity.z -= knockback_z / magnitude * self.horizontal_kb;

            // `vertical_limit` is 0.4000000059604645 by default in java mc 1.8
            if velocity.y > self.vertical_limit {
                velocity.y = self.vertical_limit;
            }
        }
    }

    fn combat_type(&self) -> CombatType {
        CombatType::Classic
    }

    fn friction(&self) -> f64 {
        self.friction
    }

    fn horizontal_kb(&self) -> f64 {
        self.horizontal_kb
    }

    fn vertical_kb(&self) -> f64 {
        self.vertical_kb
    }

    fn vertical_limit(&self) -> f64 {
        self.vertical_limit
    }

    fn extra_horizontal_kb(&self) -> f64 {
        self.extra_horizontal_kb
    }

    fn extra_vertical_kb(&self) -> f64 {
        self.extra_vertical_kb
    }
}

// TODO: Hier sind andere Werte wichtig -> ändern
pub struct ModernProfile {
    pub friction: f64,
    pub horizontal_kb: f64,
    pub vertical_kb: f64,
    pub vertical_limit: f64,
    pub extra_horizontal_kb: f64,
    pub extra_vertical_kb: f64,
}

// These are taken from the existing implementation
impl CombatProfile for ModernProfile {
    fn apply_attack_knockback(&self, attacker: Arc<Player>, target: Arc<Entity>, strength: f64) {
        let yaw = attacker.living_entity.entity.yaw.load();

        let saved_velo = target.velocity.load();
        target.knockback(
            strength * 0.5,
            f64::from(yaw.to_radians().sin()),
            f64::from(-yaw.to_radians().cos()),
        );

        let entity_id = VarInt(target.entity_id);
        let target_velocity = target.velocity.load();

        let _packet = CEntityVelocity::new(entity_id, target_velocity);
        let velocity = attacker.living_entity.entity.velocity.load();
        attacker
            .living_entity
            .entity
            .velocity
            .store(velocity.multiply(0.6, 1.0, 0.6));

        target.velocity.store(saved_velo);
        //world.broadcast_packet_all(&packet).await;
    }

    fn receive_knockback(&self, entity: Arc<Entity>, knockback_x: f64, knockback_z: f64) {
        // This has some vanilla magic
        let mut x = knockback_x;
        let mut z = knockback_z;
        // TODO: actually get the value from somewhere, this is a dummy-parameter for now
        let strength = 0.5;
        while x.mul_add(x, z * z) < 1.0E-5 {
            x = (rand::random::<f64>() - rand::random::<f64>()) * 0.01;
            z = (rand::random::<f64>() - rand::random::<f64>()) * 0.01;
        }

        let var8 = Vector3::new(x, 0.0, z).normalize() * strength;
        let velocity = entity.velocity.load();
        entity.velocity.store(Vector3::new(
            velocity.x / 2.0 - var8.x,
            if entity.on_ground.load(Relaxed) {
                (velocity.y / 2.0 + strength).min(0.4)
            } else {
                velocity.y
            },
            velocity.z / 2.0 - var8.z,
        ));
    }

    fn combat_type(&self) -> CombatType {
        CombatType::Modern
    }

    fn friction(&self) -> f64 {
        self.friction
    }

    fn horizontal_kb(&self) -> f64 {
        self.horizontal_kb
    }

    fn vertical_kb(&self) -> f64 {
        self.vertical_kb
    }

    fn vertical_limit(&self) -> f64 {
        self.vertical_limit
    }

    fn extra_horizontal_kb(&self) -> f64 {
        self.extra_horizontal_kb
    }

    fn extra_vertical_kb(&self) -> f64 {
        self.extra_vertical_kb
    }
}
