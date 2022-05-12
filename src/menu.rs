//! Module that defines several types to handle menus, streams and values retrieving.

#[cfg(test)]
mod tests;

mod stream;

pub use crate::menu::stream::MenuStream;
use crate::menu::stream::Stream;
use crate::prelude::*;
use crate::utils::select;

use std::fmt::{self, Display, Formatter};
use std::io::{BufRead, BufReader, Stdin, Stdout, Write};
use std::ops::{DerefMut, Deref};
use std::str::FromStr;

/// The default input stream used by a menu, using the standard input stream.
pub type In = BufReader<Stdin>;

/// The default output stream used by a menu, using the standard output stream.
pub type Out = Stdout;

/// Used to retrieve the stream from a container.
pub trait Streamable<'a, R: 'a, W: 'a>: Sized {
    /// Returns the ownership of the stream it contains, consuming `self`.
    fn take_stream(self) -> MenuStream<'a, R, W>;

    /// Returns the ownership of the reader and writer, consuming `self`.
    /// 
    /// # Panics
    /// 
    /// If the `Streamable::take_stream` method panics at runtime,
    /// then this method will also panic at runtime.
    fn take_io(self) -> (R, W) {
        self.take_stream().retrieve()
    }

    /// Returns a reference to the stream the container uses.
    fn get_stream(&self) -> &MenuStream<'a, R, W>;

    /// Returns a mutable reference to the stream the container uses.
    fn get_mut_stream(&mut self) -> &mut MenuStream<'a, R, W>;
}

/// Container used to handle the [stream](MenuStream) and the global [format](Format).
///
/// The `R` type parameter represents its reader type,
/// and the `W` type parameter represents its writer type.
/// By default, it uses the standard input and output streams to get values from the user.
/// It wraps the streams into a [`MenuStream`].
///
/// It has a global formatting applied to the fields it gets values from by inheritance.
/// The inheritance saves the custom format specifications of the field.
///
/// # Example
///
/// ```no_run
/// use ezmenulib::prelude::*;
/// let mut menu = Values::from(Format::prefix("->> "));
/// // Inherits the prefix specification on the written field
/// let name: String = menu.written(&Written::from("What is your name")).unwrap();
/// // Uses the custom prefix specification of the selectable field
/// let amount: u8 = menu
///     .selected(
///         Selected::new("Select an amount", [("one", 1), ("two", 2), ("three", 3)])
///         .format(Format::prefix("-- "))
///     )
///     .unwrap();
/// ```
///
/// # Streams
///
/// By default, the container uses the standard input and output stream.
/// You can provide your own stream types, wrapped in a [`MenuStream`], and
/// borrow them to the container, or take the stream by ownership at the end.
///
/// ## Example
///
/// Taking the stream from the container by ownership:
/// ```no_run
/// # use ezmenulib::prelude::*;
/// let mut menu = Values::default();
/// // ...
/// let stream = menu.take_stream();
/// // or:
/// # let mut menu = Values::default();
/// let (reader, writer) = menu.take_io();
/// ```
///
/// Giving a mutable reference to the stream to the container:
/// ```no_run
/// # use ezmenulib::prelude::*;
/// let mut my_stream = MenuStream::default();
/// let mut menu = Values::from(&mut my_stream);
/// // We can also give the ownership:
/// // let mut menu = Values::from(my_stream);
/// ```
pub struct Values<'a, R = In, W = Out> {
    /// The global format of the container.
    pub fmt: Format<'a>,
    stream: Stream<'a, MenuStream<'a, R, W>>,
}

/// Returns the default container, which corresponds to the
/// [default format](Format::default) and the [owned default stream](MenuStream::default).
// Cannot use the derivable implementation of `Default`
// because generic parameters R and W need to implement `Default`.
// Here, we use the `Default` implementation of `MenuStream`, which
// uses `BufReader<Stdin>` as `R` and `Stdout` as `W`.
#[allow(clippy::derivable_impls)]
impl Default for Values<'_> {
    fn default() -> Self {
        Self {
            fmt: Format::default(),
            stream: Stream::default(),
        }
    }
}

/// Creates the container from an owned stream.
///
/// You can still take the stream at the end of the usage, with [`Values::take_stream`].
impl<'a, R, W> From<MenuStream<'a, R, W>> for Values<'a, R, W> {
    fn from(stream: MenuStream<'a, R, W>) -> Self {
        Self::inner_new(Stream::Owned(stream), Format::default())
    }
}

/// Creates the container from a mutably borrowed stream.
///
/// This is useful if you still want to access the given streams while using the
/// container to retrieve values.
impl<'a, R, W> From<&'a mut MenuStream<'a, R, W>> for Values<'a, R, W> {
    fn from(stream: &'a mut MenuStream<'a, R, W>) -> Self {
        Self::inner_new(Stream::Borrowed(stream), Format::default())
    }
}

