// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct shrinky_pointer {
  i: @@mut int,
}

#[unsafe_destructor]
impl Drop for shrinky_pointer {
    fn drop(&mut self) {
        error2!("Hello!"); **(self.i) -= 1;
    }
}

impl shrinky_pointer {
    pub fn look_at(&self) -> int { return **(self.i); }
}

fn shrinky_pointer(i: @@mut int) -> shrinky_pointer {
    shrinky_pointer {
        i: i
    }
}

pub fn main() {
    let my_total = @@mut 10;
    { let pt = shrinky_pointer(my_total); assert!((pt.look_at() == 10)); }
    error2!("my_total = {}", **my_total);
    assert_eq!(**my_total, 9);
}
