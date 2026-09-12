//! Bot-only perception and firing controls. Weapon accuracy and damage stay shared.
use crate::game::config::BotDifficulty;
use bevy::prelude::*;
use rand::Rng;

pub(super) struct DifficultyProfile {
    pub reaction_seconds: f32,
    pub yaw_speed: f32,
    /// None preserves Hard's immediate vertical tracking.
    pub pitch_speed: Option<f32>,
    pub aim_error: f32,
    pub drift_seconds: f32,
    pub burst: Option<(f32, f32)>, // firing duration, pause duration
}

impl BotDifficulty {
    pub(super) fn profile(self) -> DifficultyProfile {
        match self {
            Self::Hard => DifficultyProfile {
                reaction_seconds: 0.45,
                yaw_speed: 2.4,
                pitch_speed: Some(2.0),
                aim_error: 0.8_f32.to_radians(),
                drift_seconds: 0.7,
                burst: None,
            },
            Self::Normal => DifficultyProfile {
                reaction_seconds: 0.85,
                yaw_speed: 1.4,
                pitch_speed: Some(0.95),
                aim_error: 3.2_f32.to_radians(),
                drift_seconds: 0.75,
                burst: Some((0.4, 0.55)),
            },
            Self::Easy => DifficultyProfile {
                reaction_seconds: 1.25,
                yaw_speed: 0.75,
                pitch_speed: Some(0.45),
                aim_error: 7.0_f32.to_radians(),
                drift_seconds: 1.05,
                burst: Some((0.2, 1.1)),
            },
        }
    }
}

#[derive(Default)]
pub(super) struct AimState {
    drift_from: Vec2,
    drift_to: Vec2,
    drift_elapsed: f32,
    burst_elapsed: f32,
    pause_remaining: f32,
}

impl AimState {
    pub fn reset(&mut self, profile: &DifficultyProfile) {
        *self = Self {
            drift_from: sample_error(profile.aim_error),
            drift_to: sample_error(profile.aim_error),
            ..default()
        };
    }

    pub fn offset(&mut self, dt: f32, profile: &DifficultyProfile) -> Vec2 {
        if profile.aim_error == 0.0 {
            return Vec2::ZERO;
        }
        self.drift_elapsed += dt;
        while self.drift_elapsed >= profile.drift_seconds {
            self.drift_elapsed -= profile.drift_seconds;
            self.drift_from = self.drift_to;
            self.drift_to = sample_error(profile.aim_error);
        }
        let t = self.drift_elapsed / profile.drift_seconds;
        // Smoothstep gives each drift segment zero velocity at its endpoints.
        self.drift_from.lerp(self.drift_to, t * t * (3.0 - 2.0 * t))
    }

    pub fn fire(&mut self, ready: bool, dt: f32, profile: &DifficultyProfile) -> bool {
        let Some((duration, pause)) = profile.burst else {
            return ready;
        };
        if self.pause_remaining > 0.0 {
            self.pause_remaining = (self.pause_remaining - dt).max(0.0);
            return false;
        }
        if !ready {
            return false;
        }
        self.burst_elapsed += dt;
        if self.burst_elapsed >= duration {
            self.burst_elapsed = 0.0;
            self.pause_remaining = pause;
        }
        true
    }
}

fn sample_error(radians: f32) -> Vec2 {
    if radians == 0.0 {
        return Vec2::ZERO;
    }
    let mut rng = rand::thread_rng();
    Vec2::new(
        rng.gen_range(-radians..=radians),
        rng.gen_range(-radians..=radians),
    )
}
