use args::StresserArgs;
use clap::Parser;

mod args;

fn main() {
    let args = StresserArgs::parse();
}
