use super::*;

pub(super) fn setup_debug_ui(
    mut commands: Commands,
    panel_state: Res<DebugPanelState>,
    asset_server: Res<AssetServer>,
) {
    // Debug UI panel - draggable window (persists across game states)
    commands
        .spawn((
            DebugUiEntity,
            DebugUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(panel_state.position.x),
                top: Val::Px(panel_state.position.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(250),
        ))
        .with_children(|parent| {
            // Draggable header bar
            parent
                .spawn((
                    DebugPanelHeader,
                    Button,
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                    BorderRadius::top(Val::Px(8.0)),
                ))
                .with_children(|header| {
                    // Title
                    header.spawn((
                        Text::new("DEBUG"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 0.0)),
                    ));

                    // Toggle button (collapse/expand) with chevron icon
                    header
                        .spawn((
                            DebugToggleButton,
                            Button,
                            Node {
                                width: Val::Px(24.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.9)),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_child((
                            DebugToggleIcon,
                            ImageNode {
                                image: asset_server.load("models/images/icon-chevron-down-48.png"),
                                color: Color::WHITE,
                                ..default()
                            },
                            Node {
                                width: Val::Px(16.0),
                                height: Val::Px(16.0),
                                ..default()
                            },
                        ));
                });

            // Collapsible content panel
            parent
                .spawn((
                    DebugPanelContent,
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                    BorderRadius::bottom(Val::Px(8.0)),
                ))
                .with_children(|content| {
                    // Target selection row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            flex_wrap: FlexWrap::Wrap,
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_debug_button!(row, "Char", DebugButton::SelectCharacter);
                            spawn_debug_button!(row, "Scene", DebugButton::SelectScene);
                            spawn_debug_button!(row, "Cam", DebugButton::SelectCamera);
                        });

                    // Position display
                    content.spawn((
                        DebugDisplayText,
                        Text::new("Loading..."),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Position X row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("X:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::PosXMinus);
                            spawn_debug_button!(row, "+", DebugButton::PosXPlus);
                        });

                    // Position Y row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Y:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::PosYMinus);
                            spawn_debug_button!(row, "+", DebugButton::PosYPlus);
                        });

                    // Position Z row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Z:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::PosZMinus);
                            spawn_debug_button!(row, "+", DebugButton::PosZPlus);
                        });

                    // Scale row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Scale:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::ScaleMinus);
                            spawn_debug_button!(row, "+", DebugButton::ScalePlus);
                        });

                    // Rotation X row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("RotX:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::RotXMinus);
                            spawn_debug_button!(row, "+", DebugButton::RotXPlus);
                        });

                    // Rotation Y row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("RotY:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "-", DebugButton::RotYMinus);
                            spawn_debug_button!(row, "+", DebugButton::RotYPlus);
                        });

                    // Action buttons
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_debug_button!(row, "Save", DebugButton::SavePositions);
                            spawn_debug_button!(row, "Reset", DebugButton::ResetPositions);
                        });

                    // Debug visualization toggles
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_debug_button!(row, "Skeleton", DebugButton::ToggleSkeleton);
                            spawn_debug_button!(row, "Hitboxes", DebugButton::ToggleHitboxes);
                        });

                    // Animation cycling row
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Anim:"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                Node {
                                    width: Val::Px(35.0),
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, "<", DebugButton::AnimPrev);
                            row.spawn((
                                AnimationIndexDisplay,
                                Text::new("15"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                                Node {
                                    width: Val::Px(25.0),
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                            ));
                            spawn_debug_button!(row, ">", DebugButton::AnimNext);
                        });

                    // Post-processing section header
                    content.spawn((
                        Text::new("— POST PROCESS —"),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.3)),
                        Node {
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                    ));

                    // Bloom intensity slider
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    Text::new("Bloom"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                ));
                                row.spawn((
                                    BloomIntensityValue,
                                    Text::new("0.30"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                            col.spawn((
                                BloomIntensitySlider,
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(12.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 1.0)),
                                BorderRadius::all(Val::Px(2.0)),
                            ))
                            .with_child((
                                Node {
                                    width: Val::Percent(30.0), // 0.3 out of 1.0
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.3, 0.5, 0.8)),
                                BorderRadius::left(Val::Px(2.0)),
                            ));
                        });

                    // Contrast slider
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    Text::new("Contrast"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                ));
                                row.spawn((
                                    ContrastValue,
                                    Text::new("1.00"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                            col.spawn((
                                ContrastSlider,
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(12.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 1.0)),
                                BorderRadius::all(Val::Px(2.0)),
                            ))
                            .with_child((
                                Node {
                                    width: Val::Percent(50.0), // 1.0 normalized to 0.5 in range 0-2
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.3, 0.5, 0.8)),
                                BorderRadius::left(Val::Px(2.0)),
                            ));
                        });

                    // Saturation slider
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    Text::new("Saturation"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                ));
                                row.spawn((
                                    SaturationValue,
                                    Text::new("1.00"),
                                    TextFont {
                                        font_size: 10.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                            col.spawn((
                                SaturationSlider,
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(12.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 1.0)),
                                BorderRadius::all(Val::Px(2.0)),
                            ))
                            .with_child((
                                Node {
                                    width: Val::Percent(50.0), // 1.0 normalized to 0.5 in range 0-2
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.3, 0.5, 0.8)),
                                BorderRadius::left(Val::Px(2.0)),
                            ));
                        });

                    // Keyboard help
                    content.spawn((
                        Text::new("WASD QE RF +/-"),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                });
        });
}

