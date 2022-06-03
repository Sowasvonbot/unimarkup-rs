use std::ops::Deref;

use crate::{
    Inline, InlineContent, InlineKind, PlainInline, Span, Token, TokenIterator, TokenKind, Tokenize,
};

#[derive(Debug, Default, Clone)]
struct ParserStack {
    data: Vec<Token>,
}

impl Deref for ParserStack {
    type Target = Vec<Token>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ParserStack {
    /// Pushes the element onto the stack and returns the index of the element
    pub fn push(&mut self, token: Token) -> usize {
        self.data.push(token);
        self.data.len() - 1
    }

    pub fn pop_last(&mut self) -> Option<Token> {
        self.data.pop()
    }

    /// Removes and returns the last item on stack
    pub fn pop(&mut self, token: &Token) -> Option<Token> {
        if self.data.is_empty() {
            None
        } else {
            let last_open_token = self.data.last_mut().unwrap();

            if last_open_token.is_ambiguous() {
                // remove the ambiguous part...
                let removed_token = last_open_token.remove_partial(token);

                Some(removed_token)
            } else {
                self.data.pop()
            }
        }
    }
}

pub struct Parser<'i> {
    iter: TokenIterator<'i>,
    stack: ParserStack,
    token_cache: Option<Token>,
    stack_cache: Vec<ParserStack>,
    stack_was_empty: bool,
}

