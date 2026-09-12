//! Local simulation authority. Future network input adapters feed ActorIntent; UI never awards damage/score.
use crate::game::{
    config::{GameConfig, GameMode, WeaponId},
    game::SimulationSet,
    level::level::LoadedGameplayMapConfig,
    net::NetRole,
    player::player::BODY_HEIGHT,
    weapons::WeaponState,
    GameState,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Team {
    Attacker,
    Defender,
}
impl Team {
    pub fn index(self) -> usize {
        if self == Self::Attacker {
            0
        } else {
            1
        }
    }
}

#[derive(Component)]
pub struct Combatant {
    pub team: Team,
    pub health: f32,
    pub armor: f32,
    pub name: String,
    pub kills: u32,
    pub deaths: u32,
    pub respawn_remaining: f32,
    pub protection_remaining: f32,
    pub slot: usize,
    /// Increments on every respawn so a networked client can detect the teleport.
    pub respawn_seq: u32,
    /// Stable identity on the wire; 0 for purely local actors.
    pub net_id: u32,
}
pub const BOT_NAMES: [&str; 12] = [
    "CEDAR", "MASON", "ROOK", "FLINT", "ASH", "BIRCH", "SLATE", "HOLLOW", "MARSH", "REED", "QUILL",
    "VALE",
];
impl Combatant {
    pub fn new(team: Team, slot: usize) -> Self {
        Self::named(
            team,
            slot,
            if slot == 0 {
                "YOU".to_string()
            } else {
                BOT_NAMES[(slot - 1) % BOT_NAMES.len()].to_string()
            },
        )
    }
    pub fn named(team: Team, slot: usize, name: String) -> Self {
        Self {
            team,
            health: 100.0,
            armor: 100.0,
            name,
            kills: 0,
            deaths: 0,
            respawn_remaining: 0.0,
            protection_remaining: 2.0,
            slot,
            respawn_seq: 0,
            net_id: 0,
        }
    }
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
}

#[derive(Component, Default)]
pub struct ActorIntent {
    pub fire: bool,
    pub reload: bool,
    pub selection: Option<crate::game::weapons::WeaponSelection>,
}

#[derive(Resource, Default)]
pub struct MatchSession {
    pub score: [u32; 2],
    pub elapsed: f32,
    pub result: String,
}

#[derive(Event, Clone)]
pub struct KillNotice {
    pub killer_entity: Entity,
    pub victim_entity: Entity,
    pub victim_team: Team,
    pub weapon: WeaponId,
    pub headshot: bool,
    pub killer: String,
    pub victim: String,
    pub team: Team,
}

pub struct MatchPlugin;
impl Plugin for MatchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchSession>()
            .add_event::<KillNotice>()
            .add_systems(OnEnter(GameState::Loading), reset_match)
            .add_systems(
                FixedUpdate,
                update_match
                    .in_set(SimulationSet::Match)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset_match(mut session: ResMut<MatchSession>) {
    *session = MatchSession::default();
}

pub fn update_match(
    time: Res<Time>,
    role: Res<NetRole>,
    config: Res<GameConfig>,
    map: Res<LoadedGameplayMapConfig>,
    mut commands: Commands,
    mut session: ResMut<MatchSession>,
    mut next: ResMut<NextState<GameState>>,
    context: ReadRapierContext,
    mut actors: Query<(
        Entity,
        &mut Combatant,
        &mut Transform,
        &mut Velocity,
        &mut FpsControllerInput,
        &mut FpsController,
        &mut WeaponState,
        &mut ActorIntent,
    )>,
) {
    let dt = time.delta_secs();
    session.elapsed += dt;
    let mut positions: Vec<(Entity, Vec3)> = actors
        .iter()
        .filter(|(_, a, ..)| a.alive())
        .map(|(e, _, t, ..)| (e, t.translation))
        .collect();
    for (
        entity,
        mut actor,
        mut transform,
        mut velocity,
        mut input,
        mut controller,
        mut weapon,
        mut intent,
    ) in &mut actors
    {
        actor.protection_remaining = (actor.protection_remaining - dt).max(0.0);
        if transform.translation.y < -30.0 && actor.alive() && !role.is_client() {
            commands.entity(entity).insert(ColliderDisabled);
            actor.health = 0.0;
            actor.deaths += 1;
            actor.respawn_remaining = config.match_settings.respawn_time.as_secs_f32();
        }
        if actor.alive() {
            continue;
        }
        velocity.linvel = Vec3::ZERO;
        input.movement = Vec3::ZERO;
        input.jump = false;
        intent.fire = false;
        intent.reload = false;
        intent.selection = None;
        // A client only mirrors the server's respawn timer and teleport.
        if role.is_client() {
            continue;
        }
        actor.respawn_remaining -= dt;
        if actor.respawn_remaining > 0.0 {
            continue;
        }
        let Some(map) = &map.config else { continue };
        let candidates: Vec<_> = map
            .spawn_points
            .iter()
            .filter(|s| !config.mode.teams() || s.team.is_none() || s.team == Some(actor.team))
            .collect();
        let Ok(physics) = context.single() else {
            continue;
        };
        for i in 0..candidates.len() {
            let spawn = candidates[(i + actor.slot + actor.deaths as usize) % candidates.len()];
            let feet = map
                .transform
                .to_transform()
                .transform_point(spawn.position.to_vec3());
            let center = feet + Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03);
            if positions
                .iter()
                .any(|(other, p)| *other != entity && p.distance(center) < 1.5)
            {
                continue;
            }
            let shape = Collider::cylinder(BODY_HEIGHT * 0.5 - 0.03, 0.29);
            let mut blocked = false;
            physics.intersections_with_shape(
                center,
                Quat::IDENTITY,
                &shape,
                QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(entity),
                |_| {
                    blocked = true;
                    false
                },
            );
            if blocked {
                continue;
            }
            commands.entity(entity).remove::<ColliderDisabled>();
            transform.translation = center;
            controller.height = BODY_HEIGHT;
            controller.ground_tick = 0;
            input.yaw = spawn.rotation.to_radians();
            input.pitch = 0.0;
            *weapon = WeaponState::armed(weapon.gun, weapon.melee_weapon);
            actor.health = 100.0;
            actor.armor = 100.0;
            actor.protection_remaining = 2.0;
            actor.respawn_seq = actor.respawn_seq.wrapping_add(1);
            positions.push((entity, center));
            info!("Respawn {:?} at {:?}", entity, feet);
            break;
        }
    }
    if role.is_client() || !config.mode.scored() {
        return;
    }
    let top = actors
        .iter()
        .map(|(_, a, ..)| (a.kills, a.name.clone()))
        .max_by_key(|(kills, _)| *kills);
    let limit_hit = config
        .match_settings
        .score_limit
        .is_some_and(|s| match config.mode {
            GameMode::Deathmatch => top.as_ref().is_some_and(|(kills, _)| *kills >= s),
            _ => session.score.iter().any(|v| *v >= s),
        });
    if config
        .match_settings
        .time_limit
        .is_some_and(|d| session.elapsed >= d.as_secs_f32())
        || limit_hit
    {
        session.result = if config.mode == GameMode::Deathmatch {
            top.map(|(_, name)| format!("{name} WINS"))
                .unwrap_or_else(|| "DRAW".into())
        } else {
            match session.score[0].cmp(&session.score[1]) {
                std::cmp::Ordering::Greater => "ATTACKERS WIN",
                std::cmp::Ordering::Less => "DEFENDERS WIN",
                _ => "DRAW",
            }
            .into()
        };
        next.set(GameState::Finished);
    }
}

/// Free-for-all treats every other actor as hostile.
pub fn hostile(mode: &GameMode, a: Team, b: Team) -> bool {
    !mode.teams() || a != b
}