pub(super) fn handle_debug_buttons(
    mut interaction_query: Query<
        (&Interaction, &DebugButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut target: ResMut<DebugTarget>,
    mut panel_state: ResMut<DebugPanelState>,
    mut debug_render_context: Option<ResMut<DebugRenderContext>>,
    shared_animations: Option<Res<SharedAnimations>>,
    mut char_query: Query<
        &mut Transform,
        (
            With<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    mut scene_query: Query<
        &mut Transform,
        (
            With<MenuBackground>,
            Without<HomePlayerModel>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    mut camera_query: Query<
        &mut Transform,
        (
            With<HomeSceneCamera>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<WarehouseScene>,
        ),
    >,
    mut warehouse_query: Query<
        (&mut Transform, &mut Visibility),
        (
            With<WarehouseScene>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
        ),
    >,
) {
    // Get animation count, default to 1 if not loaded yet
    let anim_count = shared_animations.as_ref().map(|a| a.count).unwrap_or(1);
    let move_amount = 0.1;
    let scale_factor = 1.1;
    let rot_amount = 0.1; // radians

    for (interaction, button, mut bg) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_ACTIVE);

                match button {
                    DebugButton::SelectCharacter => *target = DebugTarget::Character,
                    DebugButton::SelectScene => *target = DebugTarget::Scene,
                    DebugButton::SelectCamera => *target = DebugTarget::Camera,
                    DebugButton::SavePositions => {
                        save_positions(
                            &char_query,
                            &scene_query,
                            &camera_query,
                            &warehouse_query,
                            panel_state.current_animation_index,
                        );
                    }
                    DebugButton::ResetPositions => {
                        reset_positions(
                            &mut char_query,
                            &mut scene_query,
                            &mut camera_query,
                            &mut warehouse_query,
                        );
                    }
                    DebugButton::ToggleSkeleton => {
                        panel_state.show_skeleton = !panel_state.show_skeleton;
                        info!(
                            "Skeleton visualization: {}",
                            if panel_state.show_skeleton {
                                "ON"
                            } else {
                                "OFF"
                            }
                        );
                    }
                    DebugButton::ToggleHitboxes => {
                        panel_state.show_hitboxes = !panel_state.show_hitboxes;
                        if let Some(ref mut ctx) = debug_render_context {
                            ctx.enabled = panel_state.show_hitboxes;
                        }
                        info!(
                            "Hitbox visualization: {}",
                            if panel_state.show_hitboxes {
                                "ON"
                            } else {
                                "OFF"
                            }
                        );
                    }
                    DebugButton::AnimPrev => {
                        if panel_state.current_animation_index > 0 {
                            panel_state.current_animation_index -= 1;
                        } else {
                            panel_state.current_animation_index = anim_count.saturating_sub(1);
                            // Wrap to last
                        }
                        info!(
                            "Animation index: {} / {}",
                            panel_state.current_animation_index, anim_count
                        );
                    }
                    DebugButton::AnimNext => {
                        if panel_state.current_animation_index < anim_count.saturating_sub(1) {
                            panel_state.current_animation_index += 1;
                        } else {
                            panel_state.current_animation_index = 0; // Wrap to first
                        }
                        info!(
                            "Animation index: {} / {}",
                            panel_state.current_animation_index, anim_count
                        );
                    }
                    _ => {
                        // Position/scale/rotation adjustments
                        let transform: Option<Mut<Transform>> = match *target {
                            DebugTarget::Character => char_query.iter_mut().next(),
                            DebugTarget::Scene => scene_query.iter_mut().next(),
                            DebugTarget::Camera => camera_query.iter_mut().next(),
                            DebugTarget::Warehouse => {
                                warehouse_query.iter_mut().next().map(|(t, _)| t)
                            }
                        };

                        if let Some(mut t) = transform {
                            match button {
                                DebugButton::PosXPlus => t.translation.x += move_amount,
                                DebugButton::PosXMinus => t.translation.x -= move_amount,
                                DebugButton::PosYPlus => t.translation.y += move_amount,
                                DebugButton::PosYMinus => t.translation.y -= move_amount,
                                DebugButton::PosZPlus => t.translation.z += move_amount,
                                DebugButton::PosZMinus => t.translation.z -= move_amount,
                                DebugButton::ScalePlus => t.scale *= scale_factor,
                                DebugButton::ScaleMinus => t.scale /= scale_factor,
                                DebugButton::RotXPlus => t.rotate_x(rot_amount),
                                DebugButton::RotXMinus => t.rotate_x(-rot_amount),
                                DebugButton::RotYPlus => t.rotate_y(rot_amount),
                                DebugButton::RotYMinus => t.rotate_y(-rot_amount),
                                _ => {}
                            }
                        }
                    }
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(BTN_HOVER);
            }
            Interaction::None => {
                // Highlight active target/toggle buttons
                let is_active = matches!(
                    (button, *target),
                    (DebugButton::SelectCharacter, DebugTarget::Character)
                        | (DebugButton::SelectScene, DebugTarget::Scene)
                        | (DebugButton::SelectCamera, DebugTarget::Camera)
                ) || matches!(
                    button,
                    DebugButton::ToggleSkeleton if panel_state.show_skeleton
                ) || matches!(
                    button,
                    DebugButton::ToggleHitboxes if panel_state.show_hitboxes
                );
                *bg = if is_active {
                    BackgroundColor(BTN_ACTIVE)
                } else {
                    BackgroundColor(BTN_NORMAL)
                };
            }
        }
    }
}

