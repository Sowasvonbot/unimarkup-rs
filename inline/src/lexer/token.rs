use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::Symbol;

use super::Content;

pub(crate) struct Invalid;
pub(crate) struct Valid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenBuilder<K = Invalid, S = Invalid, W = Invalid> {
    kind: TokenKind,
    span: Span,
    spacing: Spacing,
    content: Option<String>,
    _validation: (PhantomData<K>, PhantomData<S>, PhantomData<W>),
}

impl TokenBuilder<Invalid, Invalid, Invalid> {
    pub fn new(kind: TokenKind) -> TokenBuilder<Valid, Invalid, Invalid> {
        let v1: PhantomData<Valid> = PhantomData;
        let v2: PhantomData<Invalid> = PhantomData;
        let v3: PhantomData<Invalid> = PhantomData;

        TokenBuilder {
            kind,
            span: Span::default(),
            spacing: Spacing::None,
            content: None,
            _validation: (v1, v2, v3),
        }
    }
}

impl<K, S, W> TokenBuilder<K, S, W> {
    pub fn with_content(mut self, content: String) -> TokenBuilder<K, S, W> {
        self.content = Some(content);
        self
    }

    pub fn optional_content(
        self,
        content: &[&str],
        content_option: Content,
    ) -> TokenBuilder<K, S, W> {
        match content_option {
            Content::Store => self.with_content(content.concat()),
            _ => self,
        }
    }

    pub fn span(self, span: Span) -> TokenBuilder<K, Valid, W> {
        let span_valid: PhantomData<Valid> = PhantomData;

        TokenBuilder {
            kind: self.kind,
            span,
            spacing: self.spacing,
            content: self.content,
            _validation: (self._validation.0, span_valid, self._validation.2),
        }
    }

    pub fn space(self, spacing: Spacing) -> TokenBuilder<K, S, Valid> {
        let spacing_valid: PhantomData<Valid> = PhantomData;

        TokenBuilder {
            kind: self.kind,
            span: self.span,
            spacing,
            content: self.content,
            _validation: (self._validation.0, self._validation.1, spacing_valid),
        }
    }
}