impl<'a> From<Format<'a>> for Values<'a> {
    fn from(fmt: Format<'a>) -> Self {
        Self::inner_new(Stream::default(), fmt)
    }
}

impl<'a, R, W> Values<'a, R, W> {
    fn inner_new(stream: Stream<'a, MenuStream<'a, R, W>>, fmt: Format<'a>) -> Self {
        Self { fmt, stream }
    }

    /// Defines the global formatting applied to all the fields the menu retrieves the values from.
    ///
    /// If the field contains custom formatting specifications, it will save them
    /// when printing to the writer.
    pub fn format(mut self, fmt: Format<'a>) -> Self {
        self.fmt = fmt;
        self
    }
}

impl<'a, R, W> Streamable<'a, R, W> for Values<'a, R, W> {
    /// Returns the ownership of the stream it contains, consuming `self`.
    ///
    /// # Panics
    ///
    /// If the container does not own the stream (meaning it has been constructed
    /// with the `From<&mut MenuStream<R, W>>` implementation), this function panics.
    fn take_stream(self) -> MenuStream<'a, R, W> {
        self.stream.retrieve()
    }

    fn get_stream(&self) -> &MenuStream<'a, R, W> {
        self.stream.deref()
    }

    fn get_mut_stream(&mut self) -> &mut MenuStream<'a, R, W> {
        self.stream.deref_mut()
    }
}

/// Associated functions that concerns retrieving values from the user,
/// thus using the reader and writer stream.
impl<R, W> Values<'_, R, W>
where
    R: BufRead,
    W: Write,
{
    /// Returns the next value selected by the user.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the selectable fields.
    ///
    /// See [`Selected::select`] function fore more information.
    pub fn selected<T, const N: usize>(&mut self, sel: Selected<'_, T, N>) -> MenuResult<T> {
        let fmt = sel.fmt.merged(&self.fmt);
        sel.format(fmt).select(self.stream.deref_mut())
    }

    /// Returns the next value selected by the user wrapped as `Some(value)`,
    /// else `None`.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the selectable fields.
    ///
    /// See [`Selected::optional_select`] function fore more information.
    pub fn optional_selected<T, const N: usize>(
        &mut self,
        sel: Selected<'_, T, N>,
    ) -> MenuResult<Option<T>> {
        let fmt = sel.fmt.merged(&self.fmt);
        sel.format(fmt).optional_select(self.stream.deref_mut())
    }

    /// Returns the next value selected by the user, or the default value of the output type
    /// if any error occurred.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the selectable fields.
    ///
    /// See [`Selected::select_or_default`] function for more information.
    pub fn selected_or_default<T, const N: usize>(&mut self, sel: Selected<'_, T, N>) -> T
    where
        T: Default,
    {
        let fmt = self.fmt.merged(&self.fmt);
        sel.format(fmt).select_or_default(self.stream.deref_mut())
    }

    /// Returns the next value written by the user.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::prompt`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn written<T>(&mut self, written: &Written<'_>) -> MenuResult<T>
    where
        T: FromStr,
    {
        written.prompt_with(self.stream.deref_mut(), &self.fmt)
    }

    /// Returns the next value written by the user by prompting him the field
    /// until the given constraint is applied.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::prompt_until`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn written_until<T, F>(&mut self, written: &Written<'_>, til: F) -> MenuResult<T>
    where
        T: FromStr,
        F: Fn(&T) -> bool,
    {
        written.prompt_until_with(self.stream.deref_mut(), til, &self.fmt)
    }

    /// Returns the next value written by the user wrapped as `Some(value)`
    /// if the input is correct, else `None`.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::optional_value`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn optional_written<T>(&mut self, written: &Written<'_>) -> MenuResult<Option<T>>
    where
        T: FromStr,
    {
        written.optional_value_with(self.stream.deref_mut(), &self.fmt)
    }

    /// Returns the next many values written by the user wrapped as a `Vec<T>`, separated by
    /// `sep`, until the given constraint is applied to all the values.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::many_values_until`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn many_written_until<T, S, F>(
        &mut self,
        written: &Written<'_>,
        sep: S,
        til: F,
    ) -> MenuResult<Vec<T>>
    where
        T: FromStr,
        S: AsRef<str>,
        F: Fn(&T) -> bool,
    {
        written.many_values_until_with(self.stream.deref_mut(), sep, til, &self.fmt)
    }

    /// Returns the next many values written by the user wrapped as a `Vec<T>`,
    /// separated by `sep`.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::many_values`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn many_written<T, S>(&mut self, written: &Written<'_>, sep: S) -> MenuResult<Vec<T>>
    where
        T: FromStr,
        S: AsRef<str>,
    {
        written.many_values_with(self.stream.deref_mut(), sep, &self.fmt)
    }

    /// Returns the next value written by the user, or the default value of the
    /// output type if any error occurred.
    ///
    /// It merges the [format](Format) of the field with the global format of the container.
    /// The merge saves the custom formatting specification of the written field.
    ///
    /// See [`Written::prompt_or_default`] for more information.
    ///
    /// # Panic
    ///
    /// If the given written field has an incorrect default value,
    /// this function will panic at runtime.
    pub fn written_or_default<T>(&mut self, written: &Written<'_>) -> T
    where
        T: FromStr + Default,
    {
        written.prompt_or_default_with(self.stream.deref_mut(), &self.fmt)
    }
}

pub struct Menu<'a, R = In, W = Out> {
    /// The global format of the menu.
    pub fmt: Format<'a>,
    title: Option<&'a str>,
    fields: Fields<'a, R, W>,
    stream: Stream<'a, MenuStream<'a, R, W>>,
}