pub(super) fn keyboard_debug_controls(
    keys: Res<ButtonInput<KeyCode>>,
    target: Res<DebugTarget>,
    mut char_query: Query<
        &mut Transform,
        (
            With<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    mut scene_query: Query<
        &mut Transform,
        (
            With<MenuBackground>,
            Without<HomePlayerModel>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    mut camera_query: Query<
        &mut Transform,
        (
            With<HomeSceneCamera>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<WarehouseScene>,
        ),
    >,
    mut warehouse_query: Query<
        &mut Transform,
        (
            With<WarehouseScene>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
        ),
    >,
    time: Res<Time>,
) {
    let move_speed = 2.0 * time.delta_secs();
    let scale_speed = 1.5 * time.delta_secs();

    let transform: Option<Mut<Transform>> = match *target {
        DebugTarget::Character => char_query.iter_mut().next(),
        DebugTarget::Scene => scene_query.iter_mut().next(),
        DebugTarget::Camera => camera_query.iter_mut().next(),
        DebugTarget::Warehouse => warehouse_query.iter_mut().next(),
    };

    let rot_speed = 2.0 * time.delta_secs();

    if let Some(mut t) = transform {
        // WASD for X/Z
        if keys.pressed(KeyCode::KeyW) {
            t.translation.z -= move_speed;
        }
        if keys.pressed(KeyCode::KeyS) {
            t.translation.z += move_speed;
        }
        if keys.pressed(KeyCode::KeyA) {
            t.translation.x -= move_speed;
        }
        if keys.pressed(KeyCode::KeyD) {
            t.translation.x += move_speed;
        }
        // Q/E for Y
        if keys.pressed(KeyCode::KeyQ) {
            t.translation.y -= move_speed;
        }
        if keys.pressed(KeyCode::KeyE) {
            t.translation.y += move_speed;
        }
        // +/- for scale
        if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
            t.scale *= 1.0 + scale_speed;
        }
        if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
            t.scale *= 1.0 - scale_speed;
        }
        // R/F for rotation
        if keys.pressed(KeyCode::KeyR) {
            t.rotate_y(rot_speed);
        }
        if keys.pressed(KeyCode::KeyF) {
            t.rotate_y(-rot_speed);
        }
    }

    // Tab to cycle targets
    if keys.just_pressed(KeyCode::Tab) {
        // Note: Can't mutate target here since we borrowed it immutably
        // This would need a separate system or different approach
    }
}