impl TokenBuilder<Valid, Valid, Valid> {
    pub fn build(self) -> Token {
        Token {
            kind: self.kind,
            span: self.span,
            spacing: self.spacing,
            content: self.content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    span: Span,
    spacing: Spacing,
    content: Option<String>,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, spacing: Spacing) -> Self {
        Self {
            kind,
            span,
            spacing,
            content: None,
        }
    }

    pub fn with_content(
        kind: TokenKind,
        span: Span,
        spacing: Spacing,
        content: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            span,
            spacing,
            content: Some(content.into()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self.content {
            Some(ref content) => content,
            None => self.kind.as_str(),
        }
    }

    pub fn into_inner(self) -> (String, Span) {
        let content = if let Some(text) = self.content {
            text
        } else {
            String::default()
        };

        (content, self.span)
    }

    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn spacing(&self) -> Spacing {
        self.spacing
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_plain(self) -> Self {
        Self {
            kind: TokenKind::Plain,
            ..self
        }
    }

    pub fn is_nesting_token(&self) -> bool {
        !matches!(
            self.kind,
            TokenKind::Plain | TokenKind::Newline | TokenKind::Whitespace
        )
    }

    pub fn opens(&self) -> bool {
        if self.kind().is_open_parentheses() {
            true
        } else {
            let not_followed_by_whitespace = matches!(self.spacing, Spacing::Pre | Spacing::None);

            self.is_nesting_token() && not_followed_by_whitespace
        }
    }

    pub fn closes(&self) -> bool {
        if self.kind().is_close_parentheses() {
            true
        } else {
            let not_preceded_by_whitespace = matches!(self.spacing, Spacing::Post | Spacing::None);

            self.is_nesting_token() && not_preceded_by_whitespace
        }
    }

    pub fn is_or_contains(&self, other: &Self) -> bool {
        if self.kind() == other.kind() {
            true
        } else {
            match self.kind() {
                TokenKind::ItalicBold => {
                    matches!(other.kind(), TokenKind::Bold | TokenKind::Italic)
                }
                TokenKind::UnderlineSubscript => {
                    matches!(other.kind(), TokenKind::Underline | TokenKind::Subscript)
                }
                _ => false,
            }
        }
    }

    pub fn matches_pair(&self, other: &Self) -> bool {
        match self.kind() {
            TokenKind::OpenParens => matches!(other.kind(), TokenKind::CloseParens),
            TokenKind::CloseParens => matches!(other.kind(), TokenKind::OpenParens),
            TokenKind::OpenBracket => matches!(other.kind(), TokenKind::CloseBracket),
            TokenKind::CloseBracket => matches!(other.kind(), TokenKind::OpenBracket),
            TokenKind::OpenBrace => matches!(other.kind(), TokenKind::CloseBrace),
            TokenKind::CloseBrace => matches!(other.kind(), TokenKind::OpenBrace),
            _ => false,
        }
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(
            self.kind(),
            TokenKind::ItalicBold | TokenKind::UnderlineSubscript
        )
    }

    pub fn remove_partial(&mut self, other_token: &Token) -> Self {
        let panic_message = "Can't remove partial token, tokens are not overlapping.";

        match self.kind() {
            TokenKind::ItalicBold => match other_token.kind() {
                TokenKind::Italic => self.kind = TokenKind::Bold,
                TokenKind::Bold => self.kind = TokenKind::Italic,
                _ => panic!("{panic_message}"),
            },
            TokenKind::UnderlineSubscript => match other_token.kind() {
                TokenKind::Underline => self.kind = TokenKind::Subscript,
                TokenKind::Subscript => self.kind = TokenKind::Underline,
                _ => panic!("{panic_message}"),
            },
            _ => panic!("{panic_message}"),
        };

        let start = self.span().start();
        let end = self.span().end() - other_token.span().start();

        self.span = Span::from((start, end));

        TokenBuilder::new(other_token.kind())
            .span(other_token.span())
            .space(other_token.spacing())
            .build()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Bold,
    Italic,
    ItalicBold,
    Underline,
    Subscript,
    UnderlineSubscript,
    Superscript,
    Overline,
    Strikethrough,
    Highlight,
    Verbatim,
    Quote,
    Math,
    OpenParens,
    CloseParens,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Newline,
    Whitespace,
    Plain,
}

impl TokenKind {
    fn as_str(&self) -> &str {
        match *self {
            TokenKind::Bold => "**",
            TokenKind::ItalicBold => "***",
            TokenKind::Italic => "*",
            TokenKind::Newline => "\n",
            TokenKind::Whitespace => " ",
            TokenKind::Underline => "__",
            TokenKind::Subscript => "_",
            TokenKind::Superscript => "^",
            TokenKind::UnderlineSubscript => "___",
            TokenKind::Highlight => "||",
            TokenKind::Overline => "‾",
            TokenKind::Strikethrough => "~~",
            TokenKind::Verbatim => "`",
            TokenKind::Quote => "\"",
            TokenKind::Math => "$",
            TokenKind::OpenParens => "(",
            TokenKind::CloseParens => ")",
            TokenKind::OpenBracket => "[",
            TokenKind::CloseBracket => "]",
            TokenKind::OpenBrace => "{",
            TokenKind::CloseBrace => "}",
            TokenKind::Plain => "",
        }
    }

    pub(crate) fn content_matters(&self) -> bool {
        matches!(self, TokenKind::Plain)
    }

    pub(crate) fn content_option(&self) -> Content {
        if self.content_matters() {
            Content::Store
        } else {
            Content::Auto
        }
    }

    fn is_open_parentheses(&self) -> bool {
        matches!(self, Self::OpenParens | Self::OpenBracket | Self::OpenBrace)
    }

    fn is_close_parentheses(&self) -> bool {
        matches!(
            self,
            Self::CloseParens | Self::CloseBracket | Self::CloseBrace
        )
    }
}

impl From<(Symbol, usize)> for TokenKind {
    fn from((symbol, len): (Symbol, usize)) -> Self {
        match len {
            1 => match symbol {
                Symbol::Star => Self::Italic,
                Symbol::Underline => Self::Subscript,
                Symbol::Caret => Self::Superscript,
                Symbol::Tick => Self::Verbatim,
                Symbol::Overline => Self::Overline,
                Symbol::Dollar => Self::Math,
                Symbol::OpenParens => Self::OpenParens,
                Symbol::CloseParens => Self::CloseParens,
                Symbol::OpenBracket => Self::OpenBracket,
                Symbol::CloseBracket => Self::CloseBracket,
                Symbol::OpenBrace => Self::OpenBrace,
                Symbol::CloseBrace => Self::CloseBrace,
                Symbol::Esc | Symbol::Pipe | Symbol::Tilde | Symbol::Quote | Symbol::Plain => {
                    Self::Plain
                }
            },
            2 => match symbol {
                Symbol::Star => Self::Bold,
                Symbol::Underline => Self::Underline,
                Symbol::Pipe => Self::Highlight,
                Symbol::Tilde => Self::Strikethrough,
                Symbol::Quote => Self::Quote,
                Symbol::Esc
                | Symbol::Caret
                | Symbol::Tick
                | Symbol::Overline
                | Symbol::Dollar
                | Symbol::OpenParens
                | Symbol::CloseParens
                | Symbol::OpenBracket
                | Symbol::CloseBracket
                | Symbol::OpenBrace
                | Symbol::CloseBrace
                | Symbol::Plain => Self::Plain,
            },
            3 => match symbol {
                Symbol::Star => Self::ItalicBold,
                Symbol::Underline => Self::UnderlineSubscript,
                Symbol::Esc
                | Symbol::Caret
                | Symbol::Tick
                | Symbol::Overline
                | Symbol::Pipe
                | Symbol::Tilde
                | Symbol::Quote
                | Symbol::Dollar
                | Symbol::OpenParens
                | Symbol::CloseParens
                | Symbol::OpenBracket
                | Symbol::CloseBracket
                | Symbol::OpenBrace
                | Symbol::CloseBrace
                | Symbol::Plain => Self::Plain,
            },
            _ => Self::Plain,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spacing {
    Pre,
    Post,
    Both,
    None,
}

impl Default for Spacing {
    fn default() -> Self {
        Self::None
    }
}

impl AddAssign for Spacing {
    fn add_assign(&mut self, rhs: Self) {
        match self {
            Spacing::Both => {}
            Spacing::None => *self = rhs,
            Spacing::Pre => match rhs {
                Spacing::Post | Spacing::Both => *self = Spacing::Both,
                _ => {}
            },
            Spacing::Post => match rhs {
                Spacing::Pre | Spacing::Both => *self = Spacing::Both,
                _ => {}
            },
        };
    }
}

impl SubAssign for Spacing {
    fn sub_assign(&mut self, rhs: Self) {
        match self {
            Spacing::Both => *self = Spacing::None,
            Spacing::None => {}
            Spacing::Pre => match rhs {
                Spacing::Pre | Spacing::Both => *self = Spacing::None,
                _ => {}
            },
            Spacing::Post => match rhs {
                Spacing::Post | Spacing::Both => *self = Spacing::None,
                _ => {}
            },
        };
    }
}

impl Add for Spacing {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Sub for Spacing {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: Position,
    end: Position,
}

impl Span {
    pub fn start(&self) -> Position {
        self.start
    }

    pub fn end(&self) -> Position {
        self.end
    }
}

impl From<(Position, Position)> for Span {
    fn from((start, end): (Position, Position)) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.line += rhs.line;
        self.column += rhs.column;
    }
}

impl AddAssign<(usize, usize)> for Position {
    fn add_assign(&mut self, (line, column): (usize, usize)) {
        self.line += line;
        self.column += column;
    }
}

impl<T> Add<T> for Position
where
    Position: AddAssign<T>,
{
    type Output = Position;

    fn add(mut self, rhs: T) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign for Position {
    fn sub_assign(&mut self, rhs: Self) {
        self.line -= rhs.line;
        self.column -= rhs.column;
    }
}

impl SubAssign<(usize, usize)> for Position {
    fn sub_assign(&mut self, (line, column): (usize, usize)) {
        self.line -= line;
        self.column -= column;
    }
}

impl<T> Sub<T> for Position
where
    Position: SubAssign<T>,
{
    type Output = Position;

    fn sub(mut self, rhs: T) -> Self::Output {
        self -= rhs;
        self
    }
}
