//! Contains the [`InlineToken`], the [`InlineTokenKind`], and the [`InlineTokenIterator`](iterator::InlineTokenIterator).

use unimarkup_commons::lexer::{
    position::{Offset, Position},
    token::Token,
};

use self::kind::InlineTokenKind;

pub(crate) mod iterator;
pub(crate) mod kind;

/// Converted Unimarkup [`Token`] to make inline parsing easier.
///
/// # Lifetimes
///
/// * `'input` - lifetime of input the [`Token`] was lexed from.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct InlineToken<'input> {
    pub(crate) input: &'input str,
    pub(crate) offset: Offset,
    pub(crate) kind: InlineTokenKind,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl<'slice, 't, 'i> From<&'slice Token<'t>> for InlineToken<'i>
where
    't: 'slice,
    't: 'i,
{
    fn from(value: &'slice Token<'t>) -> Self {
        InlineToken {
            input: value.input,
            offset: value.offset,
            kind: InlineTokenKind::from(value.kind),
            start: value.start,
            end: value.end,
        }
    }
}

impl<'input> InlineToken<'input> {
    pub(crate) fn as_str(&self) -> &str {
        match self.kind {
            InlineTokenKind::Plain | InlineTokenKind::Lt(_) | InlineTokenKind::Gt(_) => {
                &self.input[self.offset.start..self.offset.end]
            }
            InlineTokenKind::EscapedPlain | InlineTokenKind::EscapedWhitespace => {
                &self.input[self.offset.start + 1..self.offset.end] // +1 to skip backslash
            }
            InlineTokenKind::Bold
            | InlineTokenKind::Italic
            | InlineTokenKind::BoldItalic
            | InlineTokenKind::Highlight
            | InlineTokenKind::Underline
            | InlineTokenKind::Subscript
            | InlineTokenKind::UnderlineSubscript
            | InlineTokenKind::Math
            | InlineTokenKind::Verbatim
            | InlineTokenKind::Overline
            | InlineTokenKind::Superscript
            | InlineTokenKind::DoubleQuote
            | InlineTokenKind::SingleQuote
            | InlineTokenKind::Strikethrough
            | InlineTokenKind::NamedSubstitution
            | InlineTokenKind::OpenBrace
            | InlineTokenKind::OpenBracket
            | InlineTokenKind::OpenParenthesis
            | InlineTokenKind::CloseBrace
            | InlineTokenKind::CloseBracket
            | InlineTokenKind::CloseParenthesis
            | InlineTokenKind::Whitespace
            | InlineTokenKind::Newline
            | InlineTokenKind::EscapedNewline
            | InlineTokenKind::Comment
            | InlineTokenKind::SingleDot
            | InlineTokenKind::Digit(_)
            | InlineTokenKind::MediaInsert
            | InlineTokenKind::Eoi => self.kind.as_str(),
            InlineTokenKind::ImplicitSubstitution(impl_subst) => impl_subst.orig(), // using `orig()` here, because `as_str()` is only called to convert to plain content
            InlineTokenKind::Any
            | InlineTokenKind::Space
            | InlineTokenKind::Digits
            | InlineTokenKind::PossibleAttributes => {
                #[cfg(debug_assertions)]
                panic!(
                    "Tried to create &str from '{:?}', which has undefined &str representation.",
                    self
                );

                #[cfg(not(debug_assertions))]
                ""
            }
        }
    }
}
