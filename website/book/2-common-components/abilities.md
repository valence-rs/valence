# Player Abilities

> Relevant example : [cow_sphere](https://github.com/valence-rs/valence/blob/main/examples/cow_sphere.rs)  
> API : [valence::server::abilities](https://valence.rs/rustdoc/valence/abilities/index.html)

---

Player abilities are a set of flags and values that determine some of the client's capabilities and states.

## Components

We have 3 components in this module:

- `PlayerAbilitiesFlags` : A set of flags that determine what a client can do.
- `FovModifier` : A value that determines the client's field of view.
- `FlyingSpeed` : A value that determines the client's flying speed.

### PlayerAbilitiesFlags

Set of 4 flags :

- `invulnerable` : If the player is invulnerable.
- `flying` : If the player is flying.
- `allow_flying` : If the client can toggle flying.
- `instant_break` : If the client can break blocks instantly.

**Note** : Changing the `GameMode` of the client will change some of his abilities, without triggering change detection.
You can bypass them by updating the `PlayerAbilitiesFlags` component at the same time. (Exept at the init of the `Client`)

<details>
<summary>Example</summary>

```rust
use valence::server::abilities::PlayerAbilitiesFlags;

fn keep_flying_state(
    mut sneak_events: EventReader<SneakEvent>,
    mut clients: Query<(&mut GameMode, &mut PlayerAbilitiesFlags)>,
) {
    for sneak_event in sneak_events.iter() {
        if let Ok((mut gamemode, mut abilities)) = clients.get_mut(sneak_event.client) {
            if sneak_event.state == SneakState::Stop {
                match *gamemode {
                    GameMode::Creative => {
                        *gamemode = GameMode::Survival;
                        abilities.set_allow_flying(true);
                        abilities.set_flying(true);
                    }
                    GameMode::Survival => {
                        *gamemode = GameMode::Creative;
                    }
                    _ => {}
                }
            }
        }
    }
}
```

</details>

## Events

We have 2 events in this module:

- `PlayerStartFlyingEvent` : Triggered when the client starts flying.
- `PlayerStopFlyingEvent` : Triggered when the client stops flying.