pub(super) fn update_debug_display(
    target: Res<DebugTarget>,
    char_query: Query<
        &Transform,
        (
            With<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    scene_query: Query<
        &Transform,
        (
            With<MenuBackground>,
            Without<HomePlayerModel>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    camera_query: Query<
        &Transform,
        (
            With<HomeSceneCamera>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<WarehouseScene>,
        ),
    >,
    warehouse_query: Query<
        &Transform,
        (
            With<WarehouseScene>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
        ),
    >,
    mut text_query: Query<&mut Text, With<DebugDisplayText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let target_name = match *target {
        DebugTarget::Character => "CHARACTER",
        DebugTarget::Scene => "SCENE",
        DebugTarget::Camera => "CAMERA",
        DebugTarget::Warehouse => "WAREHOUSE",
    };

    let (pos, scale, rot_y) = match *target {
        DebugTarget::Character => char_query
            .iter()
            .next()
            .map(|t| {
                let (_, y, _) = t.rotation.to_euler(EulerRot::YXZ);
                (t.translation, t.scale.x, y)
            })
            .unwrap_or((Vec3::ZERO, 1.0, 0.0)),
        DebugTarget::Scene => scene_query
            .iter()
            .next()
            .map(|t| {
                let (_, y, _) = t.rotation.to_euler(EulerRot::YXZ);
                (t.translation, t.scale.x, y)
            })
            .unwrap_or((Vec3::ZERO, 1.0, 0.0)),
        DebugTarget::Camera => camera_query
            .iter()
            .next()
            .map(|t| {
                let (_, y, _) = t.rotation.to_euler(EulerRot::YXZ);
                (t.translation, t.scale.x, y)
            })
            .unwrap_or((Vec3::ZERO, 1.0, 0.0)),
        DebugTarget::Warehouse => warehouse_query
            .iter()
            .next()
            .map(|t| {
                let (_, y, _) = t.rotation.to_euler(EulerRot::YXZ);
                (t.translation, t.scale.x, y)
            })
            .unwrap_or((Vec3::ZERO, 1.0, 0.0)),
    };

    **text = format!(
        "Target: {}\nPos: ({:.3}, {:.3}, {:.3})\nScale: {:.4}\nRot Y: {:.2}°",
        target_name,
        pos.x,
        pos.y,
        pos.z,
        scale,
        rot_y.to_degrees()
    );
}

pub(super) fn save_positions(
    char_query: &Query<
        &mut Transform,
        (
            With<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    scene_query: &Query<
        &mut Transform,
        (
            With<MenuBackground>,
            Without<HomePlayerModel>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    camera_query: &Query<
        &mut Transform,
        (
            With<HomeSceneCamera>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<WarehouseScene>,
        ),
    >,
    warehouse_query: &Query<
        (&mut Transform, &mut Visibility),
        (
            With<WarehouseScene>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
        ),
    >,
    animation_index: usize,
) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut output = format!("=== Home Scene Positions ({})\n\n", timestamp);
    output.push_str(&format!("ANIMATION:\n  index: {}\n\n", animation_index));

    if let Some(t) = char_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "CHARACTER:\n  position: Vec3::new({:.4}, {:.4}, {:.4})\n  scale: Vec3::splat({:.6})\n  rotation: Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4})\n\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }

    if let Some(t) = scene_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "SCENE (Dust 2 vignette):\n  position: Vec3::new({:.4}, {:.4}, {:.4})\n  scale: Vec3::splat({:.6})\n  rotation: Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4})\n\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }

    if let Some(t) = camera_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "CAMERA:\n  position: Vec3::new({:.4}, {:.4}, {:.4})\n  rotation: Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4})\n\n",
            t.translation.x, t.translation.y, t.translation.z,
            axis.x, axis.y, axis.z, angle
        ));
    }

    if let Some((t, _)) = warehouse_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "WAREHOUSE (MapConfig):\n  map_position: Vec3::new({:.4}, {:.4}, {:.4})\n  map_scale: {:.6}\n  map_rotation: Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4})\n\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }

    // Generate copy-paste ready code
    output.push_str("// Copy-paste ready code:\n");
    if let Some(t) = char_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "CHARACTER: Transform::from_xyz({:.4}, {:.4}, {:.4}).with_scale(Vec3::splat({:.6})).with_rotation(Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4}))\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }
    if let Some(t) = scene_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "SCENE: Transform::from_xyz({:.4}, {:.4}, {:.4}).with_scale(Vec3::splat({:.6})).with_rotation(Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4}))\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }
    if let Some((t, _)) = warehouse_query.iter().next() {
        let (axis, angle) = t.rotation.to_axis_angle();
        output.push_str(&format!(
            "// MapConfig for level.rs:\nMapConfig {{\n    player_spawn: Vec3::new(0.0, 2.0, 0.0),\n    map_position: Vec3::new({:.4}, {:.4}, {:.4}),\n    map_scale: {:.6},\n    map_rotation: Quat::from_axis_angle(Vec3::new({:.4}, {:.4}, {:.4}), {:.4}),\n    post_process: PostProcessSettings::default(),\n}}\n",
            t.translation.x, t.translation.y, t.translation.z, t.scale.x,
            axis.x, axis.y, axis.z, angle
        ));
    }

    // Save to file
    let path = "home_scene_positions.log";
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", output) {
                eprintln!("Failed to write positions: {}", e);
            } else {
                println!("Positions saved to {}", path);
                println!("{}", output);
            }
        }
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
            // Still print to console
            println!("{}", output);
        }
    }
}

