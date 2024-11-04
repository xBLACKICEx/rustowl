use crate::models::*;
use crate::AnalyzedMir;
use rustc_borrowck::consumers::{
    BodyWithBorrowckFacts, LocationTable, PoloniusOutput, RichLocation,
};
use rustc_interface::interface::Compiler;
use rustc_middle::{
    mir::{
        BasicBlock, BasicBlockData, Body, BorrowKind, Local, LocalDecl, Location, Operand, Rvalue,
        Statement, StatementKind, TerminatorKind, VarDebugInfoContents,
    },
    ty::{RegionKind, TyKind},
};
use rustc_span::Span;
use std::collections::HashMap;
use std::str::FromStr;

pub struct MirAnalyzer<'c, 'tcx> {
    compiler: &'c Compiler,
    location_table: &'c LocationTable,
    body: Body<'tcx>,
    output: PoloniusOutput,
    bb_map: HashMap<BasicBlock, BasicBlockData<'tcx>>,
}
impl<'c, 'tcx> MirAnalyzer<'c, 'tcx> {
    /// initialize analyzer
    pub fn new(compiler: &'c Compiler, facts: &'c BodyWithBorrowckFacts<'tcx>) -> Self {
        let af = &**facts.input_facts.as_ref().unwrap();
        let location_table = facts.location_table.as_ref().unwrap();
        let body = facts.body.clone();
        log::info!("start re-computing borrow check with dump: true");
        let output = PoloniusOutput::compute(af, FromStr::from_str("Hybrid").unwrap(), true);
        log::info!("borrow check finished");

        // build basic blocks map
        let bb_map = body
            .basic_blocks
            .iter_enumerated()
            .map(|(b, d)| (b, d.clone()))
            .collect();
        Self {
            compiler,
            location_table,
            body,
            output,
            bb_map,
        }
    }

    fn sort_locs(v: &mut Vec<(BasicBlock, usize)>) {
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    }

    /// obtain map from local id to living range
    fn lives(&self) -> HashMap<Local, Vec<Range>> {
        let mut local_live_locs = HashMap::new();
        for (loc_idx, locals) in self.output.var_live_on_entry.iter() {
            let location = self.location_table.to_location(*loc_idx);
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
        HashMap::from_iter(
            local_live_locs
                .into_iter()
                .map(|(local, (mut start, mut mid))| {
                    Self::sort_locs(&mut start);
                    Self::sort_locs(&mut mid);
                    //for (start, mid) in start.iter().zip(mid.iter()) {
                    (
                        local,
                        start
                            .iter()
                            .zip(mid.iter())
                            .filter_map(|(start, mid)| {
                                let start = self
                                    .bb_map
                                    .get(&start.0)
                                    .map(|bb| bb.statements.get(start.1))
                                    .flatten();
                                let mid = self
                                    .bb_map
                                    .get(&mid.0)
                                    .map(|bb| bb.statements.get(mid.1))
                                    .flatten();
                                match (start, mid) {
                                    (Some(start), Some(mid)) => Some(Range::new(
                                        start.source_info.span.lo().0.into(),
                                        mid.source_info.span.hi().0.into(),
                                    )),
                                    _ => None,
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .into_iter(),
        )
    }

    /// collect user defined variables from debug info in MIR
    fn collect_user_vars(&self) -> HashMap<Local, (Span, String)> {
        self.body
            .var_debug_info
            .iter()
            .filter_map(|debug| match &debug.value {
                VarDebugInfoContents::Place(place) => Some((
                    place.local,
                    (debug.source_info.span, debug.name.as_str().to_owned()),
                )),
                _ => None,
            })
            .collect()
    }
    /// collect declared variables in MIR body
    fn collect_decls(&self) -> Vec<MirDecl> {
        let user_vars = self.collect_user_vars();
        let lives = self.lives();
        self.body
            .local_decls
            .iter_enumerated()
            .map(|(local, decl)| {
                let local_index = local.index();
                let ty = decl.ty.to_string();
                let lives = lives.get(&local).cloned();
                if decl.is_user_variable() {
                    let (span, name) = user_vars.get(&local).cloned().unwrap();
                    MirDecl::User {
                        local_index,
                        name,
                        span: Range::from(span),
                        ty,
                        lives,
                    }
                } else {
                    MirDecl::Other {
                        local_index,
                        ty,
                        lives,
                    }
                }
            })
            .collect()
    }

    /// collect and translate basic blocks
    fn basic_blocks(&self) -> Vec<MirBasicBlock> {
        self.bb_map
            .iter()
            .map(|(_bb, bb_data)| {
                let statements = bb_data
                    .statements
                    .iter()
                    .filter_map(|statement| {
                        if !statement
                            .source_info
                            .span
                            .is_visible(self.compiler.sess.source_map())
                        {
                            return None;
                        }
                        match &statement.kind {
                            StatementKind::StorageLive(local) => Some(MirStatement::StorageLive {
                                target_local_index: local.index(),
                                range: Range::from(statement.source_info.span),
                            }),
                            StatementKind::StorageDead(local) => Some(MirStatement::StorageDead {
                                target_local_index: local.index(),
                                range: Range::from(statement.source_info.span),
                            }),
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
                                Some(MirStatement::Assign {
                                    target_local_index,
                                    range: Range::from(statement.source_info.span),
                                    rval: rv,
                                })
                            }
                            _ => None,
                        }
                    })
                    .collect();
                let terminator =
                    bb_data
                        .terminator
                        .as_ref()
                        .map(|terminator| match &terminator.kind {
                            TerminatorKind::Drop { place, .. } => MirTerminator::Drop {
                                local_index: place.local.index(),
                                range: terminator.source_info.span.into(),
                            },
                            TerminatorKind::Call {
                                func,
                                args,
                                destination,
                                target,
                                unwind,
                                call_source,
                                fn_span,
                            } => MirTerminator::Call {
                                destination_local_index: destination.local.as_usize(),
                                fn_span: (*fn_span).into(),
                            },
                            _ => MirTerminator::Other,
                        });
                MirBasicBlock {
                    statements,
                    terminator,
                }
            })
            .collect()
    }

    /// analyze MIR to get JSON-serializable, TypeScript friendly representation
    pub fn analyze<'a>(&mut self) -> AnalyzedMir {
        let decls = self.collect_decls();
        let basic_blocks = self.basic_blocks();
        //let mut lives = HashMap::new();

        //for (locidx, borrows) in output.errors.iter() {}
        AnalyzedMir {
            basic_blocks,
            decls,
        }
    }
}
