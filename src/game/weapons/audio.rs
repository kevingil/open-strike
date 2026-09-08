//! Weapon and player cues consume accepted gameplay actions.
use super::{ShotFired, WeaponState, AK47};
use crate::game::{
    config::{PlayerSettings, WeaponId},
    matchplay::Combatant,
    player::player::{LocalPlayer, PlayerEntity},
    sound_library::SoundLibrary,
    GameState,
};
use bevy::{audio::Volume, prelude::*};

#[derive(Resource)]
struct Cues {
    shot: Handle<AudioSource>,
    draw: Handle<AudioSource>,
    knife_draw: Handle<AudioSource>,
    knife_slash: Handle<AudioSource>,
    reload: [(f32, Handle<AudioSource>); 3],
    hit: [Handle<AudioSource>; 5],
    step: [Handle<AudioSource>; 4],
    walk_loop: Option<Handle<AudioSource>>,
}
#[derive(Component)]
pub struct AudioState {
    reload_remaining: f32,
    reload_cancellations: u64,
    equips: u64,
    slashes: u64,
    alive: bool,
    health: f32,
    last_position: Vec3,
    distance: f32,
    step_index: usize,
    walk_voice: Option<Entity>,
    walk_tick: Option<std::time::Duration>,
    hit_index: usize,
}
impl Default for AudioState {
    fn default() -> Self {
        Self {
            reload_remaining: 0.0,
            reload_cancellations: 0,
            equips: 0,
            slashes: 0,
            alive: false,
            health: 100.0,
            last_position: Vec3::ZERO,
            distance: 0.0,
            step_index: 0,
            walk_voice: None,
            walk_tick: None,
            hit_index: 0,
        }
    }
}
pub struct WeaponAudioPlugin;
impl Plugin for WeaponAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundLibrary>()
            .add_systems(Startup, prepare)
            .add_systems(Update, (sounds.run_if(in_state(GameState::Playing)), pause));
    }
}
fn prepare(mut commands: Commands, mut library: ResMut<SoundLibrary>, server: Res<AssetServer>) {
    info!(
        "Knife equip sound: {}",
        library.catalog["weapons/knife_draw"].path
    );
    let walk_loop = library.load("misc/step_test_loop", &server);
    if walk_loop.is_some() {
        info!(
            "Walking loop: {}",
            library.catalog["misc/step_test_loop"].path
        );
    }
    let mut recording = |id| {
        library
            .load(id, &server)
            .expect("Required gameplay sound is cataloged")
    };
    commands.insert_resource(Cues {
        shot: recording("weapons/ak47-1"),
        draw: recording("weapons/ak47_draw"),
        knife_draw: recording("weapons/knife_draw"),
        knife_slash: recording("weapons/knife_slash"),
        // Match magazine extraction/insertion and bolt pull in the authored
        // 0..99-frame reload_rifle animation, scaled to the simulation duration.
        reload: [
            (18.0 / 99.0, recording("weapons/ak47_clipout")),
            (65.0 / 99.0, recording("weapons/ak47_clipin")),
            (73.0 / 99.0, recording("weapons/ak47_boltpull")),
        ],
        hit: [
            recording("physics/flesh_impact_bullet1"),
            recording("physics/flesh_impact_bullet2"),
            recording("physics/flesh_impact_bullet3"),
            recording("physics/flesh_impact_bullet4"),
            recording("physics/flesh_impact_bullet5"),
        ],
        // Generic default family on every surface until surface audio is implemented.
        step: [
            recording("physics/drywall_footstep1"),
            recording("physics/drywall_footstep2"),
            recording("physics/drywall_footstep3"),
            recording("physics/drywall_footstep4"),
        ],
        walk_loop,
    });
}
#[derive(Component)]
struct ReloadSound(Entity);
fn play(
    commands: &mut Commands,
    clip: Handle<AudioSource>,
    position: Vec3,
    volume: f32,
    spatial: bool,
) -> Entity {
    commands
        .spawn((
            PlayerEntity,
            AudioPlayer(clip),
            PlaybackSettings::DESPAWN
                .with_spatial(spatial)
                .with_volume(Volume::Linear(volume)),
            Transform::from_translation(position),
        ))
        .id()
}
fn sounds(
    mut commands: Commands,
    fixed_time: Res<Time<Fixed>>,
    mut shots: EventReader<ShotFired>,
    reload_sounds: Query<(Entity, &ReloadSound)>,
    cues: Res<Cues>,
    settings: Res<PlayerSettings>,
    listener: Query<Entity, With<LocalPlayer>>,
    mut actors: Query<(
        Entity,
        &Transform,
        &Combatant,
        &WeaponState,
        &bevy_fps_controller::controller::FpsController,
        &mut AudioState,
    )>,
) {
    let local = listener.single().ok();
    for shot in shots.read() {
        play(
            &mut commands,
            cues.shot.clone(),
            shot.origin,
            settings.master_volume * 0.45,
            Some(shot.actor) != local,
        );
    }
    for (entity, transform, actor, weapon, controller, mut state) in &mut actors {
        let reloading = weapon.reload_remaining > 0.0;
        let canceled = state.reload_cancellations != weapon.reload_cancellations || !actor.alive();
        if canceled {
            for (sound, owner) in &reload_sounds {
                if owner.0 == entity {
                    commands.entity(sound).despawn();
                }
            }
        }
        if actor.alive() && (!state.alive || state.equips != weapon.equips) {
            play(
                &mut commands,
                if weapon.active == WeaponId::DefaultKnife {
                    cues.knife_draw.clone()
                } else {
                    cues.draw.clone()
                },
                transform.translation,
                settings.master_volume * 0.4,
                Some(entity) != local,
            );
        }
        if actor.alive() && state.alive && weapon.slashes > state.slashes {
            play(
                &mut commands,
                cues.knife_slash.clone(),
                transform.translation,
                settings.master_volume * 0.4,
                Some(entity) != local,
            );
        }
        if actor.alive()
            && !canceled
            && weapon.active == WeaponId::AK47
            && (reloading || state.reload_remaining > 0.0)
        {
            let previous = if state.reload_remaining > 0.0 {
                AK47.reload_seconds - state.reload_remaining
            } else {
                -1.0
            };
            let elapsed = AK47.reload_seconds - weapon.reload_remaining;
            for (fraction, clip) in &cues.reload {
                let at = fraction * AK47.reload_seconds;
                if previous < at && elapsed >= at {
                    let sound = play(
                        &mut commands,
                        clip.clone(),
                        transform.translation,
                        settings.master_volume * 0.4,
                        Some(entity) != local,
                    );
                    commands.entity(sound).insert(ReloadSound(entity));
                }
            }
        }
        if actor.health < state.health && Some(entity) == local {
            play(
                &mut commands,
                cues.hit[state.hit_index].clone(),
                transform.translation,
                settings.master_volume * 0.6,
                false,
            );
            state.hit_index = (state.hit_index + 1) % cues.hit.len();
        }
        let traveled = transform.translation.distance(state.last_position);
        let grounded = traveled < 1.0 && actor.alive() && controller.ground_tick > 0;
        if let Some(clip) = &cues.walk_loop {
            // Horizontal displacement measures actual movement, including
            // collision blocking. Ignore sub-millimeter physics jitter.
            let walking = grounded
                && transform
                    .translation
                    .xz()
                    .distance_squared(state.last_position.xz())
                    > 0.000001;
            // Render frames between physics ticks repeat the same position;
            // they must not stop/restart a continuously walking actor's loop.
            let new_tick = state.walk_tick != Some(fixed_time.elapsed());
            state.walk_tick = Some(fixed_time.elapsed());
            if walking && new_tick && state.walk_voice.is_none() {
                let voice = commands
                    .spawn((
                        AudioPlayer(clip.clone()),
                        PlaybackSettings::LOOP
                            .with_spatial(Some(entity) != local)
                            .with_volume(Volume::Linear(settings.master_volume * 0.35)),
                        Transform::default(),
                    ))
                    .id();
                // Actor ownership follows its position and cleans up on exit.
                commands.entity(entity).add_child(voice);
                state.walk_voice = Some(voice);
            } else if (!walking && new_tick) || !actor.alive() {
                if let Some(voice) = state.walk_voice.take() {
                    commands.entity(voice).despawn();
                }
            }
            state.distance = 0.0;
        } else if grounded {
            state.distance += traveled;
        } else {
            state.distance = 0.0;
        }
        if cues.walk_loop.is_none() && state.distance > 1.6 {
            state.distance = 0.0;
            play(
                &mut commands,
                cues.step[state.step_index].clone(),
                transform.translation,
                settings.master_volume * 0.35,
                Some(entity) != local,
            );
            state.step_index = (state.step_index + 1) % cues.step.len();
        }
        state.last_position = transform.translation;
        state.reload_remaining = weapon.reload_remaining;
        state.reload_cancellations = weapon.reload_cancellations;
        state.equips = weapon.equips;
        state.slashes = weapon.slashes;
        state.alive = actor.alive();
        state.health = actor.health;
    }
}
fn pause(
    state: Res<State<GameState>>,
    sinks: Query<&AudioSink>,
    spatial: Query<&SpatialAudioSink>,
) {
    let paused = matches!(state.get(), GameState::Paused | GameState::Finished);
    for sink in &sinks {
        if paused {
            sink.pause();
        } else {
            sink.play();
        }
    }
    for sink in &spatial {
        if paused {
            sink.pause();
        } else {
            sink.play();
        }
    }
}