pub(super) fn reset_positions(
    char_query: &mut Query<
        &mut Transform,
        (
            With<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    scene_query: &mut Query<
        &mut Transform,
        (
            With<MenuBackground>,
            Without<HomePlayerModel>,
            Without<HomeSceneCamera>,
            Without<WarehouseScene>,
        ),
    >,
    camera_query: &mut Query<
        &mut Transform,
        (
            With<HomeSceneCamera>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<WarehouseScene>,
        ),
    >,
    warehouse_query: &mut Query<
        (&mut Transform, &mut Visibility),
        (
            With<WarehouseScene>,
            Without<HomePlayerModel>,
            Without<MenuBackground>,
            Without<HomeSceneCamera>,
        ),
    >,
) {
    for mut t in char_query.iter_mut() {
        t.translation = Vec3::new(0.0, 0.0, 1.6615);
        t.scale = Vec3::ONE;
        t.rotation = Quat::from_axis_angle(Vec3::new(0.0, -1.0, 0.0), 0.0332);
    }
    for mut t in scene_query.iter_mut() {
        t.translation = Vec3::new(5.2099, 0.0, 2.4632);
        t.scale = Vec3::splat(1.753397);
        t.rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.6154);
    }
    for mut t in camera_query.iter_mut() {
        *t = Transform::from_xyz(-0.0329, 2.7334, 5.4840)
            .with_rotation(Quat::from_axis_angle(Vec3::new(-1.0, 0.0, 0.0), 0.1244));
    }
    for (mut t, mut vis) in warehouse_query.iter_mut() {
        t.translation = Vec3::ZERO;
        t.scale = Vec3::splat(1.0);
        t.rotation = Quat::IDENTITY;
        *vis = Visibility::Hidden;
    }
}

