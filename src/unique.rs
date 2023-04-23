use rand::Rng;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueId {
    _unique: i128,
}

impl UniqueId {
    /// A randomly generated unique identifier. UniqueId's have an approximate probability of collision of one in 1.990×10¹⁴, meaning,
    /// for every 200 trillion UniqueId's you generate, you should expect at least one collision. Although the uniqueness of a UniqueId is
    /// substantially weaker than a GUID (1.844674407×10¹⁹), for nearly all operations on a single system they can be considered Unique
    /// 
    /// In practice, the chance of collision is likely to be much smaller, as each UniqueId can be indexed a further 4 billion times,
    /// this index property is useful for very fast array indexing directly from a UniqueId
    pub fn get() -> UniqueId {
        UniqueId { _unique: Self::_generate_internal() }
    }

    /// Returns a new unique id with an internal index
    pub fn get_with_index(index: usize) -> UniqueId {
        // Indexed id's are negative
        UniqueId { _unique: (-Self::_generate_internal()) | index as i128 }
    }
    
    /// Returns a copy of the UniqueId with an updated index as Some(UniqueId) if the index was changed, None if the UniqueId
    /// already had the given index
    pub fn set_index(&self, index: usize) -> Option<UniqueId> {
        debug_assert!(index < std::u32::MAX as usize);

        if self._is_indexed() {
            unsafe {
                if self.index_unchecked() == index as u32 {
                    None
                } else {
                    Some(UniqueId { _unique: (self._entropy_part() as i128) | index as i128 })
                }
            }
        } else {
            Some(UniqueId { _unique: (-(self._entropy_part() as i128) | index as i128) })
        }
    }

    pub fn index(&self) -> Option<usize> {
        debug_assert!(self._unique.is_negative());
        if self._is_indexed() {
            unsafe {
                Some(self.index_unchecked() as usize)
            }
        } else {
            None
        }
    }
    
    /// Returns a positive random i128 with the bottom 4 bytes zeroed
    pub(in self) fn _generate_internal() -> i128 {
        rand::thread_rng().gen_range(0..i128::MAX) & Self::_entropy_mask()

        // Todo: It would be nice to do batching of several thousand ID's in a separate thread with a compilation option 
    }

    #[inline(always)]
    pub(in self) fn _is_indexed(&self) -> bool {
        self._unique.is_negative()
    }

    #[inline(always)]
    pub(in self) fn _entropy_part(&self) -> u128 {
        (self._unique & Self::_entropy_mask()) as u128
    }

    #[inline(always)]
    pub unsafe fn index_unchecked(&self) -> u32 {
        (self._unique & Self::_index_mask()) as u32
    }

    #[inline(always)]
    const fn _entropy_mask() -> i128 {
        0x7FFF_FFFF_FFFF_FFFF_FFFF_FFFF_0000_0000
    }

    #[inline(always)]
    const fn _index_mask() -> i128 {
        0xFFFF_FFFF
    }
}

impl std::fmt::Debug for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ff = f.debug_struct("UniqueId");
        ff.field("UniqueId", &(self._unique));
        
        if self._is_indexed() {
            ff.field("~~entropy", &(self._entropy_part()));
            ff.field("~~sub_idx", &(unsafe { self.index_unchecked() }));
        }
        ff.finish()
    }
}

impl std::fmt::Display for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self._is_indexed() {
            write!(f, "UniqueId({}:{})", self._entropy_part(), unsafe { self.index_unchecked() })
        } else {
            write!(f, "UniqueId({})", self._entropy_part())
        }
    }
}

impl std::fmt::Binary for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self._unique, f)
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
mod experimental {
    use super::*;

    /// A vector built on UniqueId and Vec for storing uniquely identifiable indexable items
    /// 
    /// Pushing or inserting an item into the UniqueVector returns a new UniqueId that must be used to retrieve the value later.
    /// Alternatively in the case which you already have a UniqueId associated with an item you can insert an item/id pair
    #[derive(Default, Debug, Clone)]
    pub struct UniqueVec<T: Sized> {
        data: Vec<T>,
        uids: Vec<UniqueId>,
    }

    impl<T:Sized> UniqueVec<T> {
        fn push(&mut self, t: T) -> UniqueId {
            let idx = self.data.len();
            let uid = UniqueId::get_with_index(idx);
            self.data.push(t);
            self.uids.push(uid);

            debug_assert!(self.data.len() == self.uids.len());
            uid
        }

        fn pop(&mut self) -> Option<(UniqueId, T)> {
            let uid = self.uids.pop();
            let t = self.data.pop();

            debug_assert!(uid.is_some() == t.is_some());
            debug_assert!(self.data.len() == self.uids.len());

            uid.map(|u| (u, t.unwrap()))
        }

        fn get(&self, uid: UniqueId) -> Option<&T> {
            match uid.index() {
                Some(index) => {
                    if index < self.data.len() {
                        unsafe {  
                            if uid == *self.uids.get_unchecked(index) {
                                Some(self.data.get_unchecked(index))
                            } else {
                                // Mismatched identifiers
                                None
                            }
                        }
                    } else {
                        // Index out of range
                        None
                    }
                },
                // Non-indexable UID
                None => None,
            }
        }

        fn get_mut(&mut self, uid: UniqueId) -> Option<&mut T> {
            match uid.index() {
                Some(index) => {
                    if index < self.data.len() {
                        unsafe {  
                            if uid == *self.uids.get_unchecked(index) {
                                Some(self.data.get_unchecked_mut(index))
                            } else {
                                // Mismatched identifiers
                                None
                            }
                        }
                    } else {
                        // Index out of range
                        None
                    }
                },
                // Non-indexable UID
                None => None,
            }
        }

        /// Inserts an item T into the UniqueVector with a given indexed UniqueId, 
        fn insert_remove(&mut self, uid: UniqueId, t: T) -> Result<(UniqueId, T), UniqueVecResult<T>> {
            todo!()
        }
    }

    impl<T> IntoIterator for UniqueVec<T> {
        type Item = T;
        type IntoIter = std::vec::IntoIter<T>;

        fn into_iter(self) -> Self::IntoIter {
            self.data.into_iter()
        }
    }

    impl<'a, T> IntoIterator for &'a mut UniqueVec<T> {
        type Item = &'a mut T;
        type IntoIter = std::slice::IterMut<'a, T>;

        fn into_iter(self) -> Self::IntoIter {
            self.data.iter_mut()
        }
    }

    enum UniqueVecResult<T> {
        /// Result when an item is inserted 
        NonIndexedInsertion(UniqueId, T),
        InsertOutOfRange(UniqueId, T),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_id() {
        const COUNT: usize = 1000usize; // Performs COUNT^2 comparisons
        let mut uids = Vec::new();
        
        for _  in 0..COUNT {
            uids.push(UniqueId::get());
        }
        
        // Not a good test for uniqueness, but serves as a litmus that something isn't catastrophically wrong
        for (idx, id) in uids.iter().enumerate() {
            for (idx2, id2) in uids.iter().enumerate() {
                if idx != idx2 {
                    assert!(id != id2)
                }
            }
        }
    }
}
