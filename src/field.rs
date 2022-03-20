//! Module defining different types of fields.
//!
//! The selection fields [`SelectField`] corresponds to a [`SelectMenu`](crate::menu::SelectMenu),
//! while the value fields [`ValueField`] corresponds to a [`ValueMenu`](crate::menu::ValueMenu).
//!
//! A `ValueMenu` can however contain both a `ValueField` and a `SelectMenu`,
//! to be used as a sub-menu (check out the [`Field`] enum).
//!
//! ## Formatting
//!
//! You can edit the [formatting rules](ValueFieldFormatting) of a `ValueField` or set the global
//! formatting rules for the `ValueMenu`.
//!
//! If a `SelectMenu` is used as a sub-menu to a `ValueMenu`, the global formatting rules of the `ValueMenu`
//! will be applied on the title of the `SelectMenu` to integrate it in the structure.
//!
//! You can still edit the formatting rules of its title (see [`SelectTitle`](crate::menu::SelectTitle))
//! independently from the global formatting rules.

use crate::prelude::*;
use std::any::Any;
use std::env;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{BufRead, Write};
use std::rc::Rc;
use std::str::FromStr;

/// A field contained in a [`ValueMenu`](crate::menu::ValueMenu) menu.
///
/// A field of a menu returning values can be an asked value ([`ValueField`]),
/// or a menu of selectable values ([`SelectMenu`]).
pub enum Field<'a, R = In, W = Out> {
    /// A field asking a value to the user.
    Value(ValueField<'a>),
    /// A field proposing selectable values to the user.
    Select(SelectMenu<'a, R, W>),
}

impl<'a, R, W> Field<'a, R, W> {
    /// Inherits the formatting rules from a parent menu (the [`ValueMenu`](crate::ValueMenu)).
    ///
    /// If it is aimed on a selectable menu, the formatting rules will be applied on its title,
    /// to integrate it in the value-fields of the parent menu.
    /// The title of the selectable menu will however save its prefix.
    pub(crate) fn inherit_fmt(&mut self, fmt: Rc<ValueFieldFormatting<'a>>) {
        match self {
            Self::Value(vf) => vf.inherit_fmt(fmt),
            Self::Select(sm) => sm.inherit_fmt(fmt),
        }
    }
}

impl<'a, R, W> Field<'a, R, W>
where
    R: BufRead,
    W: Write,
{
    /// Builds the field according to its type.
    ///
    /// That is, if it is a value-field, it will ask the user a value then return it,
    /// while if it is a selectable menu, it will display the menu to the user then return the value
    /// selected.
    ///
    /// See [`ValueField::build`] and [`SelectMenu::next_output`] for more information.
    pub(crate) fn build<Output>(&mut self, stream: &mut MenuStream<'a, R, W>) -> MenuResult<Output>
    where
        Output: FromStr + 'static,
        Output::Err: 'static + Debug,
    {
        match self {
            Self::Value(vf) => vf.build_with(stream),
            Self::Select(sm) => sm.next_output_with(stream),
        }
    }

    pub(crate) fn build_until<Output, F>(
        &mut self,
        stream: &mut MenuStream<'a, R, W>,
        w: F,
    ) -> MenuResult<Output>
    where
        Output: FromStr,
        Output::Err: 'static + Debug,
        F: Fn(&Output) -> bool,
    {
        match self {
            Self::Value(vf) => vf.build_until(stream, w),
            _ => {
                panic!("cannot apply a conditional closure to map the output of a selectable menu")
            }
        }
    }

    pub(crate) fn build_or_default<Output>(&mut self, stream: &mut MenuStream<'a, R, W>) -> Output
    where
        Output: FromStr + Default + 'static,
        Output::Err: 'static + Debug,
    {
        match self {
            Self::Value(vf) => vf.build_or_default_with(stream),
            Self::Select(sm) => sm.next_or_default_with(stream),
        }
    }
}

/// Type used to handle the binding function executed right after
/// the corresponding field has been selected by the user.
pub type Binding<R, W> = fn(&mut MenuStream<R, W>) -> MenuResult<()>;

/// Struct modeling a field of a selective menu.
///
/// Unlike [`ValueField`], this struct should not be used alone, without a context.
/// You must instantiate it in an array in the constructor of the [`SelectMenu`](crate::menu::SelectMenu) struct.
///
/// Just like [`ValueFieldFormatting`], it contains an editable `chip` string slice, placed
/// after the selection index (`X`):
/// ```text
/// X<chip><message>
/// ```
///
/// ## Example
///
/// ```no_run
/// use ezmenulib::prelude::*;
///
/// let get_amount = SelectMenu::from([
///     SelectField::new("one", 1),
///     SelectField::new("two", 2),
/// ]);
/// ```
pub struct SelectField<'a, R = In, W = Out> {
    pub(crate) msg: &'a str,
    chip: &'a str,
    custom_chip: bool,
    bind: Option<Binding<R, W>>,
    inner: Box<dyn Any>,
}

impl<R, W> Display for SelectField<'_, R, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.chip, f)?;
        Display::fmt(self.msg, f)
    }
}

