use bevy::prelude::*;
use open_strike::game;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, game::game::GamePlugin))
        .run();
}
