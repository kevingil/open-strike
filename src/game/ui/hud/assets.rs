use crate::game::{config::WeaponId, matchplay::Team};
use bevy::prelude::*;

#[derive(Resource)]
pub struct HudArt {
    pub rifle: Handle<Image>,
    pub knife: Handle<Image>,
    pub headshot: Handle<Image>,
    pub portraits: [Handle<Image>; 2],
    pub font: Handle<Font>,
}
impl HudArt {
    pub fn weapon(&self, id: WeaponId) -> Handle<Image> {
        match id {
            WeaponId::AK47 => self.rifle.clone(),
            WeaponId::DefaultKnife => self.knife.clone(),
        }
    }
}
pub fn prepare(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(HudArt {
        rifle: server.load("generated/ui/ak47.png"),
        knife: server.load("generated/ui/knife.png"),
        headshot: server.load("generated/ui/headshot.png"),
        portraits: [
            server.load("generated/ui/attacker_portrait.png"),
            server.load("generated/ui/defender_portrait.png"),
        ],
        font: server.load("fonts/RobotoCondensed.ttf"),
    });
}
pub fn team_color(team: Team) -> Color {
    match team {
        Team::Attacker => Color::srgb(0.92, 0.78, 0.42),
        Team::Defender => Color::srgb(0.60, 0.79, 0.96),
    }
}