pub(super) fn handle_debug_toggle(
    mut panel_state: ResMut<DebugPanelState>,
    toggle_query: Query<&Interaction, (Changed<Interaction>, With<DebugToggleButton>)>,
    mut content_query: Query<&mut Visibility, With<DebugPanelContent>>,
    mut icon_query: Query<&mut ImageNode, With<DebugToggleIcon>>,
    asset_server: Res<AssetServer>,
) {
    for interaction in &toggle_query {
        if *interaction == Interaction::Pressed {
            panel_state.expanded = !panel_state.expanded;

            // Update content visibility
            if let Ok(mut visibility) = content_query.single_mut() {
                *visibility = if panel_state.expanded {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }

            // Update chevron icon direction
            if let Ok(mut image_node) = icon_query.single_mut() {
                image_node.image = if panel_state.expanded {
                    asset_server.load("models/images/icon-chevron-down-48.png")
                } else {
                    asset_server.load("models/images/icon-chevron-up-48.png")
                };
            }
        }
    }
}

pub(super) fn handle_debug_drag(
    mut panel_state: ResMut<DebugPanelState>,
    header_query: Query<&Interaction, With<DebugPanelHeader>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    for interaction in &header_query {
        match *interaction {
            Interaction::Pressed => {
                if !panel_state.dragging {
                    panel_state.dragging = true;
                    panel_state.drag_offset = cursor_pos - panel_state.position;
                }
            }
            _ => {}
        }
    }

    if panel_state.dragging {
        if mouse_button.pressed(MouseButton::Left) {
            panel_state.position = cursor_pos - panel_state.drag_offset;
            // Clamp to window bounds
            panel_state.position.x = panel_state.position.x.max(0.0);
            panel_state.position.y = panel_state.position.y.max(0.0);
        } else {
            panel_state.dragging = false;
        }
    }
}

pub(super) fn update_debug_panel_position(
    panel_state: Res<DebugPanelState>,
    mut root_query: Query<&mut Node, With<DebugUiRoot>>,
) {
    if !panel_state.is_changed() {
        return;
    }

    if let Ok(mut node) = root_query.single_mut() {
        node.left = Val::Px(panel_state.position.x);
        node.top = Val::Px(panel_state.position.y);
    }
}

/// Gizmo config for skeleton (renders on top of meshes)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub(super) struct SkeletonGizmos;

/// Configure skeleton gizmos to render on top of meshes
pub(super) fn setup_skeleton_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<SkeletonGizmos>();
    // Large negative depth_bias pushes gizmos towards the camera
    config.depth_bias = -100.0;
}

