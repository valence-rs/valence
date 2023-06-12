# valence_command

Command API for minecraft with supported brigadier arguments.

## Raw usage

To use command API 'raw', you need to know `Parse` trait.

```rust
// Anything what can be &str
let input = "gamemode @s creative";

// StrReader is needed for Parse trait to work with.
let mut reader = StrReader::new(input);

// Here we are using Parse trait. It returns an ParsingResult (it is not a standard result)
// It has 2 fields. suggestions and result
// Result contains error or an object.
// Suggestions contains suggestions.
let gamemode_literal_result = <&str>::parse(None, reader, ParsingPurpose::Reading);

if let Err((err_pos, err)) = gamemode_literal_result.result {
    // Handle error

    // Component of error (usually this need to be sent to an user)
    let text = err.build();

}

// str itself
let gamemode_literal = gamemode_literal_result.result.unwrap();

// to parse next argument you would need to skip a whitespace, because each Parse parse only its own chars.
// for now only "gamemode" parsed

if !reader.skip_char(' ') {
    // next char is not whitespace otherwise it is skipped and returned true.
}

// continue parsing.
```