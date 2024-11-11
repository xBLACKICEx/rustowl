use crate::models::*;
use crate::AnalyzedMir;
use polonius_engine::FactTypes;
use rustc_borrowck::consumers::{
    BodyWithBorrowckFacts, BorrowIndex, LocationTable, PoloniusInput, PoloniusOutput, RichLocation,
    RustcFacts,
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
use std::collections::{BTreeSet, HashMap, LinkedList};
use std::hash::Hash;

type Borrow = <RustcFacts as FactTypes>::Loan;
type Region = <RustcFacts as FactTypes>::Origin;

trait Append<K, V>
where
    K: Hash + Eq + Clone,
{
    fn append(&mut self, key: &K, value: V);
}
impl<K, V> Append<K, V> for HashMap<K, Vec<V>>
where
    K: Hash + Eq + Clone,
{
    fn append(&mut self, key: &K, value: V) {
        if let Some(v) = self.get_mut(key) {
            v.push(value);
        } else {
            self.insert(key.clone(), vec![value]);
        }
    }
}
impl<K, V> Append<K, V> for HashMap<K, BTreeSet<V>>
where
    K: Hash + Eq + Clone,
    V: Ord,
{
    fn append(&mut self, key: &K, value: V) {
        if let Some(v) = self.get_mut(key) {
            v.insert(value);
        } else {
            let mut set = BTreeSet::new();
            set.insert(value);
            self.insert(key.clone(), set);
        }
    }
}

pub struct MirAnalyzer<'c, 'tcx> {
    compiler: &'c Compiler,
    location_table: &'c LocationTable,
    facts: &'c BodyWithBorrowckFacts<'tcx>,
    input: PoloniusInput,
    output_insensitive: PoloniusOutput,
    output_datafrog: PoloniusOutput,
    bb_map: HashMap<BasicBlock, BasicBlockData<'tcx>>,
    //local_loan_live_at: HashMap<Local, Vec<RichLocation>>,
    //local_super_regions: HashMap<Local, Vec<RichLocation>>,
    //region_locations: HashMap<Region, Vec<RichLocation>>,
    local_borrows: HashMap<Local, Vec<BorrowIndex>>,
    borrow_locals: HashMap<Borrow, Local>,
    /*
    local_must_regions: HashMap<Local, BTreeSet<Region>>,
    local_actual_regions: HashMap<Local, BTreeSet<Region>>,
    borrow_regions: HashMap<BorrowIndex, BTreeSet<Region>>, //origin_live_at: HashMap<Region, Locb
    local_regions: HashMap<Local, BTreeSet<Region>>,
    local_actual_locations: HashMap<Local, Vec<RichLocation>>,
    local_borrow_regions: HashMap<Local, BTreeSet<Region>>,
    borrow_local_def: HashMap<BorrowIndex, Local>,
    local_invalid_locations: HashMap<Local, Vec<RichLocation>>,
    */
}
impl<'c, 'tcx> MirAnalyzer<'c, 'tcx> {
    /// initialize analyzer
    pub fn new(compiler: &'c Compiler, facts: &'c BodyWithBorrowckFacts<'tcx>) -> Self {
        let input = *facts.input_facts.as_ref().unwrap().clone();
        let location_table = facts.location_table.as_ref().unwrap();

        // local -> all borrows on that local
        let local_borrows: HashMap<Local, Vec<BorrowIndex>> = HashMap::from_iter(
            facts
                .borrow_set
                .local_map
                .iter()
                .map(|(local, borrow_idc)| {
                    (*local, borrow_idc.iter().map(|v| v.clone()).collect())
                }),
        );
        let mut borrow_locals = HashMap::new();
        for (local, borrow_idc) in local_borrows.iter() {
            for borrow_idx in borrow_idc {
                borrow_locals.insert(*borrow_idx, *local);
            }
        }
        log::info!("start re-computing borrow check with dump: true");
        // compute insensitive
        // it may include invalid region, which can be used at showing wrong region
        let output_insensitive = PoloniusOutput::compute(
            &input,
            polonius_engine::Algorithm::LocationInsensitive,
            true,
        );
        // compute accurate region, which may eliminate invalid region
        let output_datafrog =
            PoloniusOutput::compute(&input, polonius_engine::Algorithm::DatafrogOpt, true);
        log::info!("borrow check finished");

        //let local_loan_live: HashMap<Local, >
        //println!("{:?}", output);
        /*
        let mut local_loan_live_at = HashMap::new();
        for (location_idx, borrow_idc) in output.loan_live_at.iter() {
            let location = location_table.to_location(*location_idx);
            for borrow_idx in borrow_idc {
                if let Some(locals) = borrow_idx_local.get(borrow_idx) {
                    for local_idx in locals {
                        let locations = match local_loan_live_at.get_mut(local_idx) {
                            Some(v) => v,
                            None => {
                                local_loan_live_at.insert(*local_idx, Vec::new());
                                local_loan_live_at.get_mut(local_idx).unwrap()
                            }
                        };
                        locations.push(location);
                    }
                }
            }
        }
        */

        // local's living range in provided source code

        /*
        let mut region_idx_location_idc = HashMap::new();
        for (location_idx, region_idc) in output.origin_live_on_entry.iter(){
            for region_idx in region_idc {
                match region_idc.get_mut(region_idx) {
                    Some(v) => v,
                    None => region_idc.insert(*region_idx)
                }
            }
        }
        */

        // locations that region includes
        /*
        let mut region_locations = HashMap::new();
        let mut region_locations_idc: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (location_idx, region_idc) in output.origin_live_on_entry.iter() {
            for region_idx in region_idc {
                let insert = match region_locations.get_mut(region_idx) {
                    Some(v) => v,
                    None => {
                        region_locations.insert(*region_idx, Vec::new());
                        region_locations.get_mut(region_idx).unwrap()
                    }
                };
                insert.push(location_table.to_location(*location_idx).clone());
                region_locations_idc.append(region_idx, *location_idx);
            }
        }

        // compute regions where the local must be live
        let mut local_must_regions = HashMap::new();
        for (region_idx, borrow_idc) in output.origin_contains_loan_anywhere.iter() {
            for borrow_idx in borrow_idc {
                if let Some(local) = borrow_locals.get(borrow_idx) {
                    local_must_regions.append(local, *region_idx);
                }
            }
        }
        //let mut local_super_regions = HashMap::new();

        let mut local_locations: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (location, locals) in output.var_live_on_entry.iter() {
            for local in locals {
                local_locations.append(local, *location);
            }
        }
        /*
        for (location, locals) in output.var_drop_live_on_entry.iter() {
            for local in locals {
                local_locations.append(local, *location);
            }
        }
        */
        let mut local_regions_may_included: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (local, locations) in local_locations.iter() {
            for location in locations {
                if let Some(regions) = output.origin_live_on_entry.get(location) {
                    for region in regions {
                        local_regions_may_included.append(local, *region);
                    }
                }
            }
        }

        //println!("loan: {:?}", output.loan_live_at);
        //println!("subset_anywhere: {:?}", output.subset_anywhere);

        //for (local, regions) in local_must_regions.iter() {
        /*
        let mut error_subset = Vec::new();
        let mut ok_subset = Vec::new();
        for (outer, inners) in output.subset_anywhere.iter() {
            for inner in inners {
                match (
                    region_locations_idc.get(outer),
                    region_locations_idc.get(inner),
                ) {
                    (Some(o), Some(i)) => if !i.is_subset(o) {
                        error_subset.push((outer, inner));
                    } else {
                        ok_subset
                    },
                    _ => {}
                }
            }
        }
        */
        //}

        let mut region_borrow_issue = HashMap::new();
        let mut borrow_region_issue = HashMap::new();
        for (region, borrow, _) in input.loan_issued_at.iter() {
            region_borrow_issue.insert(*region, *borrow);
            borrow_region_issue.insert(*borrow, *region);
        }
        let mut borrow_issued_at = HashMap::new();
        for (_, borrow, location) in input.loan_issued_at.iter() {
            borrow_issued_at.insert(*borrow, *location);
        }
        let mut local_defined_at = HashMap::new();
        for (local, location) in input.var_defined_at.iter() {
            local_defined_at.insert(*location, *local);
        }
        let mut borrow_local_def = HashMap::new();
        for (borrow, location) in borrow_issued_at.iter() {
            if let Some(local) = local_defined_at.get(location) {
                borrow_local_def.insert(*borrow, *local);
            }
        }

        let mut local_borrow_regions = HashMap::new();
        for (local, borrows) in local_borrows.iter() {
            let regions = borrows
                .iter()
                .filter_map(|v| borrow_region_issue.get(v).cloned())
                .collect();
            local_borrow_regions.insert(*local, regions);
        }
        //println!("{:?}", local_borrow_regions);

        let mut local_live_at = HashMap::new();
        for (location, locals) in output.var_live_on_entry.iter() {
            for local in locals {
                let insert = match local_live_at.get_mut(local) {
                    Some(v) => v,
                    None => {
                        local_live_at.insert(*local, BTreeSet::new());
                        local_live_at.get_mut(local).unwrap()
                    }
                };
                insert.insert(*location);
            }
        }

        // for (location, regions) in output.origin_live_on_entry.iter() {}
        let mut borrow_live = HashMap::new();
        //for (region, location) in region_locations_idc.iter() {
        for (borrow, local) in borrow_local_def.iter() {
            //if let Some(borrow) = region_borrow_issue.get(region) {
            if let Some(locations) = local_live_at.get(local) {
                borrow_live.insert(*borrow, locations.clone());
            }
        }

        let mut local_live_with_borrow = local_live_at.clone();
        for (borrow, borrow_live_at) in borrow_live.iter() {
            if let Some(local) = borrow_locals.get(borrow) {
                if let Some(live) = local_live_with_borrow.get_mut(local) {
                    let union = live.union(borrow_live_at);
                    *live = union.cloned().collect();
                }
            }
        }
        let local_actual_locations =
            HashMap::from_iter(local_live_with_borrow.iter().map(|(local, locations)| {
                (
                    *local,
                    locations
                        .into_iter()
                        .map(|location| location_table.to_location(*location).clone())
                        .collect(),
                )
            }));

        for (location, region) in output.origin_live_on_entry.iter() {}

        /*
        let mut local_regions: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (local, borrows) in local_borrows.iter() {
            for borrow in borrows {
                if let Some(region) = borrow_issue_region.get(borrow) {
                    local_regions.append(local, *region);
                }
            }
        }
        */

        let mut valid_subset = Vec::new();
        for (outer, inners) in output.subset_anywhere.iter() {
            if let Some(outer_locations) = region_locations_idc.get(outer) {
                for inner in inners {
                    if let Some(inner_locations) = region_locations_idc.get(inner) {
                        if inner_locations.is_subset(outer_locations) {
                            valid_subset.push((*outer, *inner));
                        }
                    }
                }
            }
        }

        //println!("{:?}", valid_subset);
        let mut local_regions = HashMap::new();
        for (outer, inner) in valid_subset.iter() {
            if let Some(borrow) = region_borrow_issue.get(outer) {
                if let Some(local) = borrow_locals.get(borrow) {
                    local_regions.append(local, *outer);
                    local_regions.append(local, *inner);
                }
            }
        }

        let mut local_invalid: HashMap<_, Vec<_>> = HashMap::new();
        /*
        for (location, region_borrows) in output.origin_contains_loan_at.iter() {
            for (region, borrows) in region_borrows {
                for borrow in borrows {
                    if let Some(local) = borrow_locals.get(borrow) {
                        local_invalid.append(local, *location);
                    }
                }
            }
        }
        */
        for (location, borrows) in output.loan_live_at.iter() {
            for borrow in borrows {
                if let Some(local) = borrow_locals.get(borrow) {
                    local_invalid.append(local, *location);
                }
            }
        }
        let local_invalid_locations: HashMap<_, Vec<_>> =
            HashMap::from_iter(local_invalid.iter().map(|(local, locations)| {
                (
                    *local,
                    locations
                        .iter()
                        .map(|v| location_table.to_location(*v))
                        .collect(),
                )
            }));
        println!("{local_invalid_locations:?}");

        //println!("{:?}", input.loan_invalidated_at);

        /*
        // compute actual regions where the local is live
        let mut actual_region_borrows = HashMap::new();
        for (location, regions) in output.origin_live_on_entry.iter() {
            if let Some(region_borrows) = output.origin_contains_loan_at.get(location) {
                for region in regions {
                    if let Some(borrows) = region_borrows.get(region) {
                        let insert = match actual_region_borrows.get_mut(region) {
                            Some(v) => v,
                            None => {
                                actual_region_borrows.insert(*region, BTreeSet::new());
                                actual_region_borrows.get_mut(region).unwrap()
                            }
                        };
                        insert.extend(borrows.clone());
                    }
                }
            }
        }
        for (location, locals) in output.var_live_on_entry.iter() {
            if let Some(loan_at) = output.origin_contains_loan_at.get(location) {
                for local in locals {
                    if let Some(borrow) = local_borrows(local) {
                        for (region, borrows) in loan_at.iter() {}
                    }
                }
            }
        }
        let mut local_actual_regions = HashMap::new();
        for (region, borrows) in actual_region_borrows.iter() {
            for borrow in borrows {
                if let Some(local) = borrow_locals.get(borrow) {
                    local_actual_regions.append(local, *region);
                }
            }
        }
        */

        //for (location_idx, region_idc) in output.origin_live_on_entry.iter() {}

        // all subset that must hold
        // borrows lives in mapped region indices
        // regions must includes all borrows, their key
        // region: borrow[i] must hold
        let mut borrow_regions = HashMap::new();
        for (region_id, borrow_idc) in output.origin_contains_loan_anywhere.iter() {
            for borrow_idx in borrow_idc.iter() {
                let insert = match borrow_regions.get_mut(borrow_idx) {
                    Some(v) => v,
                    None => {
                        borrow_regions.insert(*borrow_idx, BTreeSet::new());
                        borrow_regions.get_mut(borrow_idx).unwrap()
                    }
                };
                insert.insert(*region_id);
            }
        }
        */

        //for (location, region_borrows) in output.origin_contains_loan_at.iter() {}

        // mapped regions must includes locals living
        //for (sup, subs) in output.subset_anywhere.iter() {}

        // build basic blocks map
        let bb_map = facts
            .body
            .basic_blocks
            .iter_enumerated()
            .map(|(b, d)| (b, d.clone()))
            .collect();
        Self {
            compiler,
            location_table,
            facts,
            input,
            output_insensitive,
            output_datafrog,
            bb_map,
            //local_must_regions,
            //local_actual_regions: HashMap::new(),
            local_borrows,
            borrow_locals,
            //region_locations,
            //borrow_regions,
            //local_regions,
            //local_actual_locations,
            //local_borrow_regions,
            //borrow_local_def,
            //local_invalid_locations,
        }
    }

    fn sort_locs(v: &mut Vec<(BasicBlock, usize)>) {
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    }
    fn stmt_location_to_range(&self, bb: BasicBlock, stmt_index: usize) -> Option<Range> {
        self.bb_map
            .get(&bb)
            .map(|bb| bb.statements.get(stmt_index))
            .flatten()
            .map(|stmt| stmt.source_info.span.into())
    }
    fn rich_locations_to_ranges(&self, locations: &[RichLocation]) -> Vec<Range> {
        let mut starts = Vec::new();
        let mut mids = Vec::new();
        for rich in locations {
            match rich {
                RichLocation::Start(l) => {
                    starts.push((l.block, l.statement_index));
                }
                RichLocation::Mid(l) => {
                    mids.push((l.block, l.statement_index));
                }
            }
        }
        Self::sort_locs(&mut starts);
        Self::sort_locs(&mut mids);
        starts
            .iter()
            .zip(mids.iter())
            .filter_map(|(s, m)| {
                let sr = self.stmt_location_to_range(s.0, s.1);
                let mr = self.stmt_location_to_range(m.0, m.1);
                match (sr, mr) {
                    (Some(s), Some(m)) => Some(Range::new(s.from, m.until)),
                    _ => None,
                }
            })
            .collect()
    }

    /// obtain map from local id to living range
    fn drop_range(&self) -> HashMap<Local, Vec<Range>> {
        let mut local_live_locs = HashMap::new();
        for (loc_idx, locals) in self.output_datafrog.var_drop_live_on_entry.iter() {
            //for (loc_idx, locals) in self.output.var_live_on_entry.iter() {
            //for (loc_idx, locals) in self.output.var_drop_live_on_entry.iter() {
            let location = self.location_table.to_location(*loc_idx);
            for local in locals {
                let insert = match local_live_locs.get_mut(local) {
                    Some(v) => v,
                    None => {
                        local_live_locs.insert(*local, Vec::new());
                        local_live_locs.get_mut(local).unwrap()
                    }
                };
                insert.push(location);
            }

            /*
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
            */
        }
        HashMap::from_iter(
            local_live_locs
                .into_iter()
                .map(|(local, richs)| (local, self.rich_locations_to_ranges(&richs))),
        )
        /*
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
        */
    }

    /// collect user defined variables from debug info in MIR
    fn collect_user_vars(&self) -> HashMap<Local, (Span, String)> {
        self.facts
            .body
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
        let lives = self.get_accurate_live();
        //let local_loan = self.local_loan();
        let must_live_at = self.get_must_live();
        //let can_live_at = self.local_actually_live_at();
        //println!("{:?}", can_live_at);
        //println!("{:?}", self.region_locations);
        //let invalid = self.local_invalid();
        let drop_range = self.drop_range();
        self.facts
            .body
            .local_decls
            .iter_enumerated()
            .map(|(local, decl)| {
                let local_index = local.index();
                let ty = decl.ty.to_string();
                //let lives = lives.get(&local).cloned().unwrap_or(Vec::new());
                //let loan_live_at = local_loan.get(&local).cloned().unwrap_or(Vec::new());
                let must_live_at = must_live_at.get(&local).cloned().unwrap_or(Vec::new());
                //let lives = can_live_at.get(&local).cloned().unwrap_or(Vec::new());
                //let lives = self.local_live(local);
                let lives = lives.get(&local).cloned().unwrap_or(Vec::new());
                let drop = self.is_drop(local);
                let drop_range = drop_range.get(&local).cloned().unwrap_or(Vec::new());
                //println!("{:?}: {:?}", local, self.collect_borrow_recursive(local));
                if decl.is_user_variable() {
                    let (span, name) = user_vars.get(&local).cloned().unwrap();
                    MirDecl::User {
                        local_index,
                        name,
                        span: Range::from(span),
                        ty,
                        lives,
                        must_live_at,
                        drop,
                        drop_range,
                        //can_live_at,
                    }
                } else {
                    MirDecl::Other {
                        local_index,
                        ty,
                        lives,
                        must_live_at,
                        drop,
                        drop_range,
                        //can_live_at,
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

    fn erase_superset(mut ranges: Vec<Range>, erase_subset: bool) -> Vec<Range> {
        let mut len = ranges.len();
        let mut i = 0;
        while i < len {
            let mut j = i + 1;
            while j < len {
                if !erase_subset
                    && ((ranges[j].from <= ranges[i].from && ranges[i].until < ranges[j].until)
                        || (ranges[j].from < ranges[i].from && ranges[i].until <= ranges[j].until))
                {
                    ranges.remove(j);
                } else if erase_subset
                    && ((ranges[i].from <= ranges[j].from && ranges[j].until < ranges[i].until)
                        || (ranges[i].from < ranges[j].from && ranges[j].until <= ranges[i].until))
                {
                    ranges.remove(j);
                } else {
                    j += 1;
                }
                len = ranges.len();
            }
            i += 1;
        }
        ranges
    }
    /*
    fn local_must_live_at(&self) -> HashMap<Local, Vec<Range>> {
        HashMap::from_iter(self.local_must_regions.iter().map(|(local, regions)| {
            (
                *local,
                Self::erase_superset(
                    self.rich_locations_to_ranges(
                        &regions
                            .into_iter()
                            .filter_map(|v| self.region_locations.get(v).cloned())
                            .flatten()
                            .collect::<Vec<_>>(),
                    ),
                    false,
                ),
            )
        }))
    }
    fn local_invalid(&self) -> HashMap<Local, Vec<Range>> {
        HashMap::from_iter(
            self.local_invalid_locations
                .iter()
                .map(|(local, richs)| (*local, self.rich_locations_to_ranges(richs))),
        )
    }
    fn local_actually_live_at(&self) -> HashMap<Local, Vec<Range>> {
        HashMap::from_iter(
            self.local_actual_locations
                .iter()
                .map(|(local, richs)| (*local, self.rich_locations_to_ranges(richs))),
        )
        /*
        //HashMap::from_iter(self.local_regions.iter().map(|(local, regions)| {
        HashMap::from_iter(self.local_borrow_regions.iter().map(|(local, regions)| {
            (
                *local,
                Self::erase_superset(
                    self.rich_locations_to_ranges(
                        &regions
                            .into_iter()
                            .filter_map(|v| self.region_locations.get(v).cloned())
                            .flatten()
                            .collect::<Vec<_>>(),
                    ),
                    false,
                ),
            )
        }))
        */
    }
    /*
    fn local_can_lives_at(&self) -> HashMap<Local, Vec<Range>> {
        HashMap::from_iter(self.local_super_regions.iter().map(|(local, regions)| {
            (
                *local,
                Self::erase_superset(self.rich_locations_to_ranges(regions), true),
            )
        }))
    }
    */

    fn borrows_recursive(&self, local: Local) -> BTreeSet<Borrow> {
        let mut new = BTreeSet::new();
        if let Some(borrows) = self.local_borrows.get(&local) {
            for borrow in borrows {
                new.insert(*borrow);
                if let Some(local) = self.borrow_local_def.get(borrow) {
                    new.extend(self.borrows_recursive(*local));
                }
            }
        }
        new
    }
    fn collect_borrow_recursive(&self, local: Local) -> BTreeSet<Local> {
        let mut new = BTreeSet::new();
        new.insert(local);
        if let Some(borrows) = self.local_borrows.get(&local) {
            for borrow in borrows {
                if let Some(local) = self.borrow_local_def.get(borrow) {
                    new.extend(self.collect_borrow_recursive(*local));
                }
            }
        }
        new
    }
    fn local_live(&self, local: Local) -> Vec<Range> {
        let mut richs = Vec::new();
        for borrow in self.borrows_recursive(local).iter() {
            if let Some(regions) = self.borrow_regions.get(borrow) {
                for region in regions {
                    if let Some(locs) = self.region_locations.get(region) {
                        richs.extend_from_slice(locs);
                    }
                }
            }
        }
        self.rich_locations_to_ranges(&richs)
        /*
        for local in self.collect_borrow_recursive(local) {
            if let Some(regions) = self.local_borrow_regions.get(&local) {
                for region in regions {
                    if let Some(locations) = self.region_locations.get(region) {
                        richs.extend_from_slice(locations);
                    }
                }
            }
        }
        */
        /*
        let mut local_live_on = BTreeSet::new();
        for local in self.collect_borrow_recursive(local) {
            for (location, locals) in self.output.var_live_on_entry.iter() {
                if locals.contains(&local) {
                    local_live_on.insert(*location);
                }
            }
        }
        */
        /*
        let richs: Vec<_> = local_live_on
            .iter()
            .map(|v| self.location_table.to_location(*v))
            .collect();
        */
    }
    /*
    fn get_computed_var_lifetime(&self) {
        // borrow to container regions
        // local to their origins
        self.local_super_regions;
        let mut local_live_at = HashMap::new();
        for (location, locals) in self.output.var_live_on_entry.iter() {
            for local in locals {
                let insert = match local_live_at.get_mut(local) {
                    Some(v) => v,
                    None => {
                        local_live_at.insert(*local, BTreeSet::new());
                        local_live_at.get_mut(local).unwrap()
                    }
                };
                insert.insert(*location);
            }
        }
        let local_borrow = &self.facts.borrow_set.local_map;
        let local_live_at = HashMap::new();
        for (local, origin) in local_origin.iter() {
            if let Some(locations) = local_live_at.get(local) {}
        }
        self.output.origin_contains_loan_anywhere
    }
    */
    */

    fn get_accurate_live(&self) -> HashMap<Local, Vec<Range>> {
        let output = &self.output_datafrog;
        let mut local_loan_live_at = HashMap::new();
        for (location_idx, borrow_idc) in output.loan_live_at.iter() {
            let location = self.location_table.to_location(*location_idx);
            for borrow_idx in borrow_idc {
                if let Some(local) = self.borrow_locals.get(borrow_idx) {
                    let locations = match local_loan_live_at.get_mut(local) {
                        Some(v) => v,
                        None => {
                            local_loan_live_at.insert(*local, Vec::new());
                            local_loan_live_at.get_mut(local).unwrap()
                        }
                    };
                    locations.push(location);
                }
            }
        }
        HashMap::from_iter(
            local_loan_live_at
                .iter()
                .map(|(local, rich)| (*local, self.rich_locations_to_ranges(rich))),
        )
        /*
        let mut local_live_locs = HashMap::new();
        for (loc_idx, locals) in self.output_datafrog.var_live_on_entry.iter() {
            //for (loc_idx, locals) in self.output.var_drop_live_on_entry.iter() {
            let location = self.location_table.to_location(*loc_idx);
            for local in locals {
                let insert = match local_live_locs.get_mut(local) {
                    Some(v) => v,
                    None => {
                        local_live_locs.insert(*local, Vec::new());
                        local_live_locs.get_mut(local).unwrap()
                    }
                };
                insert.push(location);
            }
        }
        HashMap::from_iter(
            local_live_locs
                .into_iter()
                .map(|(local, richs)| (local, self.rich_locations_to_ranges(&richs))),
        )
            */
    }

    fn get_must_live(&self) -> HashMap<Local, Vec<Range>> {
        self.live_range_from_region(&self.output_insensitive)
    }

    fn live_range_from_region(&self, output: &PoloniusOutput) -> HashMap<Local, Vec<Range>> {
        //let output = &self.output_insensitive;
        let mut region_locations = HashMap::new();
        let mut region_locations_idc: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (location_idx, region_idc) in output.origin_live_on_entry.iter() {
            for region_idx in region_idc {
                let insert = match region_locations.get_mut(region_idx) {
                    Some(v) => v,
                    None => {
                        region_locations.insert(*region_idx, Vec::new());
                        region_locations.get_mut(region_idx).unwrap()
                    }
                };
                insert.push(self.location_table.to_location(*location_idx).clone());
                region_locations_idc.append(region_idx, *location_idx);
            }
        }

        // compute regions where the local must be live
        let mut local_must_regions: HashMap<Local, BTreeSet<Region>> = HashMap::new();
        for (region_idx, borrow_idc) in output.origin_contains_loan_anywhere.iter() {
            for borrow_idx in borrow_idc {
                if let Some(local) = self.borrow_locals.get(borrow_idx) {
                    local_must_regions.append(local, *region_idx);
                }
            }
        }
        let mut region_locations = HashMap::new();
        //let mut region_locations_idc: HashMap<_, BTreeSet<_>> = HashMap::new();
        for (location_idx, region_idc) in output.origin_live_on_entry.iter() {
            for region_idx in region_idc {
                let insert = match region_locations.get_mut(region_idx) {
                    Some(v) => v,
                    None => {
                        region_locations.insert(*region_idx, Vec::new());
                        region_locations.get_mut(region_idx).unwrap()
                    }
                };
                insert.push(self.location_table.to_location(*location_idx).clone());
                //region_locations_idc.append(region_idx, *location_idx);
            }
        }

        HashMap::from_iter(local_must_regions.iter().map(|(local, regions)| {
            (
                *local,
                Self::erase_superset(
                    self.rich_locations_to_ranges(
                        &regions
                            .into_iter()
                            .filter_map(|v| region_locations.get(v).cloned())
                            .flatten()
                            .collect::<Vec<_>>(),
                    ),
                    false,
                ),
            )
        }))
    }

    fn is_drop(&self, local: Local) -> bool {
        for (drop_local, _) in self.input.var_dropped_at.iter() {
            if *drop_local == local {
                return true;
            }
        }
        return false;
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
