use std::str::Lines;

use unicode_segmentation::*;

mod token;

pub use token::*;

/// Used to create a Unimarkup [`Lexer`] over some data structure, most typically over some kind of
/// string, i.e. [`&str`].
///
/// [`Lexer`]: self::Lexer
/// [`&str`]: &str
pub trait Tokenize {
    /// Creates the `Lexer` from this type.
    fn lex(&self) -> Lexer;

    /// Creates the `Lexer` from this type starting at the given offset.
    fn lex_with_offs(&self, pos: Position) -> Lexer {
        Lexer { pos, ..self.lex() }
    }

    /// Creates an [`TokenIterator`] from this type.
    ///
    /// [`TokenIterator`]: self::TokenIterator
    fn lex_iter(&self) -> TokenIterator;

    /// Creates an [`TokenIterator`] from this type starting at the given offset.
    ///
    /// [`TokenIterator`]: self::TokenIterator
    fn lex_iter_with_offs(&self, pos: Position) -> TokenIterator {
        let lexer = self.lex_with_offs(pos);

        lexer.iter()
    }
}

impl<'a> Tokenize for &'a str {
    fn lex(&self) -> Lexer {
        Lexer {
            input: self,
            pos: Position { line: 1, column: 1 },
        }
    }

    fn lex_iter(&self) -> TokenIterator {
        self.lex().iter()
    }
}

/// Lexer of Unimarkup inline formatted text. Generates a stream of [`Token`]s from input.
///
/// [`Token`]: self::token::Token
pub struct Lexer<'a> {
    input: &'a str,
    pos: Position,
}

/// Symbols with significance in Unimarkup inline formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Symbol<'a> {
    /// The backslash (`\`) is used for escaping other symbols.
    Esc,
    /// The start (`*`) literal is used for bold and/or italic formatting.
    Star,
    /// The underline (`_`) literal is used for undeline and/or subscript formatting.
    Underline,
    /// The caret (`^`) literal is used for superscript formatting.
    Caret,
    /// The tick (```) literal is used for verbatim formatting.
    Tick,
    /// The overline (`‾`) literal is used for overline formatting.
    Overline,
    /// The pipe (`|`) literal is used for highlight formatting.
    Pipe,
    /// The tilde (`~`) literal is used for strikethrough formatting.
    Tilde,
    /// The quote (`"`) literal is used for quotation formatting.
    Quote,
    /// The dollar (`$`) literal is used for math mode formatting.
    Dollar,
    /// The open parentheses (`(`) literal is used for additional data to text group elements (e.g.
    /// image insert).
    OpenParens,
    /// The close parentheses (`)`) literal is used to close the additional data to text group.
    CloseParens,
    /// The open bracket (`[`) literal is used for text group elements.
    OpenBracket,
    /// The close bracket (`]`) literal is used for text group elements.
    CloseBracket,
    /// The open brace (`{`) literal is used for inline attributes.
    OpenBrace,
    /// The close brace (`}`) literal is used for inline attributes.
    CloseBrace,
    /// The plain symbol represents any other literal with no significance in Unimarkup inline
    /// formatting.
    Plain(&'a str),
    /// A whitespace literal (` `)
    Whitespace(&'a str),
    /// A newline literal (`\n` or '\r\n')
    Newline,
    // Colon,
}

impl<'a> From<&'a str> for Symbol<'a> {
    fn from(input: &'a str) -> Self {
        match input {
            "\\" => Symbol::Esc,
            "*" => Symbol::Star,
            "_" => Symbol::Underline,
            "^" => Symbol::Caret,
            "`" => Symbol::Tick,
            "‾" => Symbol::Overline,
            "|" => Symbol::Pipe,
            "~" => Symbol::Tilde,
            "\"" => Symbol::Quote,
            "$" => Symbol::Dollar,
            "(" => Symbol::OpenParens,
            ")" => Symbol::CloseParens,
            "[" => Symbol::OpenBracket,
            "]" => Symbol::CloseBracket,
            "{" => Symbol::OpenBrace,
            "}" => Symbol::CloseBrace,
            "\n" | "\r\n" => Symbol::Newline,
            other => match other.chars().next() {
                // NOTE: multi-character grapheme is most probably not a whitespace
                Some(literal) if literal.is_whitespace() => Symbol::Whitespace(other),
                _ => Symbol::Plain(input),
            },
        }
    }
}

impl<'a> From<&&'a str> for Symbol<'a> {
    fn from(input: &&'a str) -> Self {
        Self::from(*input)
    }
}

impl From<Symbol<'_>> for String {
    fn from(symbol: Symbol) -> Self {
        String::from(symbol.as_str())
    }
}

