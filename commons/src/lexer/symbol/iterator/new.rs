use itertools::PeekingNext;

use crate::lexer::{Symbol, SymbolKind};

#[derive(Debug, Clone)]
pub struct SymbolIterator<'input> {
    /// The [`Symbol`] slice the iterator was created for.
    symbols: &'input [Symbol<'input>],
    /// The current index of the iterator inside the [`Symbol`] slice.
    pub(super) index: usize,
    /// The peek index of the iterator inside the [`Symbol`] slice.
    pub(super) peek_index: usize,
}

impl<'input, T> From<T> for SymbolIterator<'input>
where
    T: Into<&'input [Symbol<'input>]>,
{
    fn from(value: T) -> Self {
        SymbolIterator {
            symbols: value.into(),
            index: 0,
            peek_index: 0,
        }
    }
}

impl<'input> Iterator for SymbolIterator<'input> {
    type Item = &'input Symbol<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let symbol = self.symbols.get(self.index)?;

        self.index += 1;
        self.peek_index = self.index;

        Some(symbol)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.max_len()))
    }
}

impl<'input> PeekingNext for SymbolIterator<'input> {
    fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnOnce(&Self::Item) -> bool,
    {
        let symbol = self.symbols.get(self.peek_index).filter(accept)?;
        self.peek_index += 1;
        Some(symbol)
    }
}

impl<'input> SymbolIterator<'input> {
    /// Returns the maximum length of the remaining [`Symbol`]s this iterator might return.
    ///
    /// **Note:** This length does not consider parent iterators, or matching functions.
    /// Therefore, the returned number of [`Symbol`]s might differ, but cannot be larger than this length.
    pub fn max_len(&self) -> usize {
        if self.symbols.is_empty() {
            return 0;
        }

        self.symbols[self.index.min(self.symbols.len() - 1)..].len()
    }

    /// Returns `true` if no more [`Symbol`]s are available.
    pub fn is_empty(&self) -> bool {
        self.max_len() == 0
    }

    /// Returns the current index this iterator is in the [`Symbol`] slice of the root iterator.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Sets the current index of this iterator to the given index.
    pub(crate) fn set_index(&mut self, index: usize) {
        debug_assert!(self.index <= index, "Tried to move iterator backwards.");

        self.index = index;
        self.peek_index = index;
    }

    /// Returns the index used to peek.
    pub(crate) fn peek_index(&self) -> usize {
        self.peek_index
    }

    /// Sets the peek index of this iterator to the given index.
    pub(crate) fn set_peek_index(&mut self, index: usize) {
        if self.index() <= index {
            self.peek_index = index;
        }
    }

    pub fn reset_peek(&mut self) {
        self.set_peek_index(self.index());
    }

    /// Returns the next [`Symbol`] without changing the current index.    
    pub fn peek(&mut self) -> Option<&'input Symbol<'input>> {
        self.symbols.get(self.peek_index)
    }

    /// Returns the [`SymbolKind`] of the peeked [`Symbol`].
    pub fn peek_kind(&mut self) -> Option<SymbolKind> {
        self.peek().map(|s| s.kind)
    }
}
