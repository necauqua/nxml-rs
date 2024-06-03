use std::{
    borrow::Cow,
    fmt::{self, Display},
    ops::{Div, Rem},
};

#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::HashMap<K, V>;

/// An XML element.
///
/// This is a result of zero-copy parsing, meaning you might run into lifetime
/// issues.
///
/// If you need to own the element separately from the source XML, you can
/// convert it to [`Element`] using
/// [`into_owned`](struct.ElementRef.html#method.into_owned).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ElementRef<'s> {
    /// The name of the element, e.g. `LuaComponent` in `<LuaComponent />`.
    pub name: &'s str,
    /// The text content of the element, e.g. `hello` in
    /// `<SomeComponent>hello</SomeComponent>`.
    ///
    /// If there are multiple text nodes, they are concatenated into a single
    /// string with spaces between them. This is the only case where the
    /// parsing is not zero-copy, as the text is discontinuous in the source
    /// XML.
    ///
    /// If there is no text content, the value is `Cow::Borrowed("")`.
    pub text_content: Cow<'s, str>,
    /// A map of element attributes, e.g. `name="comp"` in `<SomeComponent
    /// name="comp" />`, where the key is `name` and the value is `comp`.
    pub attributes: Map<&'s str, &'s str>,
    /// A list of child elements, e.g. [`<SomeComponent/>`,
    /// `<SomeOtherComponent/>`] in
    /// ```xml
    /// <Entity>
    ///     <SomeComponent/>
    ///     <SomeOtherComponent/>
    /// </Entity>
    /// ```
    pub children: Vec<ElementRef<'s>>,
}

impl<'s> ElementRef<'s> {
    /// Create a new element with the given name.
    pub fn new(name: &'s str) -> Self {
        Self {
            name,
            attributes: Map::new(),
            children: Vec::new(),
            text_content: Cow::Borrowed(""),
        }
    }

    /// Convert this element to an owned [`Element`] by cloning all the strings.
    ///
    /// # Example
    /// ```rust
    /// # use nxml_rs::*;
    /// # fn assert_static<T: 'static>(_: T) {}
    /// let nonstatic_prop = String::from("value");
    /// let element = nxml_ref!(<Entity {&nonstatic_prop} />);
    ///
    /// let owned_element = element.to_owned();
    ///
    /// assert_static(owned_element);
    /// ```
    pub fn to_owned(&self) -> Element {
        Element {
            name: self.name.to_owned(),
            attributes: self
                .attributes
                .iter()
                .map(|(&k, &v)| (k.to_owned(), v.to_owned()))
                .collect(),
            children: self.children.iter().map(|c| c.to_owned()).collect(),
            text_content: self.text_content.clone().into_owned(),
        }
    }
}

/// An owned XML element. Slightly easier to work with than [`ElementRef`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Element {
    /// The name of the element, e.g. `LuaComponent` in `<LuaComponent />`.
    pub name: String,
    /// The text content of the element, e.g. `hello` in
    /// `<SomeComponent>hello</SomeComponent>`.
    ///
    /// If there are multiple text nodes, they are concatenated into a single
    /// string with spaces between them.
    pub text_content: String,
    /// A map of element attributes, e.g. `name="comp"` in `<SomeComponent
    /// name="comp" />`, where the key is `name` and the value is `comp`.
    pub attributes: Map<String, String>,
    /// A list of child elements, e.g. [`<SomeComponent/>`,
    /// `<SomeOtherComponent/>`] in
    /// ```xml
    /// <Entity>
    ///     <SomeComponent/>
    ///     <SomeOtherComponent/>
    /// </Entity>
    /// ```
    pub children: Vec<Element>,
}

impl Element {
    /// Create a new element with the given name.
    pub fn new(name: impl ToString) -> Element {
        Element {
            name: name.to_string(),
            attributes: Map::new(),
            children: Vec::new(),
            text_content: String::new(),
        }
    }

