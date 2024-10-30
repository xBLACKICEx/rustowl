use crate::get_decl_from_local;
use rustc_borrowck::consumers::{BodyWithBorrowckFacts, PoloniusOutput, RichLocation};
use rustc_interface::interface::Compiler;
use rustc_middle::{
    middle::region::ScopeTree,
    mir::{
        BasicBlock, Body, BorrowKind, Local, LocalDecl, Location, Operand, Rvalue, Statement,
        StatementKind,
    },
    ty::{RegionKind, TyKind},
};
use rustc_span::{source_map::SourceMap, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

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
    from: Loc,
    until: Loc,
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
impl From<&LocalDecl<'_>> for Range {
    fn from(decl: &LocalDecl) -> Self {
        let span = decl.source_info.span;
        let range = Self::from(span);
        range
    }
}

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

/*
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Decl {
    User { index: usize },
    Other { index: usize },
}

impl Decl {
    pub fn from_local_decl(region_scope_tree: &ScopeTree, decl: &LocalDecl) {
        let range = Range::from(decl.source_info.span);
        match decl.ty.kind {
            TyKind::Ref(region, ty, m) => match region.kind {
                RegionKind::ReLateParam(region) => {
                    let local = region.scope.as_local().unwrap();
                    region_scope_tree.var_scope(local);}
                _ => {}
            }
            _ => {}
        }
        if decl.is_user_variable() {}

    }
}
*/

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Item {
    Function {
        span: Range,
        mir: MirAnalyzer, //decls: Vec,
    },
}

#[derive(Serialize, Clone, Debug)]
pub struct CollectedData {
    pub items: Vec<Item>,
}

