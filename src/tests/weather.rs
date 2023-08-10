use crate::protocol::packets::play::GameStateChangeS2c;
use crate::testing::*;
use crate::weather::{Rain, Thunder, WeatherBundle};

#[test]
fn test_client_initialization_on_join() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer: _,
    } = prepare(true);

    app.update();

    // Check if two game state change packets were sent, one for rain and one for
    // thunder
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(2);
}

#[test]
fn test_chunk_layer_initialization_on_join() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer: _,
    } = prepare(false);

    app.update();

    // Check if two game state change packets were sent, one for rain and one for
    // thunder
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(2);
}

#[test]
fn test_client_rain_change() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = prepare(true);

    app.update();

    helper.clear_received();

    // Change the rain value
    let mut rain = app.world.get_mut::<Rain>(client).unwrap();
    rain.0 = 1.0;

    app.update();

    // Check if a game state change packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(1);
}

#[test]
fn test_client_thunder_change() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = prepare(true);

    app.update();

    helper.clear_received();

    // Change the thunder value
    let mut thunder = app.world.get_mut::<Thunder>(client).unwrap();
    thunder.0 = 1.0;

    app.update();

    // Check if a game state change packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(1);
}

#[test]
fn test_chunk_layer_rain_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare(false);

    app.update();

    helper.clear_received();

    // Change the rain value
    let mut rain = app.world.get_mut::<Rain>(layer).unwrap();
    rain.0 = 1.0;

    app.update();

    // Check if a game state change packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(1);
}

#[test]
fn test_chunk_layer_thunder_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare(false);

    app.update();

    helper.clear_received();

    // Change the thunder value
    let mut thunder = app.world.get_mut::<Thunder>(layer).unwrap();
    thunder.0 = 1.0;

    app.update();

    // Check if a game state change packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<GameStateChangeS2c>(1);
}

fn prepare(client_weather: bool) -> ScenarioSingleClient {
    let mut s = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    s.app.update();

    // Add weather to either the client or the chunk layer depending on the
    // parameter
    if client_weather {
        add_weather_to_client(&mut s);
    } else {
        add_weather_to_chunk_layer(&mut s);
    }

    s
}

fn add_weather_to_client(s: &mut ScenarioSingleClient) {
    s.app.world.entity_mut(s.client).insert(WeatherBundle {
        rain: Rain(0.5),
        thunder: Thunder(0.5),
    });
}

fn add_weather_to_chunk_layer(s: &mut ScenarioSingleClient) {
    s.app.world.entity_mut(s.layer).insert(WeatherBundle {
        rain: Rain(0.5),
        thunder: Thunder(0.5),
    });
}
