use crate::game::{config::WeaponId, matchplay::Team};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct HudArt {
    pub icons: HashMap<WeaponId, Handle<Image>>,
    pub headshot: Handle<Image>,
    pub portraits: [Handle<Image>; 2],
    pub font: Handle<Font>,
}
impl HudArt {
    pub fn weapon(&self, id: WeaponId) -> Handle<Image> {
        self.icons
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.icons[&WeaponId::AK47].clone())
    }
}
pub fn prepare(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(HudArt {
        icons: WeaponId::ALL
            .into_iter()
            .map(|id| (id, server.load(id.inventory_path())))
            .collect(),
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
