use ezmenulib::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let amount = Selected::new("how many", [("zero", 0), ("one", 1), ("two", 2)])
        .format(Format {
            suffix: "> ",
            line_brk: false,
            show_default: false,
            ..Default::default()
        })
        .default(1)
        .select(&mut MenuStream::default())?;

    Ok(println!("you selected {}", amount))
}