    /// Create an [`ElementRef`] view of this element.
    /// # Example
    /// ```rust
    /// # use nxml_rs::*;
    /// let element = nxml!(<root><thing/><thing/></root>);
    ///
    /// let element_ref: ElementRef = element.as_ref();
    ///
    /// assert_eq!(element_ref.to_string(), "<root><thing/><thing/></root>");
    /// ```
    pub fn as_ref(&self) -> ElementRef {
        ElementRef {
            name: &self.name,
            attributes: self
                .attributes
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
            children: self.children.iter().map(|c| c.as_ref()).collect(),
            text_content: Cow::Borrowed(&self.text_content),
        }
    }
}

/// A text extractor, part of the DSL.
///
/// Only really needed to avoid writing &(&element / "child").text_content, to
/// be able to write `&element / "child" % Text` instead.
#[derive(Debug)]
pub struct Text;

macro_rules! dsl_impls {
    ($(
        #[$macro:ident]
        impl $tpe:ident$(<$src:lifetime>)? {
            attr($attr_str:ty) -> $attr_str_owned:ty,
            text($text_str:ty)$(.$text_transform:ident())?,
        }
    )*) => {
        $(
            impl$(<$src>)? $tpe$(<$src>)? {

                /// A shorthand for getting an attribute value.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity key=\"value\"/>);")]
                ///
                /// assert_eq!(element.attr("key"), Some("value"));
                /// ```
                pub fn attr(&self, key: &str) -> Option<&str> {
                    self.attributes.get(key).map(|s| s.as_ref())
                }

                /// Find the first child element with the given name.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity><Child>\"hello\"</Child></Entity>);")]
                ///
                /// assert_eq!(element.child("Child").unwrap().text_content, "hello");
                /// ```
                pub fn child(&self, name: &str) -> Option<&Self> {
                    self.children.iter().find(|c| c.name == name)
                }

                /// Find the first child element with the given name, mutable version.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let mut element = ", stringify!($macro),"!(<Entity><Child/></Entity>);")]
                ///
                /// element.child_mut("Child").unwrap().text_content = "world".into();
                ///
                /// assert_eq!(element.child("Child").unwrap().text_content, "world");
                pub fn child_mut(&mut self, name: &str) -> Option<&mut Self> {
                    self.children.iter_mut().find(|c| c.name == name)
                }

                /// Iterate over all child elements with the given name.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity><Child/><Other/><Child/></Entity>);")]
                ///
                /// assert_eq!(element.children("Child").count(), 2);
                /// ```
                pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Self> + 'a {
                    self.children.iter().filter(move |c| c.name == name)
                }

                /// Iterate over all child elements with the given name, mutable version.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let mut element = ", stringify!($macro),"!(<Entity><Child/><Other/><Child/></Entity>);")]
                ///
                /// for child in element.children_mut("Child") {
                ///    child.text_content = "text".into();
                /// }
                ///
                /// assert_eq!(element.to_string(), "<Entity><Child>text</Child><Other/><Child>text</Child></Entity>");
                /// ```
                pub fn children_mut<'a>(
                    &'a mut self,
                    name: &'a str,
                ) -> impl Iterator<Item = &'a mut Self> + 'a {
                    self.children.iter_mut().filter(move |c| c.name == name)
                }

                /// A shorthand for setting an attribute value.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let mut element = ", stringify!($macro),"!(<Entity />);")]
                ///
                /// element.set_attr("key", "value");
                ///
                /// assert_eq!(element.to_string(), "<Entity key=\"value\"/>");
                pub fn set_attr(&mut self, key: $attr_str, value: $attr_str) {
                    self.attributes.insert(key$(.$text_transform())?, value$(.$text_transform())?);
                }

                /// A shorthand for removing an attribute value.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let mut element = ", stringify!($macro),"!(<Entity key=\"value\" other=\"other\" />);")]
                ///
                /// element.remove_attr("key");
                ///
                /// assert_eq!(element.to_string(), "<Entity other=\"other\"/>");
                pub fn remove_attr(&mut self, key: &str) -> Option<$attr_str_owned> {
                    #[cfg(feature = "indexmap")]
                    return self.attributes.shift_remove(key);

                    #[cfg(not(feature = "indexmap"))]
                    return self.attributes.remove(key);
                }

                /// Chained version of [`set_attr`](#method.set_attr).
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($tpe), "::new(\"Entity\")")]
                ///     .with_attr("key", "value");
                ///
                /// assert_eq!(element.to_string(), "<Entity key=\"value\"/>");
                /// ```
                pub fn with_attr(mut self, key: $attr_str, value: $attr_str) -> Self {
                    self.set_attr(key, value);
                    self
                }

                /// Chained shorthand for setting the text content.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($tpe), "::new(\"Entity\")")]
                ///     .with_text("hello");
                ///
                /// assert_eq!(element.to_string(), "<Entity>hello</Entity>");
                /// ```
                pub fn with_text(mut self, text: $text_str) -> Self {
                    self.text_content = text$(.$text_transform())?;
                    self
                }

                /// Chained shorthand for adding a child element.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($tpe), "::new(\"Entity\")")]
                #[doc = concat!("     .with_child(", stringify!($tpe), "::new(\"Child\"));")]
                ///
                /// assert_eq!(element.to_string(), "<Entity><Child/></Entity>");
                /// ```
                pub fn with_child(mut self, element: Self) -> Self {
                    self.children.push(element);
                    self
                }

                /// A customizable [`Display`] impl that pretty-prints the element.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity><Child/></Entity>);")]
                ///
                /// assert_eq!(element.display().indent_width(0).to_string(), "<Entity>\n<Child/>\n</Entity>");
                /// ```
                pub fn display(&self) -> PrettyDisplay<'_, Self> {
                    PrettyDisplay {
                        element: self,
                        indent_width: 4,
                        line_separator: "\n",
                        autoclose: true,
                    }
                }
            }

            impl<$($src,)? 'e> Div<&str> for &'e $tpe$(<$src>)? {
                type Output = Self;

                /// A chainable child element accessor
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity><Child><Grandchild>\"hello\"</Grandchild></Child></Entity>);")]
                ///
                /// assert_eq!(&element / "Child" / "Grandchild" % Text, "hello");
                /// ```
                fn div(self, rhs: &str) -> Self::Output {
                    match self.child(rhs) {
                        Some(child) => child,
                        None => panic!("child element '{rhs}' not found"),
                    }
                }
            }

            impl<$($src,)? 'e> Div<&str> for &'e mut $tpe$(<$src>)? {
                type Output = Self;

                /// A mutable version of the child accessor.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let mut element = ", stringify!($macro),"!(<Entity><Child><Grandchild>hello</Grandchild></Child></Entity>);")]
                ///
                /// (&mut element / "Child").children.clear();
                ///
                /// assert_eq!(element.to_string(), "<Entity><Child/></Entity>");
                fn div(self, rhs: &str) -> Self::Output {
                    match self.child_mut(rhs) {
                        Some(child) => child,
                        None => panic!("child element '{rhs}' not found"),
                    }
                }
            }

            impl<$($src,)? 'e> Rem<&str> for &'e $tpe$(<$src>)? {
                type Output = &'e str;

                /// A shorthand for getting an attribute value.
                /// Not index because  `&element / "child" % "key"` is better
                /// than `&(&element / "child")["key"]`.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity key=\"value\"/>);")]
                ///
                /// assert_eq!(&element % "key", "value");
                fn rem(self, rhs: &str) -> Self::Output {
                    match self.attr(rhs) {
                        Some(attr) => attr,
                        None => panic!("attribute '{rhs}' not found"),
                    }
                }
            }

            impl<$($src,)? 'e> Rem<Text> for &'e $tpe$(<$src>)? {
                type Output = &'e str;

                /// A shorthand for getting the text content.
                /// # Example
                /// ```rust
                /// # use nxml_rs::*;
                #[doc = concat!("let element = ", stringify!($macro),"!(<Entity>\"hello\"</Entity>);")]
                ///
                /// assert_eq!(&element % Text, "hello");
                /// ```
                fn rem(self, _: Text) -> Self::Output {
                    &self.text_content
                }
            }

            impl<$($src)?> Display for $tpe$(<$src>)? {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    self.display().compact().fmt(f)
                }
            }
        )*
    };
}

