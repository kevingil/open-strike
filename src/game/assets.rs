use bevy::prelude::*;
#[derive(Resource)]
pub struct GameAssets {
    pub skins: [Handle<Gltf>; 2],
    pub gun: Handle<Gltf>,
    pub arms: [Handle<Gltf>; 2],
    pub knife_view: [Handle<Gltf>; 2],
    pub knife_world: Handle<Gltf>,
    pub knife_poses: [Handle<Gltf>; 2],
}
pub fn load_assets(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(GameAssets {
        skins: [
            server.load("generated/attacker.glb"),
            server.load("generated/defender.glb"),
        ],
        gun: server.load("generated/ak_world.glb"),
        knife_view: [
            server.load("generated/knife_view_soldier.glb"),
            server.load("generated/knife_view_police.glb"),
        ],
        knife_world: server.load("generated/knife_world.glb"),
        knife_poses: [
            server.load("generated/knife_pose_attacker.glb"),
            server.load("generated/knife_pose_defender.glb"),
        ],
        arms: [
            server.load(if std::env::var_os("CSRS_INVALID_ASSET").is_some() {
                "generated/intentionally_missing.glb"
            } else {
                "generated/ak_view_soldier.glb"
            }),
            server.load("generated/ak_view_police.glb"),
        ],
    });
}
impl GameAssets {
    pub fn ready(&self, server: &AssetServer) -> bool {
        self.skins
            .iter()
            .chain(self.arms.iter())
            .chain(self.knife_view.iter())
            .chain([&self.gun, &self.knife_world])
            .chain(self.knife_poses.iter())
            .all(|h| server.is_loaded_with_dependencies(h.id()))
    }
    pub fn failure(&self, server: &AssetServer) -> Option<String> {
        self.skins
            .iter()
            .chain(self.arms.iter())
            .chain(self.knife_view.iter())
            .chain([&self.gun, &self.knife_world])
            .chain(self.knife_poses.iter())
            .find_map(|h| {
                if let bevy::asset::LoadState::Failed(e) = server.load_state(h.id()) {
                    Some(e.to_string())
                } else {
                    None
                }
            })
    }
}