/// Draw skeleton gizmos for all player models (lines + spheres at joints)
pub(super) fn draw_skeleton_gizmos(
    mut skeleton_gizmos: Gizmos<SkeletonGizmos>,
    debug_state: Res<DebugPanelState>,
    // Query for all player models (home scene + in-game)
    player_model_query: Query<Entity, With<PlayerModel>>,
    home_player_query: Query<Entity, With<HomePlayerModel>>,
    children_query: Query<&Children>,
    transform_query: Query<(&GlobalTransform, Option<&Name>)>,
) {
    if !debug_state.show_skeleton {
        return;
    }

    // Colors for skeleton visualization
    let bone_color = Color::srgb(0.0, 1.0, 0.0); // Green for bones
    let joint_color = Color::srgb(1.0, 0.0, 0.0); // Red for joints (more visible)
    let joint_radius = 0.03; // Slightly larger spheres at joints

    // Collect all entities to process (both PlayerModel and HomePlayerModel)
    let mut entities_to_process: Vec<Entity> = player_model_query.iter().collect();
    for entity in home_player_query.iter() {
        if !entities_to_process.contains(&entity) {
            entities_to_process.push(entity);
        }
    }

    for root_entity in entities_to_process {
        // Recursively draw skeleton for this entity hierarchy
        draw_skeleton_recursive(
            root_entity,
            None, // No parent position for root
            &children_query,
            &transform_query,
            &mut skeleton_gizmos,
            bone_color,
            joint_color,
            joint_radius,
        );
    }
}

/// Recursively traverse entity hierarchy and draw skeleton bones
pub(super) fn draw_skeleton_recursive(
    entity: Entity,
    parent_position: Option<Vec3>,
    children_query: &Query<&Children>,
    transform_query: &Query<(&GlobalTransform, Option<&Name>)>,
    gizmos: &mut Gizmos<SkeletonGizmos>,
    bone_color: Color,
    joint_color: Color,
    joint_radius: f32,
) {
    let Ok((global_transform, name)) = transform_query.get(entity) else {
        return;
    };

    let position = global_transform.translation();

    // Check if this entity is a bone (has mixamorig: in name or is _rootJoint)
    let is_bone = name
        .map(|n| {
            let s = n.as_str();
            s.contains("mixamorig:") || s == "_rootJoint" || s == "Armature"
        })
        .unwrap_or(false);

    if is_bone {
        // Draw joint sphere at this bone's position
        gizmos.sphere(
            Isometry3d::from_translation(position),
            joint_radius,
            joint_color,
        );

        // Draw line from parent bone to this bone
        if let Some(parent_pos) = parent_position {
            gizmos.line(parent_pos, position, bone_color);
        }
    }

    // Recurse into children
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            draw_skeleton_recursive(
                child,
                if is_bone {
                    Some(position)
                } else {
                    parent_position
                },
                children_query,
                transform_query,
                gizmos,
                bone_color,
                joint_color,
                joint_radius,
            );
        }
    }
}

/// Update the animation index display text
pub(super) fn update_animation_index_display(
    panel_state: Res<DebugPanelState>,
    shared_animations: Option<Res<SharedAnimations>>,
    mut query: Query<&mut Text, With<AnimationIndexDisplay>>,
) {
    if !panel_state.is_changed() {
        return;
    }
    let count = shared_animations.as_ref().map(|a| a.count).unwrap_or(0);
    for mut text in &mut query {
        **text = format!("{}/{}", panel_state.current_animation_index, count);
    }
}

/// Update the home player animation when the index changes