impl<'a, R, W> SelectField<'a, R, W> {
    /// Creates a selection field with its message and its associated output value.
    ///
    /// This value corresponds to the output returned in case the user selected this field.
    pub fn new<T: 'static>(msg: &'a str, inner: T) -> Self {
        Self {
            msg,
            chip: " - ",
            custom_chip: false,
            bind: None,
            inner: Box::new(inner),
        }
    }

    /// Edits the chip of the selection field.
    ///
    /// The default chip is `" - "`. It includes spaces by default, so you can remove them.
    /// It is placed next right to the selection menu index (`X`):
    /// ```text
    /// X<chip><message>
    /// ```
    pub fn chip(mut self, chip: &'a str) -> Self {
        self.chip = chip;
        self.custom_chip = true;
        self
    }

    pub(crate) fn set_chip(&mut self, chip: &'a str) {
        if !self.custom_chip {
            self.chip = chip;
        }
    }

    pub(crate) fn call_bind(&self, stream: &mut MenuStream<R, W>) -> MenuResult<()> {
        if let Some(b) = self.bind {
            b(stream)?;
        }
        Ok(())
    }

    pub(crate) fn select<T: 'static>(self) -> T {
        *self
            .inner
            .downcast()
            .expect("incorrect output type providing")
    }

    /// Defines the function to execute right after the user selected this field.
    ///
    /// It is optional, and is useful if you have many modes in your program with
    /// procedures defined for each mode.
    ///
    /// The function must take a `&mut MenuStream<R, W>` as parameter, with
    /// `R` as the reader type and `W` as the writer type.
    /// The reader and writer are `BufReader<Stdin>` and `Stdout` by default, so if no specific reader
    /// has been specified when instantiating the menu, it is not necessary to specify the generic types
    /// of the `MenuStream`.
    ///
    /// The binding function must return a `MenuResult<()>`.
    /// The result return is used to spread the error to the point you built the menu.
    ///
    /// If you want to return a string (or static string slice) error, you can use `MenuError::from` associated function.
    /// For an IO Error when using `MenuStream` methods, you can map the error type by using `Result::map_err`.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use ezmenulib::prelude::*;
    /// use std::io::{Stdout, Write};
    /// # enum Type {
    /// # MIT,
    /// # BSD,
    /// # }
    ///
    /// fn bsd(e: &mut MenuStream) -> MenuResult<()> {
    ///     e.write_all(b"coucou this is from bsd\n").map_err(MenuError::from)
    /// }
    ///
    /// fn main() {
    ///     let selected = SelectMenu::from([
    ///         SelectField::new("MIT", Type::MIT).bind(|_| Ok(println!("you selected MIT!!!"))),
    ///         SelectField::new("BSD", Type::BSD).bind(bsd),
    ///         // ...
    ///     ]);
    /// }
    /// ```
    ///
    /// For other error, you can simply use the `MenuError::Other` variant and box your custom error type.
    pub fn bind(mut self, bind: Binding<R, W>) -> Self {
        self.bind = Some(bind);
        self
    }
}

/// Defines the formatting of a value-menu field.
///
/// The final text formatting looks literally like above:
/// ```md
/// <chip><message>[ ({[default: <default>]}, [example: <example>])]{\n}<prefix>
/// ```
/// where:
/// - `<...>` means a given string slice
/// - `{...}` means the value inside is displayed or not (boolean)
/// - `[...]` means the value inside is displayed if it is available
#[derive(Clone)]
pub struct ValueFieldFormatting<'a> {
    /// The small string slice displayed before the message acting as a list style attribute
    /// (by default set as `"--> "`).
    pub chip: &'a str,
    /// The small string slice displayed before the user input (by default set as `">> "`).
    pub prefix: &'a str,
    /// Sets the user input with its prefix on a new line or not (by default set as `true`).
    pub new_line: bool,
    /// Display default value or not (by default set as `true`).
    pub default: bool,
}

