use crate::game::{
    config::{GameConfig, GameMode, MapId},
    GameState,
};
use bevy::prelude::*;
pub mod debug_widgets;
mod diagnostics;
pub mod friends_drawer;
mod glass;
pub mod home_scene;
pub mod home_tab;
pub mod inventory_tab;
mod loadout_scene;
pub mod loadout_tab;
pub mod nav_bar;
pub mod play_tab;
pub mod scene_definition;
pub mod settings_tab;
mod start_button;
pub mod style;
pub mod text_field;
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum MenuTab {
    #[default]
    Home,
    Inventory,
    Play,
    LoadOut,
    Settings,
}
pub use crate::game::config::{PlayerLoadout, PlayerSettings, WeaponId};
/// Public local-play option. GameConfig alone owns the selection.
pub struct LocalMatchOption;
impl LocalMatchOption {
    pub const LABEL: &'static str = "Deathmatch";
    pub fn select(config: &mut GameConfig) {
        config.mode = GameMode::TeamDeathmatch;
        config.map = MapId::Dust2;
    }
    pub fn selected(config: &GameConfig) -> bool {
        config.mode == GameMode::TeamDeathmatch && config.map == MapId::Dust2
    }
}
#[derive(Component)]
pub(super) struct MenuPage(pub MenuTab);
pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        diagnostics::install(app);
        app.init_state::<MenuTab>()
            .add_systems(OnEnter(GameState::MainMenu), reset_menu)
            .add_systems(Update, (show_pages, style::apply_fonts))
            .add_plugins((
                glass::MenuGlassPlugin,
                nav_bar::NavBarPlugin,
                home_scene::HomeScenePlugin,
                home_tab::HomeTabPlugin,
                play_tab::PlayTabPlugin,
                inventory_tab::InventoryTabPlugin,
                loadout_tab::LoadoutTabPlugin,
                text_field::TextFieldPlugin,
                friends_drawer::FriendsDrawerPlugin,
                settings_tab::SettingsTabPlugin,
            ));
    }
}
fn reset_menu(mut config: ResMut<GameConfig>, mut next: ResMut<NextState<MenuTab>>) {
    LocalMatchOption::select(&mut config);
    next.set(MenuTab::Home);
}
fn show_pages(
    state: Res<State<GameState>>,
    tab: Res<State<MenuTab>>,
    mut pages: Query<(&MenuPage, &mut Node)>,
) {
    for (page, mut node) in &mut pages {
        node.display = if *state.get() == GameState::MainMenu && page.0 == *tab.get() {
            Display::Flex
        } else {
            Display::None
        };
    }
}
