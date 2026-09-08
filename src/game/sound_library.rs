//! Bundled original cues with a local catalog and optional explicit pack selection.
use bevy::{asset::LoadState, prelude::*};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct SoundClip {
    pub path: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub frames: u64,
    pub sha256: String,
}

#[derive(Resource)]
pub struct SoundLibrary {
    pub catalog: BTreeMap<String, SoundClip>,
    loaded: BTreeMap<String, Handle<AudioSource>>,
}

impl Default for SoundLibrary {
    fn default() -> Self {
        let mut catalog: BTreeMap<String, SoundClip> =
            bevy::asset::ron::from_str(include_str!("../../assets/audio/generated/catalog.ron"))
                .expect("The bundled sound catalog must be valid");
        let override_path = std::env::var_os("OPEN_STRIKE_AUDIO_PACK")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                let local = std::path::PathBuf::from("assets/audio/local/catalog.ron");
                local.is_file().then_some(local)
            });
        if let Some(path) = override_path {
            match std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    bevy::asset::ron::from_str::<BTreeMap<String, SoundClip>>(&text)
                        .map_err(|error| error.to_string())
                }) {
                Ok(overrides) => {
                    let mut selected = 0;
                    for (id, clip) in overrides {
                        let path = std::path::Path::new(&clip.path);
                        // Paths use Bevy's asset root. Invalid or absent local
                        // files must not prevent the default game from loading.
                        let valid = path
                            .components()
                            .all(|part| matches!(part, std::path::Component::Normal(_)))
                            && path.extension().is_some_and(|extension| extension == "wav")
                            && std::path::Path::new("assets").join(path).is_file();
                        // This optional walking loop replaces the discrete
                        // fallback steps only when selected by a local pack.
                        if valid && (catalog.contains_key(&id) || id == "misc/step_test_loop") {
                            catalog.insert(id, clip);
                            selected += 1;
                        }
                    }
                    info!(
                        "Using {} local sound overrides; other cues use generated defaults",
                        selected
                    );
                }
                Err(error) => warn!(
                    "Could not read local sound pack: {}; using generated defaults",
                    error
                ),
            }
        }
        Self {
            catalog,
            loaded: default(),
        }
    }
}

impl SoundLibrary {
    /// Exact catalog ID, e.g. `weapons/ak47-1` or `weapons/ak47_clipin`.
    pub fn load(&mut self, id: &str, server: &AssetServer) -> Option<Handle<AudioSource>> {
        let clip = self.catalog.get(id)?;
        Some(
            self.loaded
                .entry(id.to_owned())
                .or_insert_with(|| server.load(clip.path.clone()))
                .clone(),
        )
    }

    pub fn ready(&self, server: &AssetServer) -> bool {
        self.loaded
            .values()
            .all(|handle| server.is_loaded_with_dependencies(handle.id()))
    }

    pub fn failure(&self, server: &AssetServer) -> Option<String> {
        self.loaded.iter().find_map(|(id, handle)| {
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                Some(format!("Sound {id}: {error}"))
            } else {
                None
            }
        })
    }

    pub fn retry_failed(&self, server: &AssetServer) {
        for (id, handle) in &self.loaded {
            if matches!(server.load_state(handle.id()), LoadState::Failed(_)) {
                server.reload(self.catalog[id].path.clone());
            }
        }
    }
}
