use crate::{
    comments::Comment,
    lexer::{position::Position, symbol::SymbolKind, token::implicit::ImplicitSubstitutionKind},
    logic::LogicAst,
    parsing::Element,
};

use super::rules::AtRuleId;

#[derive(Debug, PartialEq, Clone)]
pub struct AttributeTokens {
    pub(crate) tokens: Vec<AttributeToken>,
    pub(crate) implicit_closed: bool,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Element for AttributeTokens {
    fn as_unimarkup(&self) -> String {
        self.tokens.iter().fold(String::new(), |mut s, t| {
            s.push_str(&t.as_unimarkup());
            s
        })
    }

    fn start(&self) -> crate::lexer::position::Position {
        self.start
    }

    fn end(&self) -> crate::lexer::position::Position {
        self.end
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AttributeToken {
    pub(crate) kind: AttributeTokenKind,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Element for AttributeToken {
    fn as_unimarkup(&self) -> String {
        self.kind.as_unimarkup()
    }

    fn start(&self) -> Position {
        self.start
    }

    fn end(&self) -> Position {
        self.end
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AttributeTokenKind {
    /// Attribute ident that ends with `: `.
    /// The stored ident does **not** include the ending `: `.
    /// Quoted idents are allowed for Unimarkup attributes, but must not span multiple lines.
    /// e.g. `"quoted ident": 2023`
    ///
    /// **Note:** The whitespace after `:` is required, but may be any non-escaped whitespace.
    /// This requirement differs from the CSS specification, but makes distinguishing single or nested properties much easier.
    /// It also allows to create properties with object-like arrays as value.
    /// See: https://www.w3.org/TR/css-syntax-3/#parsing, https://github.com/w3c/csswg-drafts/issues/9317
    Ident(Ident),
    /// Value part of a non-nested attribute.
    /// May only be part of the complete value, because the value might be split by newlines or comments.
    ValuePart(ValuePart),
    /// Selector part for a nested attribute.
    /// May only be part of the selector, because it can span multiple lines in case of a selector.
    ///
    SelectorPart(TokenPart),
    /// At-rule ident starting with `@`.
    /// The stored ident does **not** include the `@`.
    ///
    /// **Note:** Identifiers must be separated by e.g. whitespace to distinguish between at-rule ident and follow up ident.
    /// This differs from the CSS specification, but helps to handle custom at-rules.
    AtRuleIdent(AtRuleId),
    /// Rule prelude part that is between an at-rule ident and a semicolon or nested block.
    /// May only be part of the prelude, because it can span multiple lines.
    /// e.g. `@<rule ident> <prelude part> {<optional nested block>}`
    AtRulePreludePart(TokenPart),
    /// Tokens surrounded by `{}`.
    /// Nested blocks are implicity closed if the underlying token iterator ends, before `}` is reached.
    Nested(AttributeTokens),
    Logic(LogicAst),
    /// A Unimarkup comment.
    /// e.g. `;; This is a comment`
    ///
    /// **Note:** CSS comment syntax is **not** supported.
    Comment(Comment),
    /// A single semicolon used as declaration separator.
    Semicolon,
    /// A single comma used as value separator.
    Comma,
    /// A quoted part (e.g. `"value"` or `'value'`).
    QuotedPart(QuotedPart),
    Whitespace,
    Newline,
}

impl AttributeTokenKind {
    pub fn as_unimarkup(&self) -> String {
        match self {
            AttributeTokenKind::Ident(ident) => ident.0.clone() + ": ",
            AttributeTokenKind::ValuePart(value_part) => value_part.as_unimarkup(),
            AttributeTokenKind::SelectorPart(nested_ident_part) => nested_ident_part.0.clone(),
            AttributeTokenKind::AtRuleIdent(at_rule_ident) => {
                format!("@{}", at_rule_ident.as_str())
            }
            AttributeTokenKind::AtRulePreludePart(at_rule_prelude_part) => {
                at_rule_prelude_part.0.clone()
            }
            AttributeTokenKind::Nested(inner) => {
                format!(
                    "{{{}{}",
                    inner.as_unimarkup(),
                    if inner.implicit_closed {
                        ""
                    } else {
                        SymbolKind::CloseBrace.as_str()
                    }
                )
            }
            AttributeTokenKind::Logic(logic) => logic.as_unimarkup(),
            AttributeTokenKind::Comment(comment) => comment.as_unimarkup(),
            AttributeTokenKind::Semicolon => SymbolKind::Semicolon.as_str().to_string(),
            AttributeTokenKind::Comma => SymbolKind::Comma.as_str().to_string(),
            AttributeTokenKind::QuotedPart(value) => {
                let quote = value.quote;
                format!("{quote}{}{quote}", value.as_unimarkup())
            }
            AttributeTokenKind::Whitespace => SymbolKind::Whitespace.as_str().to_string(),
            AttributeTokenKind::Newline => SymbolKind::Newline.as_str().to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct QuotedPart {
    pub(crate) parts: Vec<QuotedValuePart>,
    pub(crate) quote: char,
    pub(crate) implicit_closed: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct QuotedValuePart {
    pub(crate) kind: QuotedPartKind,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Element for QuotedPart {
    fn as_unimarkup(&self) -> String {
        self.parts.iter().fold(String::new(), |mut s, q| {
            s.push_str(&q.kind.as_unimarkup());
            s
        })
    }

    fn start(&self) -> Position {
        self.parts.first().map(|p| p.start).unwrap_or_default()
    }

    fn end(&self) -> Position {
        self.parts.last().map(|p| p.end).unwrap_or_default()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum QuotedPartKind {
    /// Contains plain content.
    /// Including escaped graphemes and whitespaces.
    /// Escaped graphemes are added to the content **without** the leading backslash.
    /// e.g. "\ " <- escaped whitespace & "\n" <- escaped `n`.
    Plain(String),
    /// Implicit substitutions except `DirectUri` are converted to their *rendered* representations.
    ImplicitSubstitution(ImplicitSubstitutionKind),
    /// Named substitutions are converted to their *rendered* representations.
    /// This is possible, because the content of named susbtitutions may only consist of plain content, whitespaces, newlines, escaped variants, or implicit substitutions.
    NamedSubstitution(Ident),
    Logic(LogicAst),
    EscapedNewline,
    Newline,
}

impl QuotedPartKind {
    pub fn as_unimarkup(&self) -> String {
        match self {
            QuotedPartKind::Plain(plain) => plain.clone(),
            QuotedPartKind::ImplicitSubstitution(implicit_subst) => {
                implicit_subst.orig().to_string()
            }
            QuotedPartKind::NamedSubstitution(named_subst) => named_subst.0.clone(),
            QuotedPartKind::Logic(logic) => logic.as_unimarkup(),
            QuotedPartKind::EscapedNewline | QuotedPartKind::Newline => {
                SymbolKind::Newline.as_str().to_string()
            }
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct Ident(String);

impl std::ops::Deref for Ident {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Ident {
    fn from(value: String) -> Self {
        Ident(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenPart(String);

impl std::ops::Deref for TokenPart {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for TokenPart {
    fn from(value: String) -> Self {
        TokenPart(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ValuePart {
    Plain(String),
    Num(usize),
    /// The `!important` marker.
    /// See: https://www.w3.org/TR/css-syntax-3/#!important-diagram
    Important,
}

impl ValuePart {
    pub fn as_unimarkup(&self) -> String {
        match self {
            ValuePart::Plain(plain) => plain.clone(),
            ValuePart::Num(num) => num.to_string(),
            ValuePart::Important => "!important".to_string(),
        }
    }
}
