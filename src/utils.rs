//! This module contains many utils functions used by the library.

use crate::prelude::*;

use std::any::type_name;
use std::fmt::Display;
use std::io::BufRead;
use std::io::Write;

/// Function used by the fields associated functions.
///
/// It is useful because it always returns true no matter the output value,
/// "keeping" the value in the context of the associated functions
/// (see [`Written::prompt`] or [`Written::many_values`] methods).
pub fn keep<T>(_val: &T) -> bool {
    true
}

/// Shows the text using the given stream and maps the `io::Error` into a `MenuError`.
pub fn show<T: ?Sized + Display, S: Write>(text: &T, stream: &mut S) -> MenuResult {
    write!(stream, "{}", text)?;
    stream.flush().map_err(MenuError::from)
}

/// Shows the text using the given stream, then prompts a value to the user and
/// returns the corresponding String.
pub fn prompt<T: ?Sized + Display, R: BufRead, W: Write>(
    text: &T,
    stream: &mut MenuStream<R, W>,
) -> MenuResult<String> {
    show(text, stream)?;
    read_input(stream)
}

/// Panics at runtime, emphasizing that the given `default` value is incorrect for `T` type.
pub fn default_failed<T>(default: &str) -> ! {
    panic!(
        "`{}` has been used as default value but is incorrect for `{}` type",
        default,
        type_name::<T>(),
    )
}

/// Returns the input value as a String from the given input stream.
pub fn read_input<R: BufRead, W>(stream: &mut MenuStream<R, W>) -> MenuResult<String> {
    let mut out = String::new();
    stream.read_line(&mut out)?;
    Ok(out.trim().to_owned())
}