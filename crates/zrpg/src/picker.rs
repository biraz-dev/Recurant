//! The level-up picker: three abilities, take one.
//!
//! It opens whenever a level-up is waiting and you are in control: at once
//! in normal play, or when you take over from auto mode, which banks its
//! level-ups for you. The world holds still while it is open. Left and right
//! (turn or strafe) choose; Attack or Forward takes.

use bevy::prelude::*;
use zrpg_core::{Command, Perk};

use crate::auto::AutoMode;
use crate::input::{Action, Actions};
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, publish};

/// After taking an ability, input is ignored briefly so the press that took
/// it doesn't also swing or step.
const SETTLE_SECS: f32 = 0.25;
const GOLD: Color = Color::srgb(0.95, 0.75, 0.2);
const INK: Color = Color::srgb(0.9, 0.86, 0.78);
const DIM: Color = Color::srgb(0.62, 0.58, 0.52);
const CARD: Color = Color::srgba(0.12, 0.1, 0.09, 0.92);
const CARD_LIT: Color = Color::srgba(0.26, 0.2, 0.12, 0.96);

pub struct PickerPlugin;

impl Plugin for PickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Picker>()
            .add_systems(PostStartup, spawn)
            .add_systems(
                Update,
                (drive, show)
                    .chain()
                    .after(crate::auto::toggle)
                    .before(Frame::Command),
            );
    }
}

#[derive(Resource, Default)]
pub struct Picker {
    open: bool,
    selected: usize,
    settle: f32,
}

impl Picker {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// While true, the game neither runs nor takes commands.
    pub fn holds_the_game(&self) -> bool {
        self.open || self.settle > 0.0
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Title;
#[derive(Component)]
struct Waiting;
#[derive(Component)]
struct Card(usize);
#[derive(Component)]
struct CardName(usize);
#[derive(Component)]
struct CardStacks(usize);
#[derive(Component)]
struct CardText(usize);

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn spawn(mut commands: Commands, cams: Query<Entity, With<PlayerCamera>>) {
    let Ok(cam) = cams.single() else {
        return;
    };
    commands
        .spawn((
            Root,
            UiTargetCamera(cam),
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((Title, Text::new(""), font(16.0), TextColor(GOLD)));
            root.spawn((Text::new("CHOOSE AN ABILITY"), font(8.0), TextColor(INK)));
            root.spawn(Node {
                column_gap: px(5),
                margin: UiRect::vertical(px(4)),
                ..default()
            })
            .with_children(|row| {
                for i in 0..3 {
                    row.spawn((
                        Card(i),
                        Node {
                            width: px(98),
                            height: px(84),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(3),
                            padding: UiRect::all(px(4)),
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                        BorderColor::all(DIM),
                        BackgroundColor(CARD),
                    ))
                    .with_children(|card| {
                        card.spawn((CardName(i), Text::new(""), font(9.0), TextColor(GOLD)));
                        card.spawn((CardStacks(i), Text::new(""), font(7.0), TextColor(DIM)));
                        card.spawn((CardText(i), Text::new(""), font(8.0), TextColor(INK)));
                    });
                }
            });
            root.spawn((Waiting, Text::new(""), font(8.0), TextColor(DIM)));
            root.spawn((
                Text::new("< >  choose     ATTACK  take"),
                font(7.0),
                TextColor(DIM),
            ));
        });
}

fn drive(
    time: Res<Time>,
    actions: Res<Actions>,
    auto: Res<AutoMode>,
    mut picker: ResMut<Picker>,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<zrpg_core::Event>>,
) {
    picker.settle = (picker.settle - time.delta_secs()).max(0.0);
    let waiting = !run.pending.is_empty() && run.player.is_alive();
    let open = waiting && !auto.0;
    if open && !picker.open {
        picker.selected = 1;
    }
    picker.open = open;
    if !open || picker.settle > 0.0 {
        return;
    }
    let count = run.pending[0].len();
    let pressed = |a: &[Action]| a.iter().any(|&a| actions.just_pressed(a));
    if pressed(&[Action::TurnLeft, Action::StrafeLeft]) {
        picker.selected = picker.selected.saturating_sub(1);
    }
    if pressed(&[Action::TurnRight, Action::StrafeRight]) {
        picker.selected = (picker.selected + 1).min(count.saturating_sub(1));
    }
    if pressed(&[Action::Attack, Action::Forward]) {
        run.apply(Command::Choose(picker.selected as u8), &mut buf);
        publish(&mut buf, &mut writer);
        picker.settle = SETTLE_SECS;
        picker.selected = 1;
        picker.open = !run.pending.is_empty();
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show(
    picker: Res<Picker>,
    run: Res<Run>,
    mut root: Query<&mut Visibility, With<Root>>,
    mut title: Query<&mut Text, (With<Title>, Without<Waiting>)>,
    mut waiting: Query<&mut Text, (With<Waiting>, Without<Title>)>,
    mut cards: Query<
        (
            &Card,
            &mut BorderColor,
            &mut BackgroundColor,
            &mut Visibility,
        ),
        Without<Root>,
    >,
    mut names: Query<(&CardName, &mut Text), (Without<Title>, Without<Waiting>)>,
    mut stacks: Query<
        (&CardStacks, &mut Text),
        (Without<CardName>, Without<Title>, Without<Waiting>),
    >,
    mut texts: Query<
        (&CardText, &mut Text),
        (
            Without<CardStacks>,
            Without<CardName>,
            Without<Title>,
            Without<Waiting>,
        ),
    >,
) {
    let shown = picker.open && !run.pending.is_empty();
    for mut v in &mut root {
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    if !shown {
        return;
    }
    let offer: &[Perk] = &run.pending[0];
    let level = run.player.level + 1 - run.pending.len() as u32;
    set(&mut title, format!("LEVEL {level}"));
    let more = run.pending.len() - 1;
    set(
        &mut waiting,
        match more {
            0 => String::new(),
            1 => "1 more level-up waiting".into(),
            n => format!("{n} more level-ups waiting"),
        },
    );
    for (card, mut border, mut bg, mut v) in &mut cards {
        let lit = card.0 == picker.selected;
        *v = if card.0 < offer.len() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        border.set_all(if lit { GOLD } else { DIM });
        bg.0 = if lit { CARD_LIT } else { CARD };
    }
    for (c, mut text) in &mut names {
        let label = offer.get(c.0).map_or("", |p| p.name());
        if text.0 != label {
            text.0 = label.into();
        }
    }
    for (c, mut text) in &mut stacks {
        let label = offer.get(c.0).map_or(String::new(), |&p| {
            let n = run.player.perks.count(p);
            if n == 0 {
                "new".into()
            } else {
                format!("have {n} of {}", p.max_stacks())
            }
        });
        if text.0 != label {
            text.0 = label;
        }
    }
    for (c, mut text) in &mut texts {
        let label = offer.get(c.0).map_or("", |p| p.description());
        if text.0 != label {
            text.0 = label.into();
        }
    }
}

fn set<F: bevy::ecs::query::QueryFilter>(q: &mut Query<&mut Text, F>, label: String) {
    for mut text in q.iter_mut() {
        if text.0 != label {
            text.0 = label.clone();
        }
    }
}
