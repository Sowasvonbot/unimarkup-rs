use std::rc::Rc;

use unimarkup_commons::{
    parser::{GroupParser, Parser},
    scanner::{EndMatcher, Itertools, SymbolKind},
};

use crate::{
    element::{Inline, InlineElement, InlineError},
    inline_parser,
    new_parser::InlineParser,
    tokenize::{iterator::InlineTokenIterator, token::InlineTokenKind},
};

use super::{Bold, Italic, BOLD_ITALIC_KEYWORD_LIMIT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoldItalic {
    Bold(Bold),
    Italic(Italic),
}

pub fn parse(input: &mut InlineTokenIterator) -> Option<Inline> {
    let mut open_token = input.next()?;

    if input.peek_kind()?.is_space() {
        // Split ambiguous in case of leading space. Bold wins
        if open_token.kind == InlineTokenKind::ItalicBold {
            let mut cached = open_token;
            cached.kind = InlineTokenKind::Italic;

            //TODO: update spans

            input.cache_token(cached);
            open_token.kind = InlineTokenKind::Bold;
        } else {
            return None;
        }
    }

    match open_token.kind {
        InlineTokenKind::ItalicBold => {
            input.push_format(InlineTokenKind::Italic);
            input.push_format(InlineTokenKind::Bold);
        }
        InlineTokenKind::Italic | InlineTokenKind::Bold => {
            input.push_format(open_token.kind);
        }
        _ => {
            return None;
        }
    }

    let inner = inline_parser::InlineParser::default().parse(input);

    resolve_closing(input, inner)
}

fn resolve_closing(input: &mut InlineTokenIterator, inline: Vec<Inline>) -> Option<Inline> {
    todo!();

    match input.peek() {
        Some(close_token) => {
            // open = bold, close = italic => italic was opened in other parser => close bold, but do not consume close and no second part

            // open = italic, close = bold => bold was opened in other parser => close italic, but do not consume close and no second part

            // open & close = italic => close italic and consume close and no second part
            // open & close = bold => close bold and consume close and no second part
            // open & close = italicbold => close italicbold and consume close and no second part

            // open = bold, close = italicbold => close bold, consume close, cache italic and parse second part to get possible italic open
            // open = italic, close = italicbold => close italic, consume close, cache bold and parse second part to get possible bold open

            // open = italicbold, close = bold => close bold, consume close and parse second part (split span of open)
            // open = italicbold, close = italic => close italic, consume close and parse second part
        }
        None => {
            // close open format only and return
        }
    }
}

impl InlineElement for BoldItalic {}

impl Parser<Inline> for BoldItalic {
    fn parse(input: &mut unimarkup_commons::scanner::SymbolIterator) -> Option<Inline> {
        let first_symbol = input.next()?;
        let second_symbol = input.next()?;
        let third_symbol = input.next()?;
        let fourth_symbol = input.peek()?;

        if first_symbol.kind != SymbolKind::Star
            || second_symbol.kind != SymbolKind::Star
            || third_symbol.kind != SymbolKind::Star
            || fourth_symbol.kind == SymbolKind::Star
            || fourth_symbol.kind.is_space()
        {
            return None;
        }

        let mut inner_iter = input.nest_scoped(
            None,
            Some(Rc::new(|matcher: &mut dyn EndMatcher| {
                !matcher.prev_is_space()
                // Contiguous keywords are consumed in inline parser
                && !matcher.matches(BOLD_ITALIC_KEYWORD_LIMIT)
                && matcher.matches(&[SymbolKind::Star])
            })),
        );

        let inline_parser = InlineParser::default();
        let inner = inline_parser.parse(&mut inner_iter);
        let inner_end = inner_iter.end_reached();

        inner_iter.update(input);

        let mut star_iter = input.nest_scoped(None, None);
        let star_cnt = star_iter
            .peeking_take_while(|s| s.kind == SymbolKind::Star)
            .count();

        star_iter.by_ref().dropping(star_cnt);
        star_iter.update(input);

        if !inner_end || star_cnt > 2 || star_cnt == 0 {
            return Some(
                Bold {
                    inner: vec![Italic { inner }.into()],
                }
                .into(),
            );
        }

        let mut outer_iter = if star_cnt == 1 {
            // Italic closed => outer is Bold
            input.nest_scoped(
                None,
                Some(Rc::new(|matcher: &mut dyn EndMatcher| {
                    !matcher.prev_is_space()
                        // Contiguous keywords are consumed in inline parser
                        && !matcher.matches(BOLD_ITALIC_KEYWORD_LIMIT)
                        && matcher.consumed_matches(&[SymbolKind::Star, SymbolKind::Star])
                })),
            )
        } else {
            // Bold closed => outer is Italic
            input.nest_scoped(
                None,
                Some(Rc::new(|matcher: &mut dyn EndMatcher| {
                    !matcher.prev_is_space()
                        && !matcher.matches(&[SymbolKind::Star, SymbolKind::Star])
                        && matcher.consumed_matches(&[SymbolKind::Star])
                })),
            )
        };

        let inline_parser = InlineParser::default();
        let outer = inline_parser.parse(&mut outer_iter);

        outer_iter.update(input);

        if star_cnt == 1 {
            let mut content = vec![Italic { inner }.into()];
            content.extend(outer);
            Some(Bold { inner: content }.into())
        } else {
            let mut content = vec![Bold { inner }.into()];
            content.extend(outer);
            Some(Italic { inner: content }.into())
        }
    }
}

impl From<BoldItalic> for Inline {
    fn from(value: BoldItalic) -> Self {
        match value {
            BoldItalic::Bold(bold) => Inline::Bold(bold),
            BoldItalic::Italic(italic) => Inline::Italic(italic),
        }
    }
}

impl TryFrom<Inline> for BoldItalic {
    type Error = InlineError;

    fn try_from(value: Inline) -> Result<Self, Self::Error> {
        match value {
            Inline::Bold(bold) => Ok(BoldItalic::Bold(bold)),
            Inline::Italic(italic) => Ok(BoldItalic::Italic(italic)),
            _ => Err(InlineError::ConversionMismatch),
        }
    }
}

#[cfg(test)]
mod test {
    use unimarkup_commons::scanner::SymbolIterator;

    use crate::element::plain::Plain;

    use super::*;

    #[test]
    fn parse_bold_italic() {
        let symbols = unimarkup_commons::scanner::scan_str("***bold**italic*");
        let mut sym_iter = SymbolIterator::from(&*symbols);

        let inline = BoldItalic::parse(&mut sym_iter).unwrap();

        assert_eq!(
            Italic::try_from(inline).unwrap(),
            Italic {
                inner: vec![
                    Bold {
                        inner: vec![Plain {
                            content: "bold".to_string(),
                        }
                        .into()]
                    }
                    .into(),
                    Plain {
                        content: "italic".to_string(),
                    }
                    .into()
                ],
            },
            "Bold + italic not correctly parsed."
        );

        assert_eq!(
            sym_iter.next().unwrap().kind,
            SymbolKind::EOI,
            "Iterator returned symbols after EOI"
        );
    }

    #[test]
    fn parse_italic_bold() {
        let symbols = unimarkup_commons::scanner::scan_str("***italic*bold**");
        let mut sym_iter = SymbolIterator::from(&*symbols);

        let inline = BoldItalic::parse(&mut sym_iter).unwrap();

        assert_eq!(
            Bold::try_from(inline).unwrap(),
            Bold {
                inner: vec![
                    Italic {
                        inner: vec![Plain {
                            content: "italic".to_string(),
                        }
                        .into()]
                    }
                    .into(),
                    Plain {
                        content: "bold".to_string(),
                    }
                    .into()
                ],
            },
            "Italic + bold not correctly parsed."
        );

        assert_eq!(
            sym_iter.next().unwrap().kind,
            SymbolKind::EOI,
            "Iterator returned symbols after EOI"
        );
    }

    #[test]
    fn parse_italic_bold_with_contiguous_stars() {
        let symbols = unimarkup_commons::scanner::scan_str("***italic*a****a**");
        let mut sym_iter = SymbolIterator::from(&*symbols);

        let inline = BoldItalic::parse(&mut sym_iter).unwrap();

        assert_eq!(
            Bold::try_from(inline).unwrap(),
            Bold {
                inner: vec![
                    Italic {
                        inner: vec![Plain {
                            content: "italic".to_string(),
                        }
                        .into()]
                    }
                    .into(),
                    Plain {
                        content: "a****a".to_string(),
                    }
                    .into()
                ],
            },
            "Italic + bold not correctly parsed."
        );

        assert_eq!(
            sym_iter.next().unwrap().kind,
            SymbolKind::EOI,
            "Iterator returned symbols after EOI"
        );
    }
}