/// Builds the constructors of the [`ValueFieldFormatting`] struct
/// according to its fields.
macro_rules! impl_constructors {
    ($(
        #[doc = $doc:expr]
        $i:ident: $t:ty
    ),*) => {
        impl<'a> ValueFieldFormatting<'a> {$(
            #[doc = $doc]
            pub fn $i($i: $t) -> Self {
                Self {
                    $i,
                    ..Default::default()
                }
            }
        )*}
    }
}

impl_constructors!(
    /// Sets the chip of the formatting (`--> ` by default).
    chip: &'a str,
    /// Sets the prefix of the formatting (`>> ` by default).
    prefix: &'a str,
    /// Defines if there is a new line between the message and the prefix with the user input (`true` by default).
    new_line: bool,
    /// Defines if it displays the default value or not (`true` by default).
    default: bool
);

/// Default formatting for a field is `"--> "` as a chip and `">> "` as prefix.
///
/// This being, the field is printed like above (text between `[` and `]` is optional
/// depending on default value providing:
/// ```md
/// * <message>[ (default: <default>)]:
/// ```
impl<'a> Default for ValueFieldFormatting<'a> {
    fn default() -> Self {
        Self {
            chip: "--> ",
            prefix: ">> ",
            new_line: true,
            default: true,
        }
    }
}

// should not be accessed from outer module
enum DefaultValue<'a> {
    Value(&'a str),
    Env(String),
}

impl<'a> DefaultValue<'a> {
    pub fn env(var: &'a str) -> MenuResult<Self> {
        Ok(Self::Env(
            env::var(var).map_err(|e| MenuError::EnvVar(var.to_string(), e))?,
        ))
    }
}

impl Display for DefaultValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[inline(never)]
        fn write_str(f: &mut Formatter<'_>, s: impl AsRef<str>) -> fmt::Result {
            write!(f, "default: {}", s.as_ref())
        }

        match self {
            Self::Value(s) => write_str(f, s),
            Self::Env(s) => write_str(f, s),
        }
    }
}

struct FieldDetails<'a> {
    example: Option<&'a str>,
    default: Option<DefaultValue<'a>>,
    show_d: bool,
}

impl<'a> FieldDetails<'a> {
    fn new(show_d: bool) -> Self {
        Self {
            example: None,
            default: None,
            show_d,
        }
    }

    fn set_example(&mut self, example: &'a str) {
        self.example = Some(example);
    }

    fn set_default_value(&mut self, value: &'a str) {
        self.default = Some(DefaultValue::Value(value));
    }

    fn set_default_env(&mut self, env: &'a str) -> MenuResult<()> {
        self.default = Some(DefaultValue::env(env)?);
        Ok(())
    }

    fn set_show_d(&mut self, show_d: bool) {
        self.show_d = show_d;
    }

    #[inline]
    fn try_default<T>(&self) -> T
    where
        T: FromStr + Default,
        T::Err: 'static + Debug,
    {
        self.default.as_ref().map(default_parse).unwrap_or_default()
    }
}

impl Display for FieldDetails<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.example.is_none() && self.default.is_none()
            || self.example.is_none() && !self.show_d
        {
            return Ok(());
        }

        write!(
            f,
            " ({})",
            match (&self.default, self.example) {
                (Some(d), None) if self.show_d => format!("{}", d),
                (Some(d), Some(e)) if self.show_d => format!("example: {}, {}", e, d),
                (None, Some(e)) => format!("example: {}", e),
                (Some(_), Some(e)) => format!("example: {}", e),
                _ => unreachable!(),
            }
        )
    }
}

/// Defines behavior for a value-menu field.
///
/// It manages
/// - its message
/// - its formatting
/// - the return value type
/// - the default value
///
/// # Examples
///
/// For a make-license CLI program for example, you can use
/// ```no_run
/// use ezmenulib::field::ValueField;
/// let author: String = ValueField::from("Give the author of the license")
///     .build_init()
///     .unwrap();
/// ```
pub struct ValueField<'a> {
    msg: &'a str,
    fmt: Rc<ValueFieldFormatting<'a>>,
    custom_fmt: bool,
    details: FieldDetails<'a>,
}

impl<'a> From<&'a str> for ValueField<'a> {
    fn from(msg: &'a str) -> Self {
        let fmt = Rc::<ValueFieldFormatting<'a>>::default();
        let show_d = fmt.default;
        Self {
            msg,
            fmt,
            custom_fmt: false,
            details: FieldDetails::new(show_d),
        }
    }
}