impl Symbol<'_> {
    /// Returns the [`LexLength`] a given symbol may have.
    ///
    /// [`LexLength`]: self::LexLength
    pub(crate) fn allowed_len(&self) -> LexLength {
        match self {
            Symbol::Star | Symbol::Underline => LexLength::Limited(3),

            Symbol::Esc | Symbol::Caret | Symbol::Overline | Symbol::Tick | Symbol::Dollar => {
                LexLength::Limited(1)
            }

            Symbol::OpenParens
            | Symbol::CloseParens
            | Symbol::OpenBracket
            | Symbol::CloseBracket
            | Symbol::OpenBrace
            | Symbol::CloseBrace => LexLength::Exact(1),

            Symbol::Pipe | Symbol::Tilde | Symbol::Quote => LexLength::Limited(2),

            Symbol::Whitespace(_) | Symbol::Newline | Symbol::Plain(_) => LexLength::Unlimited,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Symbol::Esc => "\\",
            Symbol::Star => "*",
            Symbol::Underline => "_",
            Symbol::Caret => "^",
            Symbol::Tick => "`",
            Symbol::Overline => "‾",
            Symbol::Pipe => "|",
            Symbol::Tilde => "~",
            Symbol::Quote => "\"",
            Symbol::Dollar => "$",
            Symbol::OpenParens => "(",
            Symbol::CloseParens => ")",
            Symbol::OpenBracket => "[",
            Symbol::CloseBracket => "]",
            Symbol::OpenBrace => "{",
            Symbol::CloseBrace => "}",
            Symbol::Plain(content) => content,
            Symbol::Whitespace(literal) => literal,
            Symbol::Newline => "\n",
            // Symbol::Colon => ":",
        }
    }

    fn len(&self) -> usize {
        self.as_str().len()
    }

    /// Checks whether the grapheme is some Unimarkup Inline symbol.
    /// e.g. "*" can be start of Unimarkup Italic or Bold.
    fn is_keyword(&self) -> bool {
        matches!(
            self,
            Symbol::Star
                | Symbol::Underline
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
        )
    }

    /// Checks whether the grapheme is "\".
    fn is_esc(&self) -> bool {
        matches!(self, Symbol::Esc)
    }

    /// Checks whether the grapheme is any of the whitespace characters.
    fn is_whitespace(&self) -> bool {
        matches!(self, Self::Whitespace(_))
    }

    /// Checks whether the grapheme is Unix or Windows style newline.
    fn is_newline(&self) -> bool {
        matches!(self, Self::Newline)
    }

    /// Checks whether the grapheme has any significance in escape sequence.
    /// e.g. The lexer interprets "\ " as a Whitespace `Token`
    fn is_significant_esc(&self) -> bool {
        self.is_whitespace() || self.is_newline()
    }
}

impl<'a> Lexer<'a> {
    /// Creates a [`TokenIterator`] from [`Lexer`].
    ///
    /// [`TokenIterator`]: self::TokenIterator
    /// [`Lexer`]: self::Lexer
    pub fn iter(&self) -> TokenIterator<'a> {
        let skip_lines_upto_index = self.pos.line.saturating_sub(1);
        let mut lines = self.input.lines();

        let curr = lines
            .nth(skip_lines_upto_index)
            .map_or(Vec::default(), |line| Vec::from_iter(line.graphemes(true)));

        TokenIterator {
            lines,
            curr,
            index: self.pos.column.saturating_sub(1),
            pos: self.pos,
        }
    }
}

impl<'a> IntoIterator for &'a Lexer<'a> {
    type Item = Token;

    type IntoIter = TokenIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Enum used for annotating whether the literal content for some [`Symbol`] should be stored into [`Token`] or not.
///
/// [`Symbol`]: self::Symbol
/// [`Token`]: self::token::Token
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Content {
    Store,
    Auto,
}

/// Helper enum for annotation of allowed length for some given [`Symbol`]
///
/// [`Symbol`]: self::Symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LexLength {
    /// Any length allowed.
    Unlimited,
    /// Exact length allowed.
    Exact(usize),
    /// Any length up to the limit.
    Limited(usize),
}

impl From<usize> for LexLength {
    fn from(len: usize) -> Self {
        Self::Exact(len)
    }
}

/// Iterator over Unimarkup [`Token`]s, performs the actual lexing.
///
/// [`Token`]: self::token::Token
#[derive(Debug, Clone)]
pub struct TokenIterator<'a> {
    lines: Lines<'a>,
    curr: Vec<&'a str>,
    index: usize,
    pos: Position, // in input text
}

