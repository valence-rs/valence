+++
title = "News"
description = "View Valence's newest changes"
weight = 5
+++

# 0.2

- more performance
- money
- 1.21.0 support
- ```rust
  fn pause_on_crouch(
    mut events: EventReader<SneakEvent>,
    mut board: ResMut<LifeBoard>,
    mut layers: Query<&mut EntityLayer>,
  ) {
    for event in events.iter() {
        if event.state == SneakState::Start {
            let mut layer = layers.single_mut();

            if board.playing {
                board.playing = false;
                layer.set_action_bar("Paused".italic().color(Color::RED));
            } else {
                board.playing = true;
                layer.set_action_bar("Playing".italic().color(Color::GREEN));
            }
        }
    }
  }
  ```

# 0.1

🥳🎉

- performance
