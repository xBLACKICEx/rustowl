/*
 * This code is cited from Rust compiler rustc.
 *

Copyright (c) The Rust Project Contributors

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/

use rustc_borrowck::consumers::RichLocation;
use rustc_index::IndexVec;
use rustc_middle::mir::{BasicBlock, Body, Location};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocationIndex(usize);
impl LocationIndex {
    fn is_start(self) -> bool {
        (self.0 % 2) == 0
    }
}
impl From<usize> for LocationIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// https://github.com/rust-lang/rust/blob/759e07f063fb8e6306ff1bdaeb70af56a878b415/compiler/rustc_borrowck/src/location.rs
pub struct LocationTableSim {
    _num_points: usize,
    statements_before_block: IndexVec<BasicBlock, usize>,
}
impl LocationTableSim {
    pub fn new(body: &Body<'_>) -> Self {
        let mut num_points = 0;
        let statements_before_block = body
            .basic_blocks
            .iter()
            .map(|block_data| {
                let v = num_points;
                num_points += (block_data.statements.len() + 1) * 2;
                v
            })
            .collect();

        Self {
            _num_points: num_points,
            statements_before_block,
        }
    }
    pub fn to_location(&self, index: LocationIndex) -> RichLocation {
        let point_index = index.0;
        let (block, &first_index) = self
            .statements_before_block
            .iter_enumerated()
            .rfind(|&(_, &first_index)| first_index <= point_index)
            .unwrap();

        let statement_index = (point_index - first_index) / 2;
        if index.is_start() {
            RichLocation::Start(Location {
                block,
                statement_index,
            })
        } else {
            RichLocation::Mid(Location {
                block,
                statement_index,
            })
        }
    }
}
