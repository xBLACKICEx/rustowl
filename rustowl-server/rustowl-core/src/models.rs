use rustc_span::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Error {
    SyntaxError,
    UnknownError,
    LocalDeclareNotFound,
    LocalIsNotUserVariable,
}

/// location in source code
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(transparent)]
pub struct Loc(u32);
impl From<u32> for Loc {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// represents range in source code
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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

/// variable in MIR
#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Item {
    Function { span: Range, mir: AnalyzedMir },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Workspace(pub HashMap<String, File>);

#[derive(Serialize, Deserialize, Clone, Debug)]
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

/// statement in MIR
#[derive(Serialize, Deserialize, Clone, Debug)]
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
/// terminator in MIR
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirTerminator {
    Drop {
        local_index: usize,
        range: Range,
    },
    Call {
        destination_local_index: usize,
        fn_span: Range,
    },
    Other,
}
/// basic block in MIR
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MirBasicBlock {
    pub statements: Vec<MirStatement>,
    pub terminator: Option<MirTerminator>,
}

/// declared variable in MIR
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirDecl {
    User {
        local_index: usize,
        name: String,
        span: Range,
        ty: String,
        lives: Vec<Range>,
        drop: bool,
        drop_range: Vec<Range>,
        must_live_at: Vec<Range>,
    },
    Other {
        local_index: usize,
        ty: String,
        lives: Vec<Range>,
        drop: bool,
        drop_range: Vec<Range>,
        must_live_at: Vec<Range>,
    },
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnalyzedMir {
    pub basic_blocks: Vec<MirBasicBlock>,
    pub decls: Vec<MirDecl>,
}