impl Parser<'_> {
    fn next_token(&mut self) -> Option<Token> {
        if self.token_cache.is_some() {
            self.token_cache.take()
        } else {
            self.iter.next()
        }
    }

    fn is_token_open(&self, token: &Token) -> bool {
        let res = self.stack.iter().any(|inner_token| {
            inner_token.is_or_contains(token)
                || token.is_or_contains(inner_token)
                || inner_token.matches_pair(token)
        });

        res
    }

    fn is_token_latest(&self, token: &Token) -> bool {
        match self.stack().last() {
            Some(last_open_token) => {
                last_open_token.is_or_contains(token) || last_open_token.matches_pair(token)
            }
            None => false,
        }
    }

    /// Returns a mutable reference to the currently active stack - corresponding to current scope.
    fn stack_mut(&mut self) -> &mut ParserStack {
        &mut self.stack
    }

    /// Returns a reference to the currently active stack - corresponding to current scope.
    fn stack(&self) -> &ParserStack {
        &self.stack
    }

    /// Creates a new stack for the scope and sets it as the currently active stack.
    fn enter_scope(&mut self) {
        let new_stack = ParserStack::default();

        let old_stack = std::mem::replace(&mut self.stack, new_stack);

        self.stack_cache.push(old_stack);

        // println!("Stack after enter scope: {:#?}", self.stack())
    }

    /// Removes the currently active stack and restores the stack of the outer scope.
    fn exit_scope(&mut self) {
        self.stack_was_empty = self.stack().len() == 0;

        match self.stack_cache.pop() {
            Some(old_stack) => self.stack = old_stack,
            None => self.stack = ParserStack::default(),
        }
    }

    /// Pushes a token to the currently active stack.
    fn push_to_stack(&mut self, token: Token) -> usize {
        if matches!(token.kind(), TokenKind::OpenBracket) {
            self.enter_scope();
        }

        self.stack_mut().push(token)
    }

    /// Pops the token last added to the currently active stack.
    fn pop_last(&mut self) -> Option<Token> {
        match self.stack_mut().pop_last() {
            Some(token) => {
                if matches!(token.kind(), TokenKind::OpenBracket) {
                    self.exit_scope();
                }

                Some(token)
            }

            None => None,
        }
    }

    /// Pops the (part of) token that matches the token reference passed to the function.
    ///
    /// In case that token on stack contains the passed token, only the part that matches the
    /// passed token gets removed, and the rest of the token stays on the stack.
    /// This means that even if there is only one token on the stack and `pop()` is called,
    /// there might still be one token left on the stack.
    fn pop(&mut self, token: &Token) -> Option<Token> {
        let removed_token = self.stack_mut().pop(token)?;

        if matches!(removed_token.kind(), TokenKind::OpenBracket) {
            self.exit_scope();
        }

        Some(removed_token)
    }

    fn last_token(&self) -> Option<&Token> {
        self.stack().last()
    }

    fn parse_nested_inline(&mut self, token: Token) -> Inline {
        // Push token kind to stack
        // Open corresponding inline
        // If nesting of inline occurs, parse inner inline -> PROBLEM: Ambiguous tokens?
        // Parse until closing token is found
        // Close inline and return it

        // PROBLEM: AmbiguousToken that comes as next token
        // example: **Bold Text***Italic text*
        //            ^^^^^^^^^   ^^^^^^^^^^^
        //              BOLD        ITALIC
        //  So the ambiguous token AFTER bold content (***) should be split into
        //  bold close token and italic open. That means, that the ambiguous token should be split,
        //  first part taken (based on what part was open) and the second part left for the next
        //  iteration

        let mut kind = token.kind();
        let mut content: InlineContent = InlineContent::Nested(Vec::default());
        let start = token.span().start();
        let mut end = start;

        self.push_to_stack(token);

        while let Some(mut next_token) = self.next_token() {
            // Multiple cases:
            // 1. token is (nesting one and) already open
            //      - Is it closing one and it was open last? Close Inline
            //      - Is it closing one, but it was not open last? Return inline and merge into outer one
            //      - If not closing one, then it's plain text
            //      - If no more tokens available, then:
            //          -> First token (opening one) should be treated as plain text
            //          -> All inlines found inside should be given as such
            //          -> That means that the inline becomes: (PlainInline, Inline, Inline...)
            // 2. token is not already open
            //      - content until token is plain text

            if next_token.closes() {
                if self.is_token_open(&next_token) {
                    if self.is_token_latest(&next_token) {
                        // It is closing one and it was open last -> Close Inline
                        end = next_token.span().end();

                        self.pop_last();
                        break;
                    } else {
                        // It might be ambiguous token and part of it is open,
                        // for example ** followed by ***. Such token should be split as **|*,
                        // where first part (**) is being closed, and second part (*) is now in
                        // token_cache for next iteration

                        if next_token.is_ambiguous() {
                            // at this point we know there is at least one token in stack
                            let last_token = self.last_token().unwrap();

                            if next_token.is_or_contains(last_token) {
                                let _parsed_token = next_token.remove_partial(last_token);

                                self.pop_last();

                                self.token_cache = Some(next_token);

                                // close this inline
                                break;
                            }
                        } else {
                            // It is closing one, but it was not open last -> Return contents as inline

                            // remove the opening token from the stack
                            let token = self.pop(&next_token).unwrap();

                            // NOTE: when coming from nested, while loop will be continued -> takes
                            // another token from iterator or cache
                            self.token_cache = Some(next_token);

                            // prepend the token to content as plain text
                            content.prepend(InlineContent::from(token));

                            return Inline {
                                inner: content,
                                span: Span::from((start, end)),
                                kind: InlineKind::Plain,
                            };
                        }
                    }
                } else {
                    // plain text
                    end = next_token.span().end();

                    // consume plain text
                    content.append(InlineContent::from(next_token));
                }
            } else if next_token.opens() {
                if self.is_token_open(&next_token) {
                    // plain text

                    // update end position
                    end = next_token.span().end();

                    // consume plain text
                    content.append(InlineContent::from(next_token));
                } else {
                    // parse open and merge into upper one
                    let nested = self.parse_nested_inline(next_token);

                    end = nested.span().end();

                    content.append_inline(nested);
                }
            } else {
                // neither opens nor closes - is plain text
                end = next_token.span().end();

                let inline_content = InlineContent::from(next_token);
                content.append(inline_content);
            }
        }

        let span = Span::from((start, end));

        let is_inline_closed = if let Some(token) = self.last_token() {
            token.kind() != kind
        } else {
            true
        };

        if !is_inline_closed {
            if let Some(last_token) = self.pop_last() {
                content.prepend(InlineContent::from(last_token));
            }
        }

        // if content contains only plain contents, then merge them and make into one
        content.try_flatten();

        Inline::new(span, content, kind)
    }
}

