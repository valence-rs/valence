# Controlling World Time

This module contains Components and Systems needed to update, tick,
broadcast information about the time of day and world age of a
[`ChunkLayer`].

## Enable world time

To control world time of an [`ChunkLayer`], simply insert the
[`WorldTimeBundle`] bundle. We also need to broadcast world time updates to
clients. The [`IntervalBroadcast::default()`] provides configuration to
mimic vanilla behavior:

```rust ignore
fn enable(mut commands: Commands, instance: Entity) {
    commands.entity(instance).insert(WorldTimeBundle::default());
}
```

## Set the time explicitly

Mutating [`WorldTime`] will not automatically broadcast the
change to clients. Mutating [`SetTimeQuery`] to modify time
and broadcast the time changes immediately.

```rust ignore
fn into_the_night(mut instances: Query<(&mut WorldTime, SetTimeQuery), With<Instance>>) {
    for (mut t1, mut t2) in instances.iter_mut() {
        let time_to_set = DayPhase::Night.into();

        // Using [`WorldTime`] - Change won't broadcast immediately
        t1.time_of_day = time_to_set;
        // Using [`SetTimeQuery`] - Change broadcast immediately
        t2.time_of_day = time_to_set;
    }
}
```

## Advacing the world time

Time of day and world age can be ticked individually using
[`LinearTimeTicking`] and [`LinearWorldAging`] respectively.
If these components don't meet your requirements
(eg: you need time increment follow a sine wave ~~for some reason~~),
you can tick the time yourself by modifying the respective
fields on [`WorldTime`].

## Prevent client from automatically update WorldTime

_(mimics `/gamerule doDaylightCycle false`)_

By default, client will continue to update world time if the server
doesn't send packet to sync time between client and server.
This can be toggled by using [`WorldTime::set_client_time_ticking()`]
of [`WorldTime`] to true.

Here is an example of mimicking `/gamerule doDaylightCycle <value>`:

```rust ignore
#[derive(Component)]
pub struct DaylightCycle(pub bool);

fn handle_game_rule_daylight_cycle(
    mut instances: Query<
        (&mut WorldTime, &mut LinearTimeTicking, &DaylightCycle),
        Changed<DaylightCycle>,
    >,
) {
    for (mut time, mut ticking, doCycle) in instances.iter_mut() {
        // Stop client from update
        time.set_client_time_ticking(!doCycle.0);
        ticking.speed = if doCycle.0 { 1 } else { 0 };
    }
}
```