impl Display for ValueField<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{chip}{msg}{det}{nl}{prefix}",
            chip = self.fmt.chip,
            msg = self.msg,
            det = self.details,
            nl = if self.fmt.new_line { "\n" } else { "" },
            prefix = self.fmt.prefix,
        )
    }
}

/// Constructor methods defining how the field behaves
impl<'a> ValueField<'a> {
    /// Give a custom formatting for the field.
    pub fn fmt(mut self, fmt: ValueFieldFormatting<'a>) -> Self {
        self.set_fmt(Rc::new(fmt));
        self.custom_fmt = true;
        self
    }

    pub(crate) fn inherit_fmt(&mut self, fmt: Rc<ValueFieldFormatting<'a>>) {
        if !self.custom_fmt {
            self.set_fmt(fmt);
        }
    }

    fn set_fmt(&mut self, fmt: Rc<ValueFieldFormatting<'a>>) {
        let show_d = fmt.default;
        self.fmt = fmt;
        self.details.set_show_d(show_d);
    }

    /// Give the default value accepted by the field.
    ///
    /// If the value type is incorrect, the [`ValueField::build`] or [`ValueField::build_init`]
    /// methods will panic at runtime.
    ///
    /// The default value and the example (see the [`example`](ValueField::example) method documentation)
    /// will be displayed inside parenthesis according to its formatting (see [`ValueFieldFormatting`]
    /// for more information).
    pub fn default_value(mut self, default: &'a str) -> Self {
        self.details.set_default_value(default);
        self
    }

    /// Give the default value of the field, passing by an environment variable.
    ///
    /// If the provided environment variable is incorrect, it will return an error
    /// (See [`MenuError::EnvVar`] variant).
    ///
    /// If the value type of the variable is incorrect, the [`ValueField::build`] or [`ValueField::build_init`]
    /// method will panic at runtime.
    pub fn default_env(mut self, var: &'a str) -> MenuResult<Self> {
        self.details.set_default_env(var)?;
        Ok(self)
    }

    /// Give an example of correct value for the field.
    ///
    /// Obviously, it is better to give a correct value for the user, but if the value is incorrect,
    /// it will only mislead the user, and unlike the default value providing,
    /// the program will not panic at runtime to emphasize the problem.
    ///
    /// The example will be shown inside parenthesis according to its formatting
    /// (see [`ValueFieldFormatting`] for more information).
    pub fn example(mut self, example: &'a str) -> Self {
        self.details.set_example(example);
        self
    }

    /// Builds the field without specifying standard input and output files.
    ///
    /// It initializes instance of `Stdin` and `Stdout`.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use ezmenulib::prelude::*;
    /// let age: MenuResult<u8> = ValueField::from("How old are you")
    ///     .build_init();
    /// ```
    pub fn build_init<T>(&self) -> MenuResult<T>
    where
        T: FromStr,
        T::Err: 'static + Debug,
    {
        self.build_with(&mut MenuStream::default())
    }

    /// Builds the field. It prints the message according to its formatting,
    /// then returns the corresponding value.
    ///
    /// You need to provide the reader for the input, and the writer for the output.
    /// It returns a MenuResult if there was an IO error, or if the default value type is incorrect.
    ///
    /// # Example
    ///
    /// Supposing you have declared your own `Stdin` and `Stdout` in your program, you can do so:
    /// ```no_run
    /// use std::io::{stdin, stdout, BufReader};
    /// # use ezmenulib::field::ValueField;
    ///
    /// let mut stdin = BufReader::new(stdin());
    /// let mut stdout = stdout();
    ///
    /// let author: String = ValueField::from("Author")
    ///     .build(&mut stdin, &mut stdout)
    ///     .unwrap();
    /// ```
    pub fn build<T>(&self, reader: &mut In, writer: &mut Out) -> MenuResult<T>
    where
        T: FromStr,
        T::Err: 'static + Debug,
    {
        self.build_with(&mut MenuStream::with(reader, writer))
    }

    /// Builds the fields with a given menu stream. It prints out the message to the stream
    /// according to its formatting, then returns the corresponding value.
    ///
    /// You need to instantiate beforehand the `MenuStream` to use this method.
    pub fn build_with<T, R, W>(&self, stream: &mut MenuStream<R, W>) -> MenuResult<T>
    where
        T: FromStr,
        T::Err: 'static + Debug,
        R: BufRead,
        W: Write,
    {
        self.build_until(stream, |_| true)
    }