impl Iterator for Parser<'_> {
    type Item = Inline;

    fn next(&mut self) -> Option<Self::Item> {
        let next_token = self.next_token()?;

        if next_token.opens() {
            Some(self.parse_nested_inline(next_token))
        } else {
            let kind = next_token.kind();

            let (content, span) = next_token.into_inner();
            let inline_content = InlineContent::Plain(PlainInline { content, span });

            Some(Inline::new(span, inline_content, kind))
        }
    }
}

pub trait ParseUnimarkupInlines<'p, 'i>
where
    'i: 'p,
{
    fn parse_unimarkup_inlines(&'i self) -> Parser<'p>;
}

impl<'p, 'i> ParseUnimarkupInlines<'p, 'i> for &str
where
    'i: 'p,
{
    fn parse_unimarkup_inlines(&'i self) -> Parser<'p> {
        Parser {
            iter: self.lex_iter(),
            stack: ParserStack::default(),
            token_cache: None,
            stack_cache: Vec::default(),
            stack_was_empty: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Position;

    use super::*;

    #[test]
    fn parse_simple_plain() {
        let parser = "Some text".parse_unimarkup_inlines();

        assert_eq!(parser.count(), 1);
    }

    #[test]
    fn parse_simple_bold() {
        let mut parser = "**Bold text**".parse_unimarkup_inlines();

        let inline = parser.next().unwrap();
        let start = Position { line: 1, column: 3 };
        let end = start + (0, 9 - 1);

        // no remaining inlines
        assert_eq!(parser.count(), 0);
        assert_eq!(inline.kind, InlineKind::Bold);
        assert_eq!(
            inline.inner,
            InlineContent::Plain(PlainInline {
                content: String::from("Bold text"),
                span: Span::from((start, end))
            })
        );
    }

    #[test]
    fn parse_simple_italic() {
        let mut parser = "*Italic text*".parse_unimarkup_inlines();

        let inline = parser.next().unwrap();
        let start = Position { line: 1, column: 2 };
        let end = start + (0, 11 - 1);

        // no remaining inlines
        assert_eq!(parser.count(), 0);
        assert_eq!(inline.kind, InlineKind::Italic);
        assert_eq!(
            inline.inner,
            InlineContent::Plain(PlainInline {
                content: String::from("Italic text"),
                span: Span::from((start, end))
            })
        );
    }

    #[test]
    fn parse_italic_bold() {
        let mut parser = "*Italic text***Bold text**".parse_unimarkup_inlines();

        let inline = parser.next().unwrap();
        let start = Position { line: 1, column: 2 };
        let end = start + (0, 11 - 1);

        assert_eq!(inline.kind, InlineKind::Italic);
        assert_eq!(
            inline.inner,
            InlineContent::Plain(PlainInline {
                content: String::from("Italic text"),
                span: Span::from((start, end))
            })
        );

        let inline = parser.next().unwrap();
        let start = end + (0, 5 - 1);
        let end = start + (0, 9 - 1);

        assert_eq!(inline.kind, InlineKind::Bold);
        assert_eq!(
            inline.inner,
            InlineContent::Plain(PlainInline {
                content: String::from("Bold text"),
                span: Span::from((start, end))
            })
        );
    }

    #[test]
    fn parse_bold_italic_nested() {
        let mut parser = "**This is bold *with* italic inside.**".parse_unimarkup_inlines();

        let inline = parser.next().unwrap();
        let start = Position { line: 1, column: 1 };
        let end = start + (0, 38 - 1);

        // no remaining inlines
        assert_eq!(parser.count(), 0);

        // println!("Inline span: {:#?}", inline.span());

        assert_eq!(inline.kind, InlineKind::Bold);
        assert_eq!(inline.span(), Span::from((start, end)));
        assert!(matches!(inline.inner, InlineContent::Nested(_)));

        if let InlineContent::Nested(inner_content) = inline.into_inner() {
            assert_eq!(inner_content.len(), 3);

            let inline = &inner_content[0];

            let start = Position { line: 1, column: 3 };
            let end = start + (0, 13 - 1);

            assert_eq!(inline.kind, InlineKind::Plain);
            assert_eq!(
                inline.inner,
                InlineContent::Plain(PlainInline {
                    content: String::from("This is bold "),
                    span: Span::from((start, end))
                })
            );

            let inline = &inner_content[1];

            let start = end + (0, 1);
            let end = start + (0, 6 - 1);

            let inner_start = start + (0, 1);
            let inner_end = end - (0, 1);

            assert_eq!(inline.kind, InlineKind::Italic);
            assert_eq!(
                inline.inner,
                InlineContent::Plain(PlainInline {
                    content: String::from("with"),
                    span: Span::from((inner_start, inner_end))
                })
            );
            assert_eq!(inline.span(), Span::from((start, end)));

            let inline = &inner_content[2];

            let start = end + (0, 1);
            let end = start + (0, 15 - 1);

            assert_eq!(inline.kind, InlineKind::Plain);
            assert_eq!(
                inline.inner,
                InlineContent::Plain(PlainInline {
                    content: String::from(" italic inside."),
                    span: Span::from((start, end))
                })
            );
        } else {
            panic!("Inner content not nested");
        }
    }

    #[test]
    fn parse_text_group_simple() {
        let mut parser = "This is text [with text group] as part of it.".parse_unimarkup_inlines();

        let inline = parser.next().unwrap();
        let start = Position { line: 1, column: 1 };
        let end = start + (0, 13 - 1);

        assert_eq!(inline.kind, InlineKind::Plain);
        assert_eq!(inline.span(), Span::from((start, end)));
        assert!(matches!(inline.inner, InlineContent::Plain(_)));

        let inline = parser.next().unwrap();
        let start = end + (0, 1);
        let end = start + (0, 17 - 1);

        assert_eq!(inline.kind, InlineKind::TextGroup);
        assert_eq!(inline.span(), Span::from((start, end)));
        assert!(matches!(inline.inner, InlineContent::Plain(_)));

        let inline = parser.next().unwrap();
        let start = end + (0, 1);
        let end = start + (0, 15 - 1);

        assert_eq!(inline.kind, InlineKind::Plain);
        assert_eq!(inline.span(), Span::from((start, end)));
        assert!(matches!(inline.inner, InlineContent::Plain(_)));
    }

    #[test]
    fn parse_text_group_interrupt_bold() {
        let input = "This is **text [with text** group] as part of it.";
        let parser = input.parse_unimarkup_inlines();

        println!("Parsing following text: \"{input}\"");
        for inline in parser {
            println!("{inline:#?}");
        }

        // let inline = parser.next().unwrap();
        // let start = Position { line: 1, column: 1 };
        // let end = start + (0, 13 - 1);
        //
        // assert_eq!(inline.kind, TokenKind::Plain);
        // assert_eq!(inline.span(), Span::from((start, end)));
        // assert!(matches!(inline.inner, InlineContent::Plain(_)));
        //
        // let inline = parser.next().unwrap();
        // let start = end + (0, 1);
        // let end = start + (0, 17 - 1);
        //
        // assert_eq!(inline.kind, TokenKind::OpenBracket);
        // assert_eq!(inline.span(), Span::from((start, end)));
        // assert!(matches!(inline.inner, InlineContent::Plain(_)));
        //
        // let inline = parser.next().unwrap();
        // let start = end + (0, 1);
        // let end = start + (0, 15 - 1);
        //
        // assert_eq!(inline.kind, TokenKind::Plain);
        // assert_eq!(inline.span(), Span::from((start, end)));
        // assert!(matches!(inline.inner, InlineContent::Plain(_)));
    }
}
