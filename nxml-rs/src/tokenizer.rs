use std::fmt::Display;

#[derive(Debug)]
pub enum Token<'s> {
    Eof,
    OpenLess,
    CloseGreater,
    Slash,
    Equal,
    String(&'s str),
}

impl<'s> Token<'s> {
    pub fn as_str(&self) -> &'s str {
        match self {
            Token::Eof => "",
            Token::OpenLess => "<",
            Token::CloseGreater => ">",
            Token::Slash => "/",
            Token::Equal => "=",
            Token::String(s) => s,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug)]
pub struct Tokenizer<'s> {
    data: &'s str,
    current_index: usize,
    position: Position,
}

impl<'s> Tokenizer<'s> {
    pub fn new(data: &str) -> Tokenizer {
        Tokenizer {
            data,
            current_index: 0,
            position: Position { line: 1, column: 1 },
        }
    }

    pub fn position(&self) -> Position {
        self.position
    }

    fn eof(&self) -> bool {
        self.current_index >= self.data.len()
    }

    fn cur(&self) -> char {
        self.data
            .get(self.current_index..)
            .and_then(|s| s.chars().next())
            .unwrap_or_default()
    }

    fn skip(&mut self) {
        let ch = self.cur();
        self.current_index += ch.len_utf8();
        if self.current_index >= self.data.len() {
            self.current_index = self.data.len();
            return;
        }
        if ch == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
    }

    fn peek_string(&self, s: &str) -> bool {
        self.data
            .get(self.current_index..)
            .unwrap_or_default()
            .chars()
            .zip(s.chars())
            .all(|(a, b)| a == b)
    }

    fn take_string(&mut self, s: &str) -> bool {
        let peeked = self.peek_string(s);
        if !peeked {
            return false;
        }
        self.current_index += s.len();
        for ch in s.chars() {
            if ch != '\n' {
                self.position.column += 1;
                continue;
            }
            self.position.line += 1;
            self.position.column = 1;
        }
        true
    }

    fn skip_whitespace(&mut self) {
        while !self.eof() {
            if is_whitespace(self.cur()) {
                self.skip();
                continue;
            }

            macro_rules! skip_delimited {
                ($start:literal, $end:literal) => {
                    if self.take_string($start) {
                        while !self.eof() && !self.take_string($end) {
                            self.skip();
                        }
                        continue;
                    }
                };
            }

            skip_delimited!("<!--", "-->");
            skip_delimited!("<!", ">");
            skip_delimited!("<?", "?>");
            break;
        }
    }

    pub fn take(&mut self, expect: char) -> bool {
        let ch = self.cur();
        if ch == expect {
            self.skip();
            return true;
        }
        false
    }

    pub fn next_token(&mut self) -> Token<'s> {
        self.skip_whitespace();

        if self.eof() {
            return Token::Eof;
        }

        let ch = self.cur();
        self.skip();

        match ch {
            '\0' => Token::Eof,
            '<' => Token::OpenLess,
            '>' => Token::CloseGreater,
            '/' => Token::Slash,
            '=' => Token::Equal,
            '"' => Token::String({
                let start_idx = self.current_index;
                while !self.eof() && !self.take_string("\"") {
                    self.skip();
                }
                // -1 to exclude the closing quote
                // (but avoid a panic if it was EOF)
                &self.data[start_idx..self.current_index - !self.eof() as usize]
            }),
            ch => Token::String({
                // including the consumed ch
                let start_idx = self.current_index - ch.len_utf8();
                while !self.eof() && !is_punctuation_or_whitespace(self.cur()) {
                    self.skip();
                }
                &self.data[start_idx..self.current_index]
            }),
        }
    }
}

#[inline]
fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\n' || c == '\r'
}

#[inline]
fn is_punctuation_or_whitespace(c: char) -> bool {
    is_whitespace(c) || c == '<' || c == '>' || c == '=' || c == '/'
}
