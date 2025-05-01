#![allow(unused)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FnLocal {
    pub id: u32,
    pub fn_id: u32,
}

impl FnLocal {
    pub fn new(id: u32, fn_id: u32) -> Self {
        Self { id, fn_id }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(transparent)]
pub struct Loc(pub u32);
impl Loc {
    pub fn new(source: &str, byte_pos: u32, offset: u32) -> Self {
        let byte_pos = byte_pos.saturating_sub(offset);
        for (i, (byte, _)) in source.char_indices().enumerate() {
            if byte_pos <= byte as u32 {
                return Self(i as u32);
            }
        }
        Self(0)
    }
}
impl std::ops::Add<i32> for Loc {
    type Output = Loc;
    fn add(self, rhs: i32) -> Self::Output {
        if rhs < 0 && (self.0 as i32) < -rhs {
            Loc(0)
        } else {
            Loc(self.0 + rhs as u32)
        }
    }
}
impl std::ops::Sub<i32> for Loc {
    type Output = Loc;
    fn sub(self, rhs: i32) -> Self::Output {
        if 0 < rhs && (self.0 as i32) < rhs {
            Loc(0)
        } else {
            Loc(self.0 - rhs as u32)
        }
    }
}
impl From<u32> for Loc {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<Loc> for u32 {
    fn from(value: Loc) -> Self {
        value.0
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range {
    from: Loc,
    until: Loc,
}
impl Range {
    pub fn new(from: Loc, until: Loc) -> Option<Self> {
        if until.0 <= from.0 {
            None
        } else {
            Some(Self { from, until })
        }
    }
    pub fn from(&self) -> Loc {
        self.from
    }
    pub fn until(&self) -> Loc {
        self.until
    }
    pub fn size(&self) -> u32 {
        self.until.0 - self.from.0
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirVariable {
    User {
        index: u32,
        live: Range,
        dead: Range,
    },
    Other {
        index: u32,
        live: Range,
        dead: Range,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct MirVariables(HashMap<u32, MirVariable>);
impl Default for MirVariables {
    fn default() -> Self {
        Self::new()
    }
}
impl MirVariables {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn push(&mut self, var: MirVariable) {
        match &var {
            MirVariable::User { index, .. } => {
                if !self.0.contains_key(index) {
                    self.0.insert(*index, var);
                }
            }
            MirVariable::Other { index, .. } => {
                if !self.0.contains_key(index) {
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
    Function { span: Range, mir: Function },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub items: Vec<Function>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Workspace(pub HashMap<String, Crate>);
impl Workspace {
    pub fn merge(&mut self, other: Self) {
        let Workspace(crates) = other;
        for (name, krate) in crates {
            if let Some(insert) = self.0.get_mut(&name) {
                insert.merge(krate);
            } else {
                self.0.insert(name, krate);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Crate(pub HashMap<String, File>);
impl Crate {
    pub fn merge(&mut self, other: Self) {
        let Crate(files) = other;
        for (file, mir) in files {
            if let Some(insert) = self.0.get_mut(&file) {
                insert.items.extend_from_slice(&mir.items);
                insert.items.dedup_by(|a, b| a.fn_id == b.fn_id);
            } else {
                self.0.insert(file, mir);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirRval {
    Move {
        target_local: FnLocal,
        range: Range,
    },
    Borrow {
        target_local: FnLocal,
        range: Range,
        mutable: bool,
        outlive: Option<Range>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirStatement {
    StorageLive {
        target_local: FnLocal,
        range: Range,
    },
    StorageDead {
        target_local: FnLocal,
        range: Range,
    },
    Assign {
        target_local: FnLocal,
        range: Range,
        rval: Option<MirRval>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MirTerminator {
    Drop {
        local: FnLocal,
        range: Range,
    },
    Call {
        destination_local: FnLocal,
        fn_span: Range,
    },
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MirBasicBlock {
    pub statements: Vec<MirStatement>,
    pub terminator: Option<MirTerminator>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MirDecl {
    User {
        local: FnLocal,
        name: String,
        span: Range,
        ty: String,
        lives: Vec<Range>,
        shared_borrow: Vec<Range>,
        mutable_borrow: Vec<Range>,
        drop: bool,
        drop_range: Vec<Range>,
        must_live_at: Vec<Range>,
    },
    Other {
        local: FnLocal,
        ty: String,
        lives: Vec<Range>,
        shared_borrow: Vec<Range>,
        mutable_borrow: Vec<Range>,
        drop: bool,
        drop_range: Vec<Range>,
        must_live_at: Vec<Range>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Function {
    pub fn_id: u32,
    pub basic_blocks: Vec<MirBasicBlock>,
    pub decls: Vec<MirDecl>,
}
