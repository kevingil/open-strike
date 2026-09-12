use crate::game::config::WeaponId;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct GameAssets {
    pub skins: [Handle<Gltf>; 2],
    pub gun: Handle<Gltf>,
    pub arms: [Handle<Gltf>; 2],
    pub knife_view: [Handle<Gltf>; 2],
    pub knife_world: Handle<Gltf>,
    pub reference_knife_view: [Handle<Gltf>; 2],
    pub reference_knife_world: Handle<Gltf>,
    pub knife_poses: [Handle<Gltf>; 2],
    pub worlds: HashMap<WeaponId, Handle<Gltf>>,
}
/// The dedicated server keeps meshes for colliders and animations for validation but
/// never uploads anything to a GPU, so materials and textures are skipped entirely.
pub fn load_gltf(
    server: &AssetServer,
    role: crate::game::net::NetRole,
    path: &str,
) -> Handle<Gltf> {
    if role.is_server() {
        server.load_with_settings(
            path.to_string(),
            |settings: &mut bevy::gltf::GltfLoaderSettings| {
                settings.load_meshes = bevy::asset::RenderAssetUsages::MAIN_WORLD;
                settings.load_materials = bevy::asset::RenderAssetUsages::empty();
            },
        )
    } else {
        server.load(path.to_string())
    }
}
pub fn load_assets(
    mut commands: Commands,
    server: Res<AssetServer>,
    role: Option<Res<crate::game::net::NetRole>>,
) {
    let role = role.map(|r| *r).unwrap_or_default();
    let load = |path: &str| load_gltf(&server, role, path);
    let worlds = WeaponId::ALL
        .into_iter()
        .map(|weapon| (weapon, load(weapon.world_path())))
        .collect();
    commands.insert_resource(GameAssets {
        skins: [
            load("generated/attacker.glb"),
            load("generated/defender.glb"),
        ],
        gun: load("generated/ak_world.glb"),
        knife_view: [
            load("generated/knife_view_soldier.glb"),
            load("generated/knife_view_police.glb"),
        ],
        knife_world: load("generated/knife_world.glb"),
        reference_knife_view: [
            load("generated/reference_knife_view_soldier.glb"),
            load("generated/reference_knife_view_police.glb"),
        ],
        reference_knife_world: load("generated/reference_knife_world.glb"),
        knife_poses: [
            load("generated/knife_pose_attacker.glb"),
            load("generated/knife_pose_defender.glb"),
        ],
        arms: [
            load(if std::env::var_os("CSRS_INVALID_ASSET").is_some() {
                "generated/intentionally_missing.glb"
            } else {
                "generated/ak_view_soldier.glb"
            }),
            load("generated/ak_view_police.glb"),
        ],
        worlds,
    });
}
impl GameAssets {
    pub fn world(&self, id: WeaponId) -> &Handle<Gltf> {
        self.worlds.get(&id).unwrap_or(&self.gun)
    }

    pub fn ready(&self, server: &AssetServer) -> bool {
        self.skins
            .iter()
            .chain(self.arms.iter())
            .chain(self.knife_view.iter())
            .chain(self.reference_knife_view.iter())
            .chain([&self.gun, &self.knife_world, &self.reference_knife_world])
            .chain(self.knife_poses.iter())
            .chain(self.worlds.values())
            .all(|h| server.is_loaded_with_dependencies(h.id()))
    }
    pub fn failure(&self, server: &AssetServer) -> Option<String> {
        self.skins
            .iter()
            .chain(self.arms.iter())
            .chain(self.knife_view.iter())
            .chain(self.reference_knife_view.iter())
            .chain([&self.gun, &self.knife_world, &self.reference_knife_world])
            .chain(self.knife_poses.iter())
            .chain(self.worlds.values())
            .find_map(|h| {
                if let bevy::asset::LoadState::Failed(e) = server.load_state(h.id()) {
                    Some(e.to_string())
                } else {
                    None
                }
            })
    }
}
