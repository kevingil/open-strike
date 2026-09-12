//! The one text input the menus need. Values live in `HubForms`; the widget only
//! renders and edits them, so screens can be rebuilt without losing what was typed.
use super::style::*;
use crate::game::hub::HubForms;
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FieldId {
    Identifier,
    Password,
    Username,
    Email,
    Confirm,
    Search,
}

impl FieldId {
    pub fn value(self, forms: &HubForms) -> &str {
        match self {
            FieldId::Identifier => &forms.identifier,
            FieldId::Password => &forms.password,
            FieldId::Username => &forms.username,
            FieldId::Email => &forms.email,
            FieldId::Confirm => &forms.confirm,
            FieldId::Search => &forms.search,
        }
    }
    pub fn value_mut(self, forms: &mut HubForms) -> &mut String {
        match self {
            FieldId::Identifier => &mut forms.identifier,
            FieldId::Password => &mut forms.password,
            FieldId::Username => &mut forms.username,
            FieldId::Email => &mut forms.email,
            FieldId::Confirm => &mut forms.confirm,
            FieldId::Search => &mut forms.search,
        }
    }
    fn masked(self) -> bool {
        matches!(self, FieldId::Password | FieldId::Confirm)
    }
}

#[derive(Component)]
pub struct TextField {
    pub id: FieldId,
    pub placeholder: String,
}

#[derive(Component)]
struct FieldText;

#[derive(Resource, Default)]
pub struct FieldFocus(pub Option<FieldId>);

#[derive(Event, Clone, Copy, Debug)]
pub struct FieldSubmit(pub FieldId);

#[derive(Event, Clone, Copy, Debug)]
pub struct FieldChanged(pub FieldId);

pub fn text_field(id: FieldId, placeholder: &str) -> impl Bundle {
    (
        TextField {
            id,
            placeholder: placeholder.into(),
        },
        Button,
        Node {
            width: Val::Percent(100.),
            min_height: Val::Px(34.),
            padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
            border: UiRect::all(Val::Px(1.)),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.35)),
        BorderColor(Color::srgba(1., 1., 1., 0.18)),
    )
}

/// Spawns the field with its text child.
pub fn spawn_text_field(parent: &mut ChildSpawnerCommands, id: FieldId, placeholder: &str) {
    parent
        .spawn(text_field(id, placeholder))
        .with_child((FieldText, label("", 15., WHITE)));
}

pub struct TextFieldPlugin;
impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldFocus>()
            .add_event::<FieldSubmit>()
            .add_event::<FieldChanged>()
            .add_systems(Update, (focus_on_click, typing, render).chain());
    }
}

fn focus_on_click(
    mut focus: ResMut<FieldFocus>,
    fields: Query<(&Interaction, &TextField), Changed<Interaction>>,
) {
    for (interaction, field) in &fields {
        if *interaction == Interaction::Pressed {
            focus.0 = Some(field.id);
        }
    }
}

fn typing(
    mut focus: ResMut<FieldFocus>,
    mut keys: EventReader<KeyboardInput>,
    mut forms: ResMut<HubForms>,
    mut submit: EventWriter<FieldSubmit>,
    mut changed: EventWriter<FieldChanged>,
    fields: Query<&TextField>,
) {
    let Some(id) = focus.0 else {
        keys.clear();
        return;
    };
    if !fields.iter().any(|f| f.id == id) {
        focus.0 = None;
        return;
    }
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => {
                if id.value_mut(&mut forms).pop().is_some() {
                    changed.write(FieldChanged(id));
                }
            }
            Key::Enter => {
                submit.write(FieldSubmit(id));
            }
            Key::Escape => {
                focus.0 = None;
            }
            Key::Tab => {
                let order: Vec<FieldId> = fields.iter().map(|f| f.id).collect();
                if let Some(index) = order.iter().position(|f| *f == id) {
                    focus.0 = Some(order[(index + 1) % order.len()]);
                }
            }
            _ => {
                if let Some(text) = &key.text {
                    let value = id.value_mut(&mut forms);
                    let mut edited = false;
                    for c in text.chars() {
                        if !c.is_control() && value.chars().count() < 64 {
                            value.push(c);
                            edited = true;
                        }
                    }
                    if edited {
                        changed.write(FieldChanged(id));
                    }
                }
            }
        }
    }
}

fn render(
    time: Res<Time<Real>>,
    focus: Res<FieldFocus>,
    forms: Res<HubForms>,
    mut fields: Query<(&TextField, &Children, &mut BorderColor)>,
    mut texts: Query<(&mut Text, &mut TextColor), With<FieldText>>,
) {
    let blink = (time.elapsed_secs() * 2.0) as i32 % 2 == 0;
    for (field, children, mut border) in &mut fields {
        let focused = focus.0 == Some(field.id);
        *border = BorderColor(if focused {
            ACCENT
        } else {
            Color::srgba(1., 1., 1., 0.18)
        });
        let value = field.id.value(&forms);
        for child in children.iter() {
            let Ok((mut text, mut color)) = texts.get_mut(child) else {
                continue;
            };
            let shown = if value.is_empty() {
                if focused && blink {
                    "|".to_string()
                } else if focused {
                    String::new()
                } else {
                    field.placeholder.clone()
                }
            } else {
                let body = if field.id.masked() {
                    "•".repeat(value.chars().count())
                } else {
                    value.to_string()
                };
                if focused && blink {
                    format!("{body}|")
                } else {
                    body
                }
            };
            **text = shown;
            *color = TextColor(if value.is_empty() && !focused {
                MUTED.with_alpha(0.6)
            } else {
                WHITE
            });
        }
    }
}