impl<'a, R, W> Streamable<'a, R, W> for Menu<'a, R, W> {
    /// Returns the ownership of the stream the menu contains, consuming `self`.
    ///
    /// # Panics
    ///
    /// If the menu does not own the stream (meaning it has been constructed
    /// with the `From<&mut MenuStream<R, W>>` implementation), this function panics.
    fn take_stream(self) -> MenuStream<'a, R, W> {
        self.stream.retrieve()
    }

    fn get_stream(&self) -> &MenuStream<'a, R, W> {
        self.stream.deref()
    }

    fn get_mut_stream(&mut self) -> &mut MenuStream<'a, R, W> {
        self.stream.deref_mut()
    }
}

impl<R, W> Display for Menu<'_, R, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Title
        if let Some(title) = self.title {
            f.write_str(title)?;
        }

        // Fields
        // The chip representation is managed by the field itself.
        for (i, field) in self.fields.iter().enumerate() {
            write!(f, "{}", i + 1)?;
            f.write_str(self.fmt.chip)?;
            f.write_str(field.0)?;
            f.write_str("\n")?;
        }

        Ok(())
    }
}

impl<'a> From<Fields<'a>> for Menu<'a> {
    fn from(fields: Fields<'a>) -> Self {
        Self::new_owned(MenuStream::default(), fields)
    }
}

impl<'a, const N: usize> From<SizedFields<'a, N>> for Menu<'a> {
    fn from(fields: SizedFields<'a, N>) -> Self {
        Self::from(fields.as_ref())
    }
}

impl<'a, R, W> Menu<'a, R, W> {
    fn inner_new(stream: Stream<'a, MenuStream<'a, R, W>>, fields: Fields<'a, R, W>) -> Self {
        Self {
            title: None,
            fields,
            stream,
            fmt: Format::default(),
        }
    }

    pub fn new_owned(stream: MenuStream<'a, R, W>, fields: Fields<'a, R, W>) -> Self {
        Self::inner_new(Stream::Owned(stream), fields)
    }

    pub fn new_borrowed(stream: &'a mut MenuStream<'a, R, W>, fields: Fields<'a, R, W>) -> Self {
        Self::inner_new(Stream::Borrowed(stream), fields)
    }

    pub fn format(mut self, fmt: Format<'a>) -> Self {
        self.fmt = fmt;
        self
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl<'a, R, W> Menu<'a, R, W>
where
    R: BufRead,
    W: Write,
{
    pub fn run(&mut self) -> MenuResult {
        run_with(self.title, self.stream.deref_mut(), self.fields, &self.fmt).map(|_| ())
    }
}

enum Res {
    Finished,
    Back(usize),
}

fn run_with<R: BufRead, W: Write>(
    msg: Option<&str>,
    stream: &mut MenuStream<R, W>,
    fields: Fields<R, W>,
    fmt: &Format<'_>,
) -> MenuResult<Res> {
    loop {
        // Title of current selective menu.
        if let Some(s) = msg {
            writeln!(stream, "{}", s)?;
        }

        // Fields of current selective menu.
        for (i, (field_msg, _)) in fields.iter().enumerate() {
            writeln!(stream, "{}{}{}", i + 1, fmt.chip, field_msg)?;
        }

        let (msg, kind) = loop {
            match select(stream, fmt.suffix, None, fields.len())?.and_then(|i| fields.get(i)) {
                Some(field) => break field,
                None => continue,
            }
        };

        match kind {
            Kind::Unit(f) => return f(stream).map(|_| Res::Finished),
            Kind::Parent(fields) => match run_with(Some(msg), stream, fields, fmt)? {
                Res::Finished => return Ok(Res::Finished),
                Res::Back(0) => continue,
                Res::Back(i) => return Ok(Res::Back(i - 1)),
            },
            Kind::Back(0) => continue,
            Kind::Back(i) => return Ok(Res::Back(i - 1)),
            Kind::Quit => return Ok(Res::Finished),
        }
    }
}
