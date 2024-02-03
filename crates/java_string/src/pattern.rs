use crate::{JavaCodePoint, JavaStr};

mod private_pattern {
    use crate::{JavaCodePoint, JavaStr};

    pub trait Sealed {}

    impl Sealed for char {}
    impl Sealed for JavaCodePoint {}
    impl Sealed for &str {}
    impl Sealed for &JavaStr {}
    impl<F> Sealed for F where F: FnMut(JavaCodePoint) -> bool {}
    impl Sealed for &[char] {}
    impl Sealed for &[JavaCodePoint] {}
    impl Sealed for &char {}
    impl Sealed for &JavaCodePoint {}
    impl Sealed for &&str {}
    impl Sealed for &&JavaStr {}
}

/// # Safety
///
/// Methods in this trait must only return indexes that are on char boundaries
pub unsafe trait JavaStrPattern: private_pattern::Sealed {
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize>;
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize>;
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)>;
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)>;
}

unsafe impl JavaStrPattern for char {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next()?;
        (ch == *self).then(|| ch.len_utf8())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next_back()?;
        (ch == *self).then(|| ch.len_utf8())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut encoded = [0; 4];
        let encoded = self.encode_utf8(&mut encoded).as_bytes();
        find(haystack.as_bytes(), encoded).map(|index| (index, encoded.len()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut encoded = [0; 4];
        let encoded = self.encode_utf8(&mut encoded).as_bytes();
        rfind(haystack.as_bytes(), encoded).map(|index| (index, encoded.len()))
    }
}

unsafe impl JavaStrPattern for JavaCodePoint {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next()?;
        (ch == *self).then(|| ch.len_utf8())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next_back()?;
        (ch == *self).then(|| ch.len_utf8())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut encoded = [0; 4];
        let encoded = self.encode_semi_utf8(&mut encoded);
        find(haystack.as_bytes(), encoded).map(|index| (index, encoded.len()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut encoded = [0; 4];
        let encoded = self.encode_semi_utf8(&mut encoded);
        rfind(haystack.as_bytes(), encoded).map(|index| (index, encoded.len()))
    }
}

unsafe impl JavaStrPattern for &str {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        haystack
            .as_bytes()
            .starts_with(self.as_bytes())
            .then_some(self.len())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        haystack
            .as_bytes()
            .ends_with(self.as_bytes())
            .then_some(self.len())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        find(haystack.as_bytes(), self.as_bytes()).map(|index| (index, self.len()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        rfind(haystack.as_bytes(), self.as_bytes()).map(|index| (index, self.len()))
    }
}

unsafe impl JavaStrPattern for &JavaStr {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        haystack
            .as_bytes()
            .starts_with(self.as_bytes())
            .then(|| self.len())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        haystack
            .as_bytes()
            .ends_with(self.as_bytes())
            .then(|| self.len())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        find(haystack.as_bytes(), self.as_bytes()).map(|index| (index, self.len()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        rfind(haystack.as_bytes(), self.as_bytes()).map(|index| (index, self.len()))
    }
}

unsafe impl<F> JavaStrPattern for F
where
    F: FnMut(JavaCodePoint) -> bool,
{
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next()?;
        self(ch).then(|| ch.len_utf8())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next_back()?;
        self(ch).then(|| ch.len_utf8())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .find(|(_, ch)| self(*ch))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .rfind(|(_, ch)| self(*ch))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }
}

unsafe impl JavaStrPattern for &[char] {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next()?;
        self.iter().any(|c| ch == *c).then(|| ch.len_utf8())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next_back()?;
        self.iter().any(|c| ch == *c).then(|| ch.len_utf8())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .find(|(_, ch)| self.iter().any(|c| *ch == *c))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .rfind(|(_, ch)| self.iter().any(|c| *ch == *c))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }
}

unsafe impl JavaStrPattern for &[JavaCodePoint] {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next()?;
        self.contains(&ch).then(|| ch.len_utf8())
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let ch = haystack.chars().next_back()?;
        self.contains(&ch).then(|| ch.len_utf8())
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .find(|(_, ch)| self.contains(ch))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        haystack
            .char_indices()
            .rfind(|(_, ch)| self.contains(ch))
            .map(|(index, ch)| (index, ch.len_utf8()))
    }
}

unsafe impl JavaStrPattern for &char {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut ch = **self;
        ch.prefix_len_in(haystack)
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut ch = **self;
        ch.suffix_len_in(haystack)
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut ch = **self;
        ch.find_in(haystack)
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut ch = **self;
        ch.rfind_in(haystack)
    }
}

unsafe impl JavaStrPattern for &JavaCodePoint {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut ch = **self;
        ch.prefix_len_in(haystack)
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut ch = **self;
        ch.suffix_len_in(haystack)
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut ch = **self;
        ch.find_in(haystack)
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut ch = **self;
        ch.rfind_in(haystack)
    }
}

unsafe impl JavaStrPattern for &&str {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut str = **self;
        str.prefix_len_in(haystack)
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut str = **self;
        str.suffix_len_in(haystack)
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut str = **self;
        str.find_in(haystack)
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut str = **self;
        str.rfind_in(haystack)
    }
}

unsafe impl JavaStrPattern for &&JavaStr {
    #[inline]
    fn prefix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut str = **self;
        str.prefix_len_in(haystack)
    }

    #[inline]
    fn suffix_len_in(&mut self, haystack: &JavaStr) -> Option<usize> {
        let mut str = **self;
        str.suffix_len_in(haystack)
    }

    #[inline]
    fn find_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut str = **self;
        str.find_in(haystack)
    }

    #[inline]
    fn rfind_in(&mut self, haystack: &JavaStr) -> Option<(usize, usize)> {
        let mut str = **self;
        str.rfind_in(haystack)
    }
}

#[inline]
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[inline]
fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}
