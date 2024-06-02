use std::{
    borrow::Cow,
    fmt::Display,
    ops::{Shl, Shr},
};

#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::HashMap<K, V>;

/// An XML element.
///
/// This is a borrowed version of [Element], a result of zero-copy parsing.
/// It is useful for reading Noita XML, but if you need to modify the
/// element, you'd want to convert it to [Element] using
/// [into_owned](ElementRef::into_owned).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ElementRef<'s> {
    pub name: &'s str,
    pub attributes: Map<&'s str, &'s str>,
    pub children: Vec<ElementRef<'s>>,
    pub text_content: Cow<'s, str>,
}

impl<'s> ElementRef<'s> {
    pub fn new(name: &'s str) -> Self {
        Self {
            name,
            attributes: Map::new(),
            children: Vec::new(),
            text_content: Cow::Borrowed(""),
        }
    }

    pub fn into_owned(self) -> Element {
        Element {
            name: self.name.to_owned(),
            attributes: self
                .attributes
                .iter()
                .map(|(&k, &v)| (k.to_owned(), v.to_owned()))
                .collect(),
            children: self.children.into_iter().map(|c| c.into_owned()).collect(),
            text_content: self.text_content.into_owned(),
        }
    }
}

/// An owned XML element, easy to create and/or manipulate.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Element {
    pub name: String,
    pub attributes: Map<String, String>,
    pub children: Vec<Element>,
    pub text_content: String,
}

impl Element {
    pub fn new(name: impl ToString) -> Element {
        Element {
            name: name.to_string(),
            attributes: Map::new(),
            children: Vec::new(),
            text_content: String::new(),
        }
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Find the first child element with the given name.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Iterate over all child elements with the given name.
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    pub fn set_attr(&mut self, key: impl ToString, value: impl ToString) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    pub fn remove_attr(&mut self, key: &str) -> Option<String> {
        #[cfg(feature = "indexmap")]
        return self.attributes.shift_remove(key);

        #[cfg(not(feature = "indexmap"))]
        return self.attributes.remove(key);
    }

    pub fn with_attr(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.set_attr(key, value);
        self
    }

    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.text_content = text.to_string();
        self
    }

    pub fn with_child(mut self, element: Element) -> Self {
        self.children.push(element);
        self
    }
}

impl<'e> Shr<&str> for &'e Element {
    type Output = &'e Element;

    fn shr(self, rhs: &str) -> Self::Output {
        match self.child(rhs) {
            Some(child) => child,
            None => panic!("child element '{rhs}' not found"),
        }
    }
}

impl<'e> Shl<&str> for &'e Element {
    type Output = &'e str;

    fn shl(self, rhs: &str) -> Self::Output {
        match self.attr(rhs) {
            Some(attr) => attr,
            None => panic!("attribute '{rhs}' not found"),
        }
    }
}

// Instead of duplicating the Display impl, lets abstract over accessors in 3x
// the code xd
// But the algorith is not duplicated and that's what matters
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

fn write_element<E: ElementAccessor>(
    w: &mut std::fmt::Formatter,
    element: &E,
    indent: usize,
) -> std::fmt::Result {
    write!(w, "{:indent$}<{}", "", element.name())?;

    for (key, value) in element.attributes() {
        write!(w, " {key}=\"{value}\"")?;
    }

    let text_content = element.text_content();
    if element.children().is_empty() && text_content.is_empty() {
        write!(w, "/>")?;
        return Ok(());
    }

    writeln!(w, ">")?;

    if !text_content.is_empty() {
        let indent = indent + 4;
        writeln!(w, "{:indent$}{text_content}", "")?;
    }

    for child in element.children() {
        write_element(w, child, indent + 4)?;
        writeln!(w)?;
    }

    write!(w, "{:indent$}</{}>", "", element.name())
}

impl Display for ElementRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_element(f, self, 0)
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_element(f, self, 0)
    }
}