dsl_impls! {
    #[nxml_ref]
    impl ElementRef<'s> {
        attr(&'s str) -> &'s str,
        text(&'s str).into(),
    }

    #[nxml]
    impl Element {
        attr(impl ToString) -> String,
        text(impl ToString).to_string(),
    }
}

// Instead of duplicating the Display impl, lets abstract over accessors in 3x
// the code xd
// But the algorith is not duplicated, so discrepancies are not possible
trait ElementAccessor: Sized {
    fn name(&self) -> &str;
    fn attributes(&self) -> impl Iterator<Item = (&str, &str)>;
    fn children(&self) -> &[Self];
    fn text_content(&self) -> &str;
}

impl ElementAccessor for ElementRef<'_> {
    fn name(&self) -> &str {
        self.name
    }
    fn attributes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attributes.iter().map(|(k, v)| (*k, *v))
    }
    fn children(&self) -> &[Self] {
        &self.children
    }
    fn text_content(&self) -> &str {
        &self.text_content
    }
}

impl ElementAccessor for Element {
    fn name(&self) -> &str {
        &self.name
    }
    fn attributes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }
    fn children(&self) -> &[Self] {
        &self.children
    }
    fn text_content(&self) -> &str {
        &self.text_content
    }
}

/// A pretty-printer for XML elements.
#[derive(Debug)]
pub struct PrettyDisplay<'a, E> {
    element: &'a E,
    indent_width: usize,
    line_separator: &'a str,
    autoclose: bool,
}

