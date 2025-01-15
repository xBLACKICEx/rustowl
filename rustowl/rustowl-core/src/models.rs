use rustc_span::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
impl std::ops::Sub for Loc {
    type Output = Loc;
    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 < rhs.0 {
            0.into()
        } else {
            Loc::from(self.0 - rhs.0)
        }
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
    /*
    pub fn from_source_info(body: &Body<'_>, source_info: SourceInfo) -> Self {
        let scope = Range::from(body.source_scopes.get(source_info.scope).unwrap().span);
        let wide = Range::from(source_info.span);
        Range::new(
            Loc::from(wide.from - scope.from),
            Loc::from(wide.until - scope.from),
        )
    }
    */
    pub fn offset(self, offset: u32) -> Self {
        Self {
            from: self.from - Loc::from(offset),
            until: self.until - Loc::from(offset),
        }
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
    pub items: Vec<AnalyzedMir>,
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