/*
pub(crate) struct DataCollector {
    locals: HashMap<Local, ()>,
}
impl CollectedData {
    pub(crate) fn collect_local(local: Local) {}
}
*/

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
pub struct MirBasicBlock {
    statements: Vec<MirStatement>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Decl {
    User {
        local_index: usize,
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
pub struct MirAnalyzer {
    basic_blocks: Vec<MirBasicBlock>,
    decls: Vec<Decl>,
}
impl MirAnalyzer {
    pub fn analyze<'c, 'tcx, 'a>(
        compiler: &'c Compiler,
        facts: &'a BodyWithBorrowckFacts<'tcx>,
    ) -> Self {
        //let regctx = &facts.region_inference_context;
        let mir = &facts.body;

        let af = &**facts.input_facts.as_ref().unwrap();
        let output = PoloniusOutput::compute(af, FromStr::from_str("Hybrid").unwrap(), true);

        //let output = &**facts.output_facts.as_ref().unwrap();

        //let constraints: Vec<_> = regctx.outlives_constraints().collect();
        let source_map = compiler.sess.source_map();

        // collect basic blocks
        let mut bb_map = HashMap::new();
        for (bb, data) in mir.basic_blocks.iter_enumerated() {
            bb_map.insert(bb, data.clone());
        }

        // collect var live locations
        let mut local_live_locs: HashMap<
            Local,
            (Vec<(BasicBlock, usize)>, Vec<(BasicBlock, usize)>),
        > = HashMap::new();
        let locations = facts.location_table.as_ref().unwrap();
        for (loc_idx, locals) in output.var_live_on_entry.iter() {
            log::info!("var live {:?}", loc_idx);
            let location = locations.to_location(*loc_idx);
            for local in locals {
                if local_live_locs.get(local).is_none() {
                    local_live_locs.insert(*local, (Vec::new(), Vec::new()));
                }
                let (starts, mids) = local_live_locs.get_mut(local).unwrap();
                match location {
                    RichLocation::Start(l) => {
                        starts.push((l.block, l.statement_index));
                    }
                    RichLocation::Mid(l) => {
                        mids.push((l.block, l.statement_index));
                    }
                }
            }
        }
        let mut local_live_spans = HashMap::new();
        for (local, (start, mid)) in local_live_locs.iter_mut() {
            let sort = |v: &mut Vec<(BasicBlock, usize)>| {
                v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
            };
            sort(start);
            sort(mid);
            if local_live_spans.get(local).is_none() {
                local_live_spans.insert(*local, Vec::new());
            }
            for (start, mid) in start.iter().zip(mid.iter()) {
                let start = bb_map
                    .get(&start.0)
                    .map(|bb| bb.statements.get(start.1))
                    .flatten();
                let mid = bb_map
                    .get(&mid.0)
                    .map(|bb| bb.statements.get(mid.1))
                    .flatten();
                match (start, mid) {
                    (Some(start), Some(mid)) => {
                        local_live_spans.get_mut(local).unwrap().push(Range::new(
                            start.source_info.span.lo().0.into(),
                            mid.source_info.span.hi().0.into(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        // collect declared variables
        let mut decls = Vec::new();
        for (local, decl) in mir.local_decls.iter_enumerated() {
            let span = Range::from(decl.source_info.span);
            let local_index = local.index();
            let ty = decl.ty.to_string();
            let lives = local_live_spans.get(&local).cloned();
            if decl.is_user_variable() {
                decls.push(Decl::User {
                    local_index,
                    span,
                    ty,
                    lives,
                });
            } else {
                decls.push(Decl::Other {
                    local_index,
                    ty,
                    lives,
                });
            }
        }

        let mut basic_blocks = Vec::new();
        //let mut lives = HashMap::new();

        for (bb, bb_data) in bb_map.iter() {
            let mut statements = Vec::new();
            for statement in bb_data.statements.iter() {
                if !statement.source_info.span.is_visible(source_map) {
                    continue;
                }
                match &statement.kind {
                    StatementKind::StorageLive(local) => {
                        statements.push(MirStatement::StorageLive {
                            target_local_index: local.index(),
                            range: Range::from(statement.source_info.span),
                        });
                    }
                    StatementKind::StorageDead(local) => {
                        statements.push(MirStatement::StorageDead {
                            target_local_index: local.index(),
                            range: Range::from(statement.source_info.span),
                        })
                    }
                    StatementKind::Assign(ref v) => {
                        let (place, rval) = &**v;
                        let target_local_index = place.local.index();
                        //place.local
                        let rv = match rval {
                            Rvalue::Use(usage) => match usage {
                                Operand::Move(p) => {
                                    let local = p.local;
                                    Some(MirRval::Move {
                                        target_local_index: local.index(),
                                        range: Range::from(statement.source_info.span),
                                    })
                                }
                                _ => None,
                            },
                            Rvalue::Ref(region, kind, place) => {
                                let mutable = match kind {
                                    BorrowKind::Mut { .. } => true,
                                    _ => false,
                                };
                                let regvid = region.as_var();
                                //let regvid = regctx.to_region_vid(*region);
                                //let outlive = regctx.var_infos.get(regvid).map(|v| );

                                /*
                                let regionc: Vec<_> = constraints
                                    .iter()
                                    .filter(|v| {
                                        regctx.eval_equal(v.sup, regvid)
                                            || regctx.eval_equal(v.sub, regvid)
                                    })
                                    .collect();
                                //regionc[0].locations
                                /*
                                    for varinfo in regctx.var_infos.iter() {
                                        varinfo.origin.span()
                                    }
                                    let region_constraints: Vec<_> =
                                        constraints.iter().filter(|v| v.sub == regvid).collect();
                                */
                                let min = regionc.iter().map(|v| v.span.lo().0).min();
                                let max = regionc.iter().map(|v| v.span.hi().0).max();
                                let outlive = match (min, max) {
                                    (Some(min), Some(max)) => {
                                        Some(Range::new(min.into(), max.into()))
                                    }
                                    _ => None,
                                };
                                */
                                let local = place.local;
                                let outlive = None;
                                Some(MirRval::Borrow {
                                    target_local_index: local.index(),
                                    range: Range::from(statement.source_info.span),
                                    mutable,
                                    outlive,
                                })
                            }
                            _ => None,
                        };
                        statements.push(MirStatement::Assign {
                            target_local_index,
                            range: Range::from(statement.source_info.span),
                            rval: rv,
                        });
                    }
                    _ => {}
                }
            }
            /*
            if let Some(term) = bb.terminator {
                term.
            }
            */
            basic_blocks.push(MirBasicBlock { statements });
        }
        Self {
            basic_blocks,
            decls,
        }
    }
}