impl<'a, E> PrettyDisplay<'a, E> {
    /// Set the indentation width.
    pub fn indent_width(mut self, indent_width: usize) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line separator. Usually it's either `"\n"` or `""`.
    pub fn line_separator(mut self, line_separator: &'a str) -> Self {
        self.line_separator = line_separator;
        self
    }

    /// A shorthand for `.line_separator("").indent_width(0)`.
    pub fn compact(mut self) -> Self {
        self.line_separator = "";
        self.indent_width = 0;
        self
    }

    /// Disable the `/>` syntax.
    /// # Example
    /// ```rust
    /// # use nxml_rs::*;
    /// let element = nxml!(<Entity><Child/></Entity>);
    ///
    /// assert_eq!(element.display().compact().no_autoclose().to_string(), "<Entity><Child></Child></Entity>");
    pub fn no_autoclose(mut self) -> Self {
        self.autoclose = false;
        self
    }

    fn write(&self, w: &mut fmt::Formatter, element: &E, indent: usize) -> fmt::Result
    where
        E: ElementAccessor,
    {
        write!(w, "{:indent$}<{}", "", element.name())?;

        for (key, value) in element.attributes() {
            write!(w, " {key}=\"{value}\"")?;
        }

        let text_content = element.text_content();
        if element.children().is_empty() && text_content.is_empty() {
            if self.autoclose {
                write!(w, "/>")?;
            } else {
                write!(w, "></{}>", element.name())?;
            }
            return Ok(());
        }

        write!(w, ">{}", self.line_separator)?;

        if !text_content.is_empty() {
            let indent = indent + self.indent_width;
            write!(w, "{:indent$}{text_content}{}", "", self.line_separator)?;
        }

        for child in element.children() {
            self.write(w, child, indent + self.indent_width)?;
            write!(w, "{}", self.line_separator)?;
        }

        write!(w, "{:indent$}</{}>", "", element.name())
    }
}

impl<'a, E: ElementAccessor> Display for PrettyDisplay<'a, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write(f, self.element, 0)
    }
}
