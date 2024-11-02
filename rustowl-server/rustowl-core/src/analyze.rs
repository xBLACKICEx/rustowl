use crate::get_decl_from_local;
use crate::models::*;
use crate::AnalyzedMir;
use rustc_borrowck::consumers::{BodyWithBorrowckFacts, PoloniusOutput, RichLocation};
use rustc_interface::interface::Compiler;
use rustc_middle::{
    mir::{
        BasicBlock, Body, BorrowKind, Local, LocalDecl, Location, Operand, Rvalue, Statement,
        StatementKind, TerminatorKind, VarDebugInfoContents,
    },
    ty::{RegionKind, TyKind},
};
use std::collections::HashMap;
use std::str::FromStr;

pub struct MirAnalyzer;
impl MirAnalyzer {
    pub fn analyze<'c, 'tcx, 'a>(
        compiler: &'c Compiler,
        facts: &'a BodyWithBorrowckFacts<'tcx>,
    ) -> AnalyzedMir {
        let mir = &facts.body;

        let af = &**facts.input_facts.as_ref().unwrap();
        let output = PoloniusOutput::compute(af, FromStr::from_str("Hybrid").unwrap(), true);

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

        let mut user_vars = HashMap::new();
        for debug in mir.var_debug_info.iter() {
            match &debug.value {
                VarDebugInfoContents::Place(p) => {
                    user_vars.insert(
                        p.local,
                        (debug.source_info.span, debug.name.as_str().to_owned()),
                    );
                }
                _ => {}
            }
        }

        // collect declared variables
        let mut decls = Vec::new();
        for (local, decl) in mir.local_decls.iter_enumerated() {
            //let span = Range::from(decl.source_info.span);
            let local_index = local.index();
            let ty = decl.ty.to_string();
            let lives = local_live_spans.get(&local).cloned();
            if decl.is_user_variable() {
                let (span, name) = user_vars.get(&local).cloned().unwrap();
                decls.push(Decl::User {
                    local_index,
                    name,
                    span: Range::from(span),
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
            let terminator = bb_data
                .terminator
                .as_ref()
                .map(|terminator| match &terminator.kind {
                    TerminatorKind::Drop { place, .. } => MirTerminator::Drop {
                        local_index: place.local.index(),
                        range: terminator.source_info.span.into(),
                    },
                    _ => MirTerminator::Other,
                });
            basic_blocks.push(MirBasicBlock {
                statements,
                terminator,
            });
        }

        for (locidx, borrows) in output.errors.iter() {}
        AnalyzedMir {
            basic_blocks,
            decls,
        }
    }
}
