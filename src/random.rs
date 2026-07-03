use rand::RngExt;
use rand::rngs::StdRng;

use crate::sip::SipHasher13;
use std::cell::RefCell;
use std::fmt;
use std::hash::BuildHasher;
use std::rc::Rc;

#[derive(Clone)]
pub struct HasherBuilder {
    k0: u64,
    k1: u64,
}

impl HasherBuilder {
    pub fn new(rng: Rc<RefCell<StdRng>>) -> HasherBuilder {
        let (k0, k1) = {
            let mut r = rng.borrow_mut();
            let k0: u64 = r.random();
            let k1: u64 = r.random();
            (k0, k1)
        };
        HasherBuilder { k0, k1 }
    }
}

impl BuildHasher for HasherBuilder {
    type Hasher = SipHasher13;

    fn build_hasher(&self) -> SipHasher13 {
        SipHasher13::new_with_keys(self.k0, self.k1)
    }
}

impl fmt::Debug for HasherBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomState").finish_non_exhaustive()
    }
}