impl TokenIterator<'_> {
    /// Returns true if no more characters are available in currently observed line.
    fn is_end_of_line(&self) -> bool {
        self.index >= self.curr.len()
    }

    /// Consumes the next line in buffer and sets the cursor to the first grapheme on it.
    ///
    /// Returns false if no new line available.
    fn next_line(&mut self) -> bool {
        // remove last line from cache
        self.curr.clear();

        if let Some(next_line) = self.lines.next() {
            // load next line into cache
            self.curr.extend(next_line.graphemes(true));

            // update index into current line
            self.index = 0;

            self.pos.line += 1;
            self.pos.column = 1;

            true
        } else {
            false
        }
    }

    fn get_symbol(&self, index: usize) -> Option<Symbol> {
        self.curr.get(index).map(Symbol::from)
    }

    /// Lexes a given [`Symbol`] with significance, i.e. `**` and produces a [`Token`] out of it, if possible.
    ///
    /// [`Token`]: self::token::Token
    /// [`Symbol`]: self::Symbol
    fn lex_keyword(&mut self) -> Option<Token> {
        // NOT YET IMPLEMENTED :
        // - ":" -> Custom Emoji, e.g. ::heart::
        // ... and more

        // NOTE: General invariant of lexing:
        // If some literal occurs the maximal symbol length + 1 times, then it's lexed as plain.

        let symbol = self.get_symbol(self.index)?;

        let symbol_len = self.symbol_len(symbol);

        let start_pos = self.pos;
        let end_pos = start_pos + (0, symbol_len - 1);

        let spacing = self.spacing_around(symbol_len);

        let kind = TokenKind::from((symbol, symbol_len));

        let pos = self.index + symbol_len;

        let token = TokenBuilder::new(kind)
            .span(Span::from((start_pos, end_pos)))
            .space(spacing)
            .optional_content(&self.curr[self.index..pos], kind.content_option())
            .build();

        self.index = pos;

        Some(token)
    }

    /// Returns the lexed length of a given [`Symbol`] based on the allowed [`LexLength`]
    /// constraint of the [`Symbol`].
    ///
    /// For [`Symbol`] with exact [`LexLength::Exact`], the length returned is equal or smaller
    /// than that exact length. This behavior is used to temporarily disable lexing invariant.
    ///
    /// The invariant in general is that, for any given [`Symbol`], scanning more symbols than
    /// it's expected to produce a valid [`Token`], the [`TokenKind`] is changed to
    /// [`TokenKind::Plain`] no matter what the first [`Symbol`] implies.
    ///
    /// For example:
    /// - `*` is seen as one [`Symbol::Star`] literals, and is lexed as [`TokenKind::Italic`]
    /// - `**` is seen as two [`Symbol::Star`] literals, and is lexed as [`TokenKind::Bold`]
    /// - `***` is seen as three [`Symbol::Star`] literals, and is lexed as [`TokenKind::ItalicBold`]
    /// - `****` is seen as four [`Symbol::Star`] literals, so it's more than expected and is lexed
    /// as [`TokenKind::Plain`].
    ///
    /// Disabling the invariant is necessary for some [`Token`]s where we want to stop further
    /// scanning soon as one valid [`Token`] is lexed. That is the case for [`Symbol::OpenBracket`].
    /// Consecutive `[` literals are seen as distinct starts of a text group inline format.
    ///
    /// [`Symbol`]: self::Symbol
    /// [`Symbol::Star`]: self::Symbol::Star
    /// [`Symbol::OpenBracket`]: self::Symbol::OpenBracket
    /// [`LexLength`]: self::LexLength
    /// [`LexLength::Exact`]: self::LexLength::Exact
    /// [`Token`]: self::token::Token
    /// [`TokenKind`]: self::token::TokenKind
    /// [`TokenKind::Plain`]: self::token::TokenKind::Plain
    /// [`TokenKind::Italic`]: self::token::TokenKind::Italic
    /// [`TokenKind::Bold`]: self::token::TokenKind::Bold
    /// [`TokenKind::ItalicBold`]: self::token::TokenKind::ItalicBold
    fn symbol_len(&self, symbol: Symbol) -> usize {
        let end_pos = self.literal_end_index(symbol);
        let scanned_len = end_pos - self.index;

        match symbol.allowed_len() {
            LexLength::Exact(len) => scanned_len.min(len),
            _ => scanned_len,
        }
    }

    /// Finds the furthest grapheme in line where, starting from the current cursor position, each grapheme
    /// matches the one provided as the `symbol`.
    ///
    /// Note that the current cursor position will be returned if the grapheme under cursor doesn't match the
    /// `symbol` grapheme provided as the function parameter.
    fn literal_end_index(&self, symbol: Symbol) -> usize {
        let mut pos = self.index;

        loop {
            match self.get_symbol(pos) {
                Some(sym) if sym == symbol => pos += 1,
                _ => break pos,
            }
        }
    }

    /// Calculates the [`Spacing`] just before the cursor position and after cursor position and the given len.
    ///
    /// [`Spacing`]: self::token::Spacing
    fn spacing_around(&self, len: usize) -> Spacing {
        let mut spacing = Spacing::None;

        if self.is_whitespace_at_offs(-1) {
            spacing += Spacing::Pre;
        }
        if self.is_whitespace_at_offs(len as isize) {
            spacing += Spacing::Post;
        }

        spacing
    }

    /// Check if character at cursor position with offset is whitespace.
    fn is_whitespace_at_offs(&self, offset: isize) -> bool {
        let pos = if offset < 0 {
            self.index.saturating_sub(offset.abs() as usize)
        } else {
            self.index.saturating_add(offset as usize)
        };

        self.get_symbol(pos).map_or(false, |ch| ch.is_whitespace())
    }

    /// Lexes a [`Token`] with [`TokenKind::Plain`], so a [`Token`] containing just regular text.
    ///
    /// [`Token`]: self::token::Token
    /// [`TokenKind::Plain`]: self::token::TokenKind::Plain
    fn lex_plain(&mut self) -> Option<Token> {
        let start_pos = self.pos;
        let mut content = String::with_capacity(self.curr.len());

        // multiple cases:
        // 1. got to end of line -> interpret as end of token
        // 2. some keyword found -> end interpretation
        // 3. escape grapheme found -> end interpretation if grapheme is whitespace | newline;
        //    otherwise continue from next character
        // 4. any other grapheme -> consume into plain

        while let Some(symbol) = self.get_symbol(self.index) {
            if symbol.is_keyword() {
                break;
            } else if symbol.is_esc() {
                match self.get_symbol(self.index + 1) {
                    // character can be consumed if not significant in escape sequence
                    Some(symbol) if symbol.is_significant_esc() => break,
                    Some(symbol) => {
                        // consume and skip the symbol in next iteration
                        content.push_str(symbol.as_str());
                        self.index += 2;
                        continue;
                    }
                    None => break,
                }
            } else {
                content.push_str(symbol.as_str());
                self.index += 1;
            }
        }

        // NOTE: index points to the NEXT character, token Span is UP TO that character
        let offset = self.index - self.pos.column;
        let end_pos = self.pos + (0, offset);

        let token = TokenBuilder::new(TokenKind::Plain)
            .with_content(content)
            .span(Span::from((start_pos, end_pos)))
            .space(Spacing::None)
            .build();

        Some(token)
    }

    /// Lexes an escaped [`Symbol`], creating [`Token`] with either [`TokenKind::Plain`] or some
    /// kind of significant escape, such es escaped newline.
    ///
    /// [`Symbol`]: self::Symbol
    /// [`Token`]: self::token::Token
    /// [`TokenKind::Plain`]: self::token::TokenKind::Plain
    fn lex_escape_seq(&mut self) -> Option<Token> {
        let symbol = self.get_symbol(self.index)?;

        // NOTE: index here is pointing to the current grapheme
        let start_pos = self.pos; // escape character
        let end_pos = start_pos + (0, symbol.len());

        let token_kind = if symbol.is_whitespace() {
            TokenKind::Whitespace
        } else {
            TokenKind::Newline
        };

        let token = TokenBuilder::new(token_kind)
            .with_content(String::from(symbol))
            .span(Span::from((start_pos, end_pos)))
            .space(Spacing::None)
            .build();

        self.index += 1;
        Some(token)
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        // NOTE: pos.line is updated only in next_line() function
        self.pos.column = self.index + 1;

        if self.is_end_of_line() && !self.next_line() {
            return None;
        }

        // three cases:
        // 1. next grapheme is keyword -> generate some token
        // 2. next grapheme is '\' -> handle escape sequence
        // 3. next grapheme is not a keyword -> it is plain text

        if let Some(symbol) = self.get_symbol(self.index) {
            if symbol.is_keyword() {
                return self.lex_keyword();
            } else if symbol.is_esc() {
                // Three cases:
                // 1. next character has significance in escape sequence -> some token
                // 2. next character has no significance -> lex as plain text
                // 3. there is no next character. That implies that we've got to end of line, which
                //    implies that the character following '\' is either '\n' or '\r\t' -> lex newline

                match self.get_symbol(self.index + 1) {
                    Some(grapheme) if grapheme.is_significant_esc() => {
                        self.index += 1;
                        return self.lex_escape_seq();
                    }
                    Some(_) => return self.lex_plain(),
                    None => {
                        // is end of line -> newline token!
                        let start_pos = self.pos;
                        let end_pos = start_pos + (0, 1);

                        let token = TokenBuilder::new(TokenKind::Newline)
                            .span(Span::from((start_pos, end_pos)))
                            .space(Spacing::None)
                            .with_content(String::from("\n"))
                            .build();

                        return Some(token);
                    }
                }
            } else {
                return self.lex_plain();
            }
        }

        None
    }
}

#[cfg(test)]
mod tests;