pub(super) fn handle_postprocess_sliders(
    bloom_slider: Query<(&Interaction, &Node, &GlobalTransform), With<BloomIntensitySlider>>,
    contrast_slider: Query<
        (&Interaction, &Node, &GlobalTransform),
        (With<ContrastSlider>, Without<BloomIntensitySlider>),
    >,
    saturation_slider: Query<
        (&Interaction, &Node, &GlobalTransform),
        (
            With<SaturationSlider>,
            Without<BloomIntensitySlider>,
            Without<ContrastSlider>,
        ),
    >,
    windows: Query<&Window>,
    mut debug_state: ResMut<DebugPanelState>,
    mut bloom_query: Query<&mut Bloom, With<HomeSceneCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Handle bloom intensity slider (range 0.0 - 1.0)
    for (interaction, node, transform) in &bloom_slider {
        if *interaction == Interaction::Pressed {
            if let Some(new_value) =
                calculate_postprocess_slider_value(cursor_pos, node, transform, 0.0, 1.0)
            {
                debug_state.bloom_intensity = new_value;
                // Update the actual bloom component
                if let Ok(mut bloom) = bloom_query.single_mut() {
                    bloom.intensity = new_value;
                }
            }
        }
    }

    // Handle contrast slider (range 0.0 - 2.0)
    for (interaction, node, transform) in &contrast_slider {
        if *interaction == Interaction::Pressed {
            if let Some(new_value) =
                calculate_postprocess_slider_value(cursor_pos, node, transform, 0.0, 2.0)
            {
                debug_state.contrast = new_value;
            }
        }
    }

    // Handle saturation slider (range 0.0 - 2.0)
    for (interaction, node, transform) in &saturation_slider {
        if *interaction == Interaction::Pressed {
            if let Some(new_value) =
                calculate_postprocess_slider_value(cursor_pos, node, transform, 0.0, 2.0)
            {
                debug_state.saturation = new_value;
            }
        }
    }
}

/// Calculate slider value from cursor position
pub(super) fn calculate_postprocess_slider_value(
    cursor_pos: Vec2,
    node: &Node,
    transform: &GlobalTransform,
    min: f32,
    max: f32,
) -> Option<f32> {
    let width = match node.width {
        Val::Px(w) => w,
        Val::Percent(p) => p * 5.0,
        _ => return None,
    };

    let left = transform.translation().x - width / 2.0;
    let normalized = ((cursor_pos.x - left) / width).clamp(0.0, 1.0);
    Some(min + normalized * (max - min))
}

/// Update post-processing value displays and slider fills
pub(super) fn update_postprocess_displays(
    debug_state: Res<DebugPanelState>,
    mut bloom_value: Query<
        &mut Text,
        (
            With<BloomIntensityValue>,
            Without<ContrastValue>,
            Without<SaturationValue>,
        ),
    >,
    mut contrast_value: Query<
        &mut Text,
        (
            With<ContrastValue>,
            Without<BloomIntensityValue>,
            Without<SaturationValue>,
        ),
    >,
    mut saturation_value: Query<
        &mut Text,
        (
            With<SaturationValue>,
            Without<BloomIntensityValue>,
            Without<ContrastValue>,
        ),
    >,
    bloom_slider: Query<&Children, With<BloomIntensitySlider>>,
    contrast_slider: Query<&Children, (With<ContrastSlider>, Without<BloomIntensitySlider>)>,
    saturation_slider: Query<
        &Children,
        (
            With<SaturationSlider>,
            Without<BloomIntensitySlider>,
            Without<ContrastSlider>,
        ),
    >,
    mut fill_query: Query<&mut Node, Without<BloomIntensitySlider>>,
) {
    if !debug_state.is_changed() {
        return;
    }

    // Update bloom display
    if let Ok(mut text) = bloom_value.single_mut() {
        **text = format!("{:.2}", debug_state.bloom_intensity);
    }
    if let Ok(children) = bloom_slider.single() {
        for child in children.iter() {
            if let Ok(mut node) = fill_query.get_mut(child) {
                node.width = Val::Percent(debug_state.bloom_intensity * 100.0);
            }
        }
    }

    // Update contrast display
    if let Ok(mut text) = contrast_value.single_mut() {
        **text = format!("{:.2}", debug_state.contrast);
    }
    if let Ok(children) = contrast_slider.single() {
        for child in children.iter() {
            if let Ok(mut node) = fill_query.get_mut(child) {
                node.width = Val::Percent((debug_state.contrast / 2.0) * 100.0);
            }
        }
    }

    // Update saturation display
    if let Ok(mut text) = saturation_value.single_mut() {
        **text = format!("{:.2}", debug_state.saturation);
    }
    if let Ok(children) = saturation_slider.single() {
        for child in children.iter() {
            if let Ok(mut node) = fill_query.get_mut(child) {
                node.width = Val::Percent((debug_state.saturation / 2.0) * 100.0);
            }
        }
    }
}
