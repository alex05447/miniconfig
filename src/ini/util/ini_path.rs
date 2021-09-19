use {crate::*, std::iter::Iterator};

/// Path of a nested `.ini` section path - either borrowed directly from the source string,
/// or owned and contained in a helper buffer
/// (i.e. the parsed string contained at least one escape sequence and thus could not be borrowed directly).
enum IniPathPart<'s> {
    /// The path string is borrowed from the `.ini` source.
    Borrowed(&'s NonEmptyStr),
    /// The path string is owned and is contained in a helper buffer.
    /// Contains the (non-inclusive) byte range of the path string in the helper buffer.
    Owned(std::ops::Range<usize>),
}

/// A simple wrapper around the nested `.ini` section path, used to minimize the number of allocations.
/// Stores owned section names in the contiguous local buffer.
pub(crate) struct IniPath<'s> {
    /// Helper buffer for owned section names.
    buffer: String,
    /// Nested section path parts. Contains at most one entry if we don't support nested sections.
    parts: Vec<IniPathPart<'s>>,
}

impl<'s> IniPath<'s> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
            parts: Vec::new(),
        }
    }

    /// Pushes a new section name to the end of the path.
    /// If the section name is owned, appends it to the local buffer.
    pub(crate) fn push(&mut self, section: NonEmptyIniStr<'s, '_>) {
        use NonEmptyIniStr::*;

        match section {
            Borrowed(section) => {
                self.parts.push(IniPathPart::Borrowed(section));
            }
            Owned(section) => {
                let current_len = self.buffer.len();
                let offset = current_len + section.as_str().len();

                self.buffer.push_str(section.as_str());
                self.parts.push(IniPathPart::Owned(current_len..offset));
            }
        }
    }

    /// Pops a section name off the end of the path.
    /// NOTE - the caller guarantees that the path is not empty.
    pub(crate) fn pop(&mut self) {
        debug_assert!(!self.parts.is_empty(), "tried to pop an empty `.ini` path");

        self.parts.pop();

        if let Some(last) = self.parts.last() {
            if let IniPathPart::Owned(offset) = last {
                debug_assert!(offset.end > 0);
                self.buffer.truncate(offset.end);
            }
        }
    }

    /// Returns the last section name, if any.
    pub(crate) fn last(&self) -> Option<NonEmptyIniStr<'s, '_>> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.slice(self.len() - 1) })
        }
    }

    /// Returns the number of section names in the path.
    pub(crate) fn len(&self) -> u32 {
        self.parts.len() as _
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over nested section path parts, parent to child.
    pub(crate) fn iter(&self) -> impl Iterator<Item = NonEmptyIniStr<'s, '_>> {
        IniPathIter::new(self)
    }

    pub(crate) fn to_config_path(&self) -> IniConfigPath {
        let mut path = IniConfigPath::new();

        for section in self.iter() {
            path.0.push(section.as_ne_str().into());
        }

        path
    }

    /// Returns the section name at `index` in the path.
    /// NOTE - the caller guarantees `index` is valid.
    /// Passing an invalid `index` is UB.
    unsafe fn slice(&self, index: u32) -> NonEmptyIniStr<'s, '_> {
        use IniPathPart::*;

        debug_assert!(index < self.len());

        match self.parts.get_unchecked(index as usize) {
            Owned(range) => {
                debug_assert!(range.end > 0);
                debug_assert!(range.start < range.end);

                NonEmptyIniStr::Owned(unwrap_unchecked_msg(
                    NonEmptyStr::new(self.buffer.get_unchecked(range.start as _..range.end as _)),
                    "empty section name",
                ))
            }
            Borrowed(part) => NonEmptyIniStr::Borrowed(*part),
        }

        //let end = *(self.offsets.get_unchecked(index as usize)) as _;
    }
}

/// Iterates over the `IniPath` nested section path parts, parent to child.
struct IniPathIter<'a, 's> {
    path: &'a IniPath<'s>,
    index: u32,
}

impl<'a, 's> IniPathIter<'a, 's> {
    fn new(path: &'a IniPath<'s>) -> Self {
        Self { path, index: 0 }
    }
}

impl<'a, 's> std::iter::Iterator for IniPathIter<'a, 's> {
    type Item = NonEmptyIniStr<'s, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.path.len() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(unsafe { self.path.slice(index) })
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, ministr_macro::nestr};

    #[allow(non_snake_case)]
    #[test]
    fn IniPath() {
        let mut path = IniPath::new();

        assert!(path.is_empty());
        assert!(path.len() == 0);

        path.push(NonEmptyIniStr::Owned(nestr!("foo")));

        assert!(!path.is_empty());
        assert!(path.len() == 1);

        assert_eq!(
            unsafe { path.slice(0) },
            NonEmptyIniStr::Owned(nestr!("foo"))
        );

        path.push(NonEmptyIniStr::Owned(nestr!("bill")));

        assert!(!path.is_empty());
        assert!(path.len() == 2);

        assert_eq!(
            unsafe { path.slice(0) },
            NonEmptyIniStr::Owned(nestr!("foo"))
        );
        assert_eq!(
            unsafe { path.slice(1) },
            NonEmptyIniStr::Owned(nestr!("bill"))
        );

        path.push(NonEmptyIniStr::Borrowed(nestr!("bob")));

        assert!(!path.is_empty());
        assert!(path.len() == 3);

        assert_eq!(
            unsafe { path.slice(0) },
            NonEmptyIniStr::Owned(nestr!("foo"))
        );
        assert_eq!(
            unsafe { path.slice(1) },
            NonEmptyIniStr::Owned(nestr!("bill"))
        );
        assert_eq!(
            unsafe { path.slice(2) },
            NonEmptyIniStr::Borrowed(nestr!("bob"))
        );

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, NonEmptyIniStr::Owned(nestr!("foo"))),
                1 => assert_eq!(path_part, NonEmptyIniStr::Owned(nestr!("bill"))),
                2 => assert_eq!(path_part, NonEmptyIniStr::Borrowed(nestr!("bob"))),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(!path.is_empty());
        assert!(path.len() == 2);

        assert_eq!(
            unsafe { path.slice(0) },
            NonEmptyIniStr::Owned(nestr!("foo"))
        );
        assert_eq!(
            unsafe { path.slice(1) },
            NonEmptyIniStr::Owned(nestr!("bill"))
        );

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, NonEmptyIniStr::Owned(nestr!("foo"))),
                1 => assert_eq!(path_part, NonEmptyIniStr::Owned(nestr!("bill"))),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(!path.is_empty());
        assert!(path.len() == 1);

        assert_eq!(
            unsafe { path.slice(0) },
            NonEmptyIniStr::Owned(nestr!("foo"))
        );

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, NonEmptyIniStr::Owned(nestr!("foo"))),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(path.is_empty());
        assert!(path.len() == 0);
    }
}
