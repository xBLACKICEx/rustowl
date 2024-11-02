use rustc_span::{source_map::SourceMap, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Error {
    SyntaxError,
    UnknownError,
    LocalDeclareNotFound,
    LocalIsNotUserVariable,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(transparent)]
pub struct Loc(u32);
impl From<u32> for Loc {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct Range {
    pub from: Loc,
    pub until: Loc,
}
impl Range {
    pub fn new(from: Loc, until: Loc) -> Self {
        Self { from, until }
    }
}
impl From<Span> for Range {
    fn from(span: Span) -> Self {
        Self::new(span.lo().0.into(), span.hi().0.into())
    }
}

/*
impl From<&LocalDecl<'_>> for Range {
    fn from(decl: &LocalDecl) -> Self {
        let span = decl.source_info.span;
        let range = Self::from(span);
        range
    }
}
*/

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirVariable {
    User {
        index: usize,
        live: Range,
        dead: Range,
    },
    Other {
        index: usize,
        live: Range,
        dead: Range,
    },
}

#[derive(Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct MirVariables(HashMap<usize, MirVariable>);
impl MirVariables {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn push(&mut self, var: MirVariable) {
        match &var {
            MirVariable::User { index, .. } => {
                if self.0.get(index).is_none() {
                    self.0.insert(*index, var);
                }
            }
            MirVariable::Other { index, .. } => {
                if self.0.get(index).is_none() {
                    self.0.insert(*index, var);
                }
            }
        }
    }
    pub fn to_vec(self) -> Vec<MirVariable> {
        self.0.into_values().collect()
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Item {
    Function { span: Range, mir: AnalyzedMir },
}

#[derive(Serialize, Clone, Debug)]
pub struct CollectedData {
    pub items: Vec<Item>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Region {}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirRval {
    Move {
        target_local_index: usize,
        range: Range,
    },
    Borrow {
        target_local_index: usize,
        range: Range,
        mutable: bool,
        outlive: Option<Range>,
    },
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirStatement {
    StorageLive {
        target_local_index: usize,
        range: Range,
    },
    StorageDead {
        target_local_index: usize,
        range: Range,
    },
    Assign {
        target_local_index: usize,
        range: Range,
        rval: Option<MirRval>,
    },
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirTerminator {
    Drop { local_index: usize, range: Range },
    Other,
}
#[derive(Serialize, Clone, Debug)]
pub struct MirBasicBlock {
    pub statements: Vec<MirStatement>,
    pub terminator: Option<MirTerminator>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Decl {
    User {
        local_index: usize,
        name: String,
        span: Range,
        ty: String,
        lives: Option<Vec<Range>>,
    },
    Other {
        local_index: usize,
        ty: String,
        lives: Option<Vec<Range>>,
    },
}
#[derive(Serialize, Clone, Debug)]
pub struct AnalyzedMir {
    pub basic_blocks: Vec<MirBasicBlock>,
    pub decls: Vec<Decl>,
}
