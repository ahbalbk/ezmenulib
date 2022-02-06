#[cfg(test)]
#[path = "tests/menu.rs"]
mod menu;

use crate::field::StructFieldFormatting;
use crate::StructField;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{stdin, stdout, BufRead, Stdin, Stdout, Write};
use std::str::FromStr;

/// Represents a menu describing a struct.
///
/// It has a global formatting applied to the fields it contains.
/// The menu uses an R reader and W writer for polymorphism purposes.
/// By default, it uses Stdin and Stdout. For custom reader and writer types,
/// use the `custom_io` feature in your `Cargo.toml`:
/// ```toml
/// [dependencies]
/// ezmenu = { features = ["custom_io"], ... }
/// ```
///
/// # Examples
///
/// For a make-licence CLI program for example, you can build the menu like above:
/// ```
/// use ezmenu::{StructField, StructFieldFormatting};
/// let mut menu = StructMenu::default()
///     .title("-- Mklicense --")
///     .fmt(StructFieldFormatting {
///         chip: "* Give the ",
///         ..Default::default()
///     })
///     .with_field(StructField::from("project author name"))
///     .with_field(StructField::from("project name"))
///     .with_field(StructField::from("Give the year of the license")
///         .default("2022")
///         .fmt(StructFieldFormatting {
///             prefix: ">> ",
///             new_line: true,
///             ..Default::default()
///         })
///     );
///
/// let name: String = menu.next_with(|s: &String, w| {
///     if s.to_lowercase() == "ahmad" {
///         w.write(b"Nice name!!").expect("unable to pat :/");
///     }
/// });
/// let proj_name: String = menu.next();
/// let proj_year: i64 = menu.next();
/// ```
///
/// The example below will display this menu:
/// ```text
/// -- Mklicense --
/// * Give the project author name: ahmad
/// Nice name!!
/// * Give the project name: ezmenu
/// * Give the year of the license (default: 2022)
/// >> 2018
/// ```
pub struct StructMenu<'a, R, W> {
    title: &'a str,
    fmt: StructFieldFormatting<'a>,
    fields: VecDeque<StructField<'a>>,
    reader: R,
    writer: W,
    // used to know when to print the title
    first_popped: bool,
}

/// The default menu uses `Stdin` as reader and `Stdout` as writer.
impl<'a> Default for StructMenu<'a, Stdin, Stdout> {
    fn default() -> Self {
        Self::new(stdin(), stdout())
    }
}

/// The default menu uses `Stdin` as reader and `Stdout` as writer.
impl<'a> From<&'a str> for StructMenu<'a, Stdin, Stdout> {
    fn from(title: &'a str) -> Self {
        Self {
            title,
            ..Default::default()
        }
    }
}

/// Methods used to construct a menu describing a struct.
impl<'a, R, W> StructMenu<'a, R, W> {
    /// Builds a new menu by defining its reader and writer.
    /// The reader must implement `std::io::BufRead`
    /// and the writer must implement `std::io::Write`, at the menu building.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            title: "",
            fmt: Default::default(),
            fields: Default::default(),
            reader,
            writer,
            first_popped: false,
        }
    }

    /// Give the global formatting applied to all the fields the menu contains.
    /// If a field has a custom formatting, it will uses the formatting rules of the field
    /// when printing to the writer.
    pub fn fmt(mut self, fmt: StructFieldFormatting<'a>) -> Self {
        self.fmt = fmt;
        self
    }

    /// Give the main title of the menu.
    /// It is printed at the beginning, right before the first field.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Append a new field to the menu.
    /// You can chain them and they will be printed according to the order
    /// you instantiated them.
    pub fn with_field(mut self, field: StructField<'a>) -> Self {
        self.fields.push_back(if field.custom_fmt {
            field
        } else {
            field.inherit_fmt(self.fmt.clone())
        });
        self
    }
}

impl<'a, R, W: Write> StructMenu<'a, R, W> {
    /// Returns the next field to print when building the menu.
    fn get_next_field(&mut self) -> StructField<'a> {
        // prints the menu title or not
        if !self.first_popped {
            writeln!(self.writer, "{}", self.title).expect("Unable to print title");
            self.first_popped = true;
        }
        self.fields.pop_front().expect("no more field in menu")
    }
}

/// Trait used to return the next output of the menu.
pub trait Menu<Output, R, W>
where
    Output: FromStr,
    <Output as FromStr>::Err: Debug,
{
    /// Returns the next output from the reader.
    fn next(&mut self) -> Output;

    /// Returns the output at the current state.
    fn get_output(&mut self) -> &mut W;

    // TODO: use a result as return type for F
    /// Method used to return the user output after applied a function on it.
    fn next_with<F: FnOnce(&Output, &mut W)>(&mut self, f: F) -> Output {
        let val = self.next();
        f(&val, &mut self.get_output());
        val
    }
}

/// The implementation of the Menu trait using `Stdin` as reader requires
/// to use the `Stdin::lock` method for polymorphism purposes, because we need
/// an `impl BufRead` to read the next line input.
/// In the future versions, this will not be necessary because `Stdin` will implement `BufRead`.
#[cfg(not(any(feature = "custom_io", test)))]
impl<'a, Output> Menu<Output, Stdin, Stdout> for StructMenu<'a, Stdin, Stdout>
where
    Output: FromStr,
    <Output as FromStr>::Err: Debug,
{
    /// Returns the next field output with the correct type.
    ///
    /// # Panics
    ///
    /// This method panics if there is no more value to output.
    fn next(&mut self) -> Output {
        let field = self.get_next_field();
        field.build(&mut self.reader.lock(), &mut self.writer)
    }

    fn get_output(&mut self) -> &mut Stdout {
        &mut self.writer
    }
}

#[cfg(any(feature = "custom_io", test))]
impl<'a, Output, R, W> Menu<Output, R, W> for StructMenu<'a, R, W>
where
    R: BufRead,
    W: Write,
    Output: FromStr,
    <Output as FromStr>::Err: Debug,
{
    /// Returns the next field output with the correct type.
    ///
    /// # Panics
    ///
    /// This method panics if there is no more value to output.
    fn next(&mut self) -> Output {
        let field = self.get_next_field();
        field.build(&mut self.reader, &mut self.writer)
    }

    fn get_output(&mut self) -> &mut W {
        &mut self.writer
    }
}

/// The position of the title for an enum menu.
// TODO: implement enum menu to use this
#[allow(unused)]
pub enum TitlePos {
    /// Position at the top of the menu:
    /// ```md
    /// <title>
    /// 1 - field1
    /// 2 - field2
    /// ...
    /// >>
    /// ```
    Top,
    /// Position at the bottom of the menu:
    /// ```md
    /// 1 - field1
    /// 2 - field2
    /// ...
    /// <title>
    /// >>
    /// ```
    Bottom,
}

/// Default position for the menu title is at the top.
impl Default for TitlePos {
    fn default() -> Self {
        Self::Top
    }
}