use valence_ident::ident;

use super::*;

#[test]
fn text_round_trip() {
    let before = "foo".color(Color::RED).bold()
        + ("bar".obfuscated().color(Color::YELLOW)
            + "baz".underlined().not_bold().italic().color(Color::BLACK));

    let json = format!("{before:#}");

    let after = Text::from_str(&json).unwrap();

    println!("==== Before ====\n");
    println!("{before:#?}");
    println!("==== After ====\n");
    println!("{after:#?}");

    assert_eq!(before, after);
    assert_eq!(before.to_string(), after.to_string());
}

#[test]
fn non_object_data_types() {
    let input = r#"["foo", true, false, 1.9E10, 9999]"#;
    let txt: Text = serde_json::from_str(input).unwrap();

    assert_eq!(txt, "foo".into_text() + true + false + 1.9E10 + 9999);
}

#[test]
fn translate() {
    let txt = Text::translate(
        "chat.type.advancement.task",
        ["arg1".into_text(), "arg2".into_text()],
    );
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    assert_eq!(
        serialized,
        r#"{"translate":"chat.type.advancement.task","with":[{"text":"arg1"},{"text":"arg2"}]}"#
    );
    assert_eq!(txt, deserialized);
}

#[test]
fn score() {
    let txt = Text::score("foo", "bar", Some(Cow::from("baz")));
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    assert_eq!(
        serialized,
        r#"{"score":{"name":"foo","objective":"bar","value":"baz"}}"#
    );
    assert_eq!(txt, deserialized);
}

#[test]
fn selector() {
    let separator = Text::text("bar").color(Color::RED).bold();
    let txt = Text::selector("foo", Some(separator));
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    assert_eq!(
        serialized,
        r#"{"selector":"foo","separator":{"text":"bar","color":"red","bold":true}}"#
    );
    assert_eq!(txt, deserialized);
}

#[test]
fn keybind() {
    let txt = Text::keybind("foo");
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    assert_eq!(serialized, r#"{"keybind":"foo"}"#);
    assert_eq!(txt, deserialized);
}

#[test]
fn block_nbt() {
    let txt = Text::block_nbt("foo", "bar", Some(true), Some("baz".into_text()));
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    let expected = r#"{"block":"foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
    assert_eq!(serialized, expected);
    assert_eq!(txt, deserialized);
}

#[test]
fn entity_nbt() {
    let txt = Text::entity_nbt("foo", "bar", Some(true), Some("baz".into_text()));
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    let expected = r#"{"entity":"foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
    assert_eq!(serialized, expected);
    assert_eq!(txt, deserialized);
}

#[test]
fn storage_nbt() {
    let txt = Text::storage_nbt(ident!("foo"), "bar", Some(true), Some("baz".into_text()));
    let serialized = txt.to_string();
    let deserialized = Text::from_str(&serialized).unwrap();
    let expected =
        r#"{"storage":"minecraft:foo","nbt":"bar","interpret":true,"separator":{"text":"baz"}}"#;
    assert_eq!(serialized, expected);
    assert_eq!(txt, deserialized);
}

#[test]
fn text_to_legacy_lossy() {
    let text = "Heavily formatted green text\n"
        .bold()
        .italic()
        .strikethrough()
        .underlined()
        .obfuscated()
        .color(Color::GREEN)
        + "Lightly formatted red text\n"
            .not_bold()
            .not_strikethrough()
            .not_obfuscated()
            .color(Color::RED)
        + "Not formatted blue text"
            .not_italic()
            .not_underlined()
            .color(Color::BLUE);

    assert_eq!(
        text.to_legacy_lossy(),
        "§a§k§l§m§n§oHeavily formatted green text\n§r§c§n§oLightly formatted red text\n§r§9Not \
         formatted blue text"
    );
}
