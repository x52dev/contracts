/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use contracts::*;

use std::collections::HashSet;

pub struct Library {
    available: HashSet<String>,
    lent: HashSet<String>,
}

impl Library {
    fn book_exists(&self, book_id: &str) -> bool {
        self.available.contains(book_id) || self.lent.contains(book_id)
    }

    #[debug_pre(!self.book_exists(book_id), "Book IDs are unique")]
    #[debug_post(self.available.contains(book_id), "Book now available")]
    #[post(self.available.len() == old(self.available.len()) + 1)]
    #[post(self.lent.len() == old(self.lent.len()), "No lent change")]
    pub fn add_book(&mut self, book_id: &str) {
        self.available.insert(book_id.to_string());
    }

    #[debug_pre(self.book_exists(book_id))]
    #[post(ret ==> self.available.len() == old(self.available.len()) - 1)]
    #[post(ret ==> self.lent.len() == old(self.lent.len()) + 1)]
    #[debug_post(ret ==> self.lent.contains(book_id))]
    #[debug_post(!ret ==> self.lent.contains(book_id), "Book already lent")]
    pub fn lend(&mut self, book_id: &str) -> bool {
        if self.available.contains(book_id) {
            self.available.remove(book_id);
            self.lent.insert(book_id.to_string());
            true
        } else {
            false
        }
    }

    #[debug_pre(self.lent.contains(book_id), "Can't return a non-lent book")]
    #[post(self.lent.len() == old(self.lent.len()) - 1)]
    #[post(self.available.len() == old(self.available.len()) + 1)]
    #[debug_post(!self.lent.contains(book_id))]
    #[debug_post(self.available.contains(book_id), "Book available again")]
    pub fn return_book(&mut self, book_id: &str) {
        self.lent.remove(book_id);
        self.available.insert(book_id.to_string());
    }
}

fn main() {
    let mut lib = Library {
        available: HashSet::new(),
        lent: HashSet::new(),
    };

    lib.add_book("Das Kapital");
    println!("Adding a book.");

    let lent_successful = lib.lend("Das Kapital");
    assert_eq!(lent_successful, true);

    if lent_successful {
        println!("Lent out the book.");
        println!("Reading for a bit...");

        println!("Giving back the book.");

        lib.return_book("Das Kapital");
    }
}