    /// Builds the field, or returns the default value from the type.
    pub fn build_or_default<T>(&self, reader: &mut In, writer: &mut Out) -> T
    where
        T: FromStr + Default,
        T::Err: 'static + Debug,
    {
        self.build_or_default_with(&mut MenuStream::with(reader, writer))
    }

    /// Builds the field with a given menu stream, or returns the default value from the type.
    pub fn build_or_default_with<T, R, W>(&self, stream: &mut MenuStream<R, W>) -> T
    where
        T: FromStr + Default,
        T::Err: 'static + Debug,
        R: BufRead,
        W: Write,
    {
        match self.build_once(stream) {
            Query::Finished(out) => out,
            _ => self.details.try_default(),
        }
    }

    /// Builds the field with a given menu stream, prompting the user input until the condition
    /// returned by the function is valid.
    ///
    /// The function takes a reference to the returned output provided by the user, and returns
    /// a `bool` to check if the output is correct.
    pub fn build_until<T, R, W, F>(&self, stream: &mut MenuStream<R, W>, until: F) -> MenuResult<T>
    where
        T: FromStr,
        T::Err: 'static + Debug,
        R: BufRead,
        W: Write,
        F: Fn(&T) -> bool,
    {
        // loops while incorrect input
        loop {
            match self.build_once(stream) {
                Query::Finished(out) if until(&out) => break Ok(out),
                Query::Loop => {
                    if let Some(default) = &self.details.default {
                        return Ok(default_parse(default));
                    }
                }
                Query::Err(e) => break Err(e),
                _ => continue,
            }
        }
    }

    fn build_once<T, R, W>(&self, stream: &mut MenuStream<R, W>) -> Query<T>
    where
        T: FromStr,
        T::Err: 'static + Debug,
        R: BufRead,
        W: Write,
    {
        fn show<R, W>(field: &ValueField<'_>, stream: &mut MenuStream<R, W>) -> MenuResult<String>
        where
            R: BufRead,
            W: Write,
        {
            write!(stream, "{}", field)?;
            stream.flush()?;
            raw_read_input(stream)
        }

        show(self, stream).map(parse_value).into()
    }
}

pub(crate) enum Query<T> {
    Finished(T),
    Err(MenuError),
    Loop,
}

impl<T> From<MenuResult<MenuResult<T>>> for Query<T> {
    fn from(res: MenuResult<MenuResult<T>>) -> Self {
        match res {
            Ok(Ok(out)) => Self::Finished(out),
            Ok(Err(_)) => Self::Loop,
            Err(e) => Self::Err(e),
        }
    }
}

impl<T> From<Query<T>> for MenuResult<T> {
    fn from(q: Query<T>) -> Self {
        match q {
            Query::Finished(out) => Ok(out),
            Query::Err(e) => Err(e),
            Query::Loop => Err(MenuError::from("incorrect input")),
        }
    }
}

/// Returns the input value as a String from the standard input stream.
pub(crate) fn raw_read_input<R, W>(stream: &mut MenuStream<R, W>) -> MenuResult<String>
where
    R: BufRead,
    W: Write,
{
    let mut out = String::new();
    stream.read_line(&mut out)?;
    stream.write_all(b"\n")?;
    Ok(out.trim().to_owned())
}

/// Parses the input value.
///
/// It is useful because it maps the error according to the [`MenuError`](crate::MenuError)
/// type definition.
pub(crate) fn parse_value<T, S>(s: S) -> MenuResult<T>
where
    T: FromStr,
    T::Err: 'static + Debug,
    S: AsRef<str>,
{
    let s = s.as_ref();
    s.parse()
        .map_err(|e| MenuError::Parse(s.to_owned(), Box::new(e)))
}

pub(crate) fn default_parse_failed<S, E>(s: S, e: E) -> !
where
    S: ToString,
    E: 'static + Debug,
{
    panic!(
        "`{}` has been used as default value but its type is incorrect: {:?}",
        s.to_string(),
        e
    )
}

/// Function that parses the default value with a check if the default value is incorrect.
/// It it used to return a value if there is some default value,
/// and if no value was provided, or if the value provided is incorrect.
fn default_parse<T>(default: &DefaultValue<'_>) -> T
where
    T: FromStr,
    T::Err: 'static + Debug,
{
    match default {
        DefaultValue::Value(s) => s.parse().unwrap_or_else(|e| default_parse_failed(s, e)),
        DefaultValue::Env(s) => s.parse().unwrap_or_else(|e| default_parse_failed(s, e)),
    }
}
