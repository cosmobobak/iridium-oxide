pub trait BitIterable {
    fn iter_bits(self) -> IterBits;
}

impl BitIterable for u8 {
    fn iter_bits(self) -> IterBits {
        IterBits {
            bitboard: u64::from(self),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterBits {
    bitboard: u64,
}

impl Iterator for IterBits {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.bitboard == 0 {
            None
        } else {
            let bit = self.bitboard.trailing_zeros();
            self.bitboard &= self.bitboard - 1;
            Some(bit as usize)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.bitboard.count_ones();
        (count as usize, Some(count as usize))
    }
}
