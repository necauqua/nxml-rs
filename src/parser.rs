use std::borrow::Cow;

use thiserror::Error;

use crate::{
    element::{Element, ElementRef},
    tokenizer::{Position, Token, Tokenizer},
};

#[derive(Debug, Error)]
pub enum NxmlErr<'s> {
    #[error("No closing '>' found for ending element </{element}>")]
    NoClosingSymbolFound { element: &'s str },
    #[error("Couldn't find a '<' to start parsing with")]
    NoOpeningSymbolFound,
    #[error(
        "Closing element is in wrong order. Expected '</{expected}>', but instead got '{}'", got.as_str()
    )]
    MismatchedClosingTag { expected: &'s str, got: Token<'s> },
    #[error("parsing tag '{tag}', attribute '{attribute}' - expected '='")]
    MissingEqualsSign { tag: &'s str, attribute: &'s str },
    #[error("parsing tag '{tag}', attribute '{attribute}' - expected a \"string\" after =, but none found")]
    MissingAttributeValue { tag: &'s str, attribute: &'s str },
    #[error("Expected a name of the element after <")]
    MissingElementName,
}

#[derive(Debug, Error)]
#[error("{err} [{at}]")]
pub struct NxmlError<'s> {
    pub err: NxmlErr<'s>,
    pub at: Position,
}

impl ElementRef<'_> {
    pub fn parse(s: &str) -> Result<ElementRef, NxmlError> {
        Parser::new(s).parse()
    }

    pub fn parse_lenient(s: &str) -> (ElementRef, Vec<NxmlError>) {
        let mut parser = Parser::new(s).lenient();
        let element = parser.parse().expect("lenient parser never errors");
        (element, parser.errors)
    }
}

impl Element {
    pub fn parse(s: &str) -> Result<Self, NxmlError> {
        ElementRef::parse(s).map(ElementRef::into_owned)
    }
    pub fn parse_lenient(s: &str) -> (Self, Vec<NxmlError>) {
        let (element, errors) = ElementRef::parse_lenient(s);
        (element.into_owned(), errors)
    }
}

#[derive(Debug)]
struct Parser<'s> {
    tokenizer: Tokenizer<'s>,
    errors: Vec<NxmlError<'s>>,
    lenient: bool,
}

impl<'s> Parser<'s> {
    fn new(data: &str) -> Parser {
        Parser {
            tokenizer: Tokenizer::new(data),
            errors: Vec::new(),
            lenient: false,
        }
    }

    fn lenient(mut self) -> Self {
        self.lenient = true;
        self
    }

    fn report(&mut self, err: NxmlErr<'s>) -> Result<(), NxmlError<'s>> {
        let error = NxmlError {
            err,
            at: self.tokenizer.position(),
        };
        if self.lenient {
            self.errors.push(error);
            return Ok(());
        }
        Err(error)
    }

    fn parse(&mut self) -> Result<ElementRef<'s>, NxmlError<'s>> {
        self.parse_inner(false)
    }

    fn parse_inner(&mut self, skip_opening_tag: bool) -> Result<ElementRef<'s>, NxmlError<'s>> {
        if !skip_opening_tag && !matches!(self.tokenizer.next_token(), Token::OpenLess) {
            self.report(NxmlErr::NoOpeningSymbolFound)?;
        }

        let name = match self.tokenizer.next_token() {
            Token::String(name) => name,
            _ => {
                self.report(NxmlErr::MissingElementName)?;
                ""
            }
        };

        let mut element = ElementRef::new(name);

        loop {
            match self.tokenizer.next_token() {
                Token::Eof => return Ok(element),
                Token::Slash => {
                    if self.tokenizer.take('>') {
                        return Ok(element);
                    }
                    break;
                }
                Token::CloseGreater => break,
                Token::String(name) => {
                    let Token::Equal = self.tokenizer.next_token() else {
                        self.report(NxmlErr::MissingEqualsSign {
                            tag: element.name,
                            attribute: name,
                        })?;
                        continue;
                    };

                    let Token::String(value) = self.tokenizer.next_token() else {
                        self.report(NxmlErr::MissingAttributeValue {
                            tag: element.name,
                            attribute: name,
                        })?;
                        continue;
                    };

                    element.attributes.insert(name, value);
                }
                _ => (),
            }
        }
        loop {
            match self.tokenizer.next_token() {
                Token::Eof => return Ok(element),
                Token::OpenLess => (),
                token => {
                    match element.text_content {
                        Cow::Borrowed("") => {
                            element.text_content = Cow::Borrowed(token.as_str());
                        }
                        Cow::Borrowed(content) => {
                            element.text_content =
                                Cow::Owned(content.to_owned() + " " + token.as_str())
                        }
                        Cow::Owned(ref mut s) => s.push_str(token.as_str()),
                    }
                    continue;
                }
            }

            if !self.tokenizer.take('/') {
                element.children.push(self.parse_inner(true)?);
                continue;
            }

            match self.tokenizer.next_token() {
                Token::String(name) if name == element.name => {
                    if let Token::CloseGreater = self.tokenizer.next_token() {
                        return Ok(element);
                    }
                    self.report(NxmlErr::NoClosingSymbolFound { element: name })?;
                }
                token => self.report(NxmlErr::MismatchedClosingTag {
                    expected: element.name,
                    got: token,
                })?,
            };
            return Ok(element);
        }
    }
}
