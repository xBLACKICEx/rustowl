#![feature(rustc_private)]

pub extern crate rustc_borrowck;
pub extern crate rustc_driver;
pub extern crate rustc_errors;
pub extern crate rustc_hash;
pub extern crate rustc_hir;
pub extern crate rustc_interface;
pub extern crate rustc_middle;
pub extern crate rustc_session;
pub extern crate rustc_span;

pub mod models;

use models::*;
use rustc_borrowck::consumers;
use rustc_hir::{ExprKind, ItemKind};
use rustc_interface::interface;
use rustc_middle::mir::{BindingForm, Body, Local, LocalDecl, LocalInfo, LocalKind, StatementKind};
use rustc_session::config;
use rustc_span::{FileName, RealFileName};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};

fn get_decl_from_local<'a, 'tcx>(mir: &'a Body<'tcx>, local: Local) -> Option<LocalDecl<'tcx>> {
    if let Some(decl) = mir.local_decls.get(local) {
        Some(decl.clone())
    } else {
        None
    }
}

fn show_span(source: &str, lo: u32, hi: u32) {
    let lines = source.split("\n");
    let mut current_line_head = 0;
    let lo = lo as usize;
    let hi = hi as usize;
    for (line, num) in lines.zip(1..) {
        let line_len = line.as_bytes().len() + 1;
        if lo <= current_line_head + line_len {
            let lo = if current_line_head <= lo {
                lo - current_line_head
            } else {
                0
            };
            let hi = hi - current_line_head;
            print!("{:3} | ", num);
            let mut acc = 0;
            for ch in line.chars() {
                let chlen = ch.len_utf8();
                if lo < acc + chlen {
                    print!("\x1b[91m");
                }
                acc += chlen;
                if hi < acc {
                    print!("\x1b[0m");
                }
                print!("{}", ch);
            }
            println!("");
        }
        if hi <= current_line_head + line_len {
            break;
        }
        current_line_head += line_len;
        print!("\x1b[0m");
    }
    print!("\x1b[0m");
}

pub fn run_compiler(
    name: &str,
    source: &str,
) -> Result<CollectedData, (Error, Option<CollectedData>)> {
    let path = PathBuf::from(name);
    //let source = fs::read_to_string(&path).unwrap();
    let config = interface::Config {
        opts: config::Options {
            debuginfo: config::DebugInfo::Full,
            error_format: config::ErrorOutputType::Json {
                pretty: true,
                json_rendered: rustc_errors::emitter::HumanReadableErrorType::Default,
                color_config: rustc_errors::ColorConfig::Auto,
            },
            unstable_opts: config::UnstableOptions {
                polonius: config::Polonius::Legacy,
                ..Default::default()
            },
            ..Default::default()
        },
        input: config::Input::Str {
            name: FileName::Real(RealFileName::LocalPath(path.clone())),
            input: source.to_owned(),
        },
        output_dir: None,
        output_file: None,
        file_loader: None,
        lint_caps: rustc_hash::FxHashMap::default(),
        register_lints: None,
        override_queries: None,
        registry: rustc_driver::diagnostics_registry(),
        crate_cfg: vec![],
        crate_check_cfg: vec![],
        expanded_args: vec![],
        hash_untracked_state: None,
        ice_file: None,
        locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES.to_owned(),
        make_codegen_backend: None,
        psess_created: None,
        //using_internal_features: rustc_driver::install_ice_hook("ice", |_| ()),
        using_internal_features: Arc::new(AtomicBool::new(true)),
    };
    log::info!("compiler configured; start to compile");
    interface::run_compiler(config, |compiler| {
        log::info!("interface::run_compiler called");
        compiler.enter(|queries| {
            println!("{}", compiler.sess.opts.unstable_opts.nll_facts);
            compiler.sess.source_map();
            log::info!("compiler.enter called");
            let Ok(mut gcx) = queries.global_ctxt() else {
                //rustc_errors::FatalError.raise()
                log::warn!("unknown error");
                return Err((Error::UnknownError, None));
            };
            let collected = gcx.enter(|ctx| {
                let mut items = Vec::new();
                log::info!("gcx.enter called");
                for item_id in ctx.hir().items() {
                    let item = ctx.hir().item(item_id);
                    match item.kind {
                        ItemKind::Fn(fnsig, _, fnbid) => {
                            //let mut variables = Vec::new();
                            //let mir = ctx.mir_built(item.owner_id).borrow();
                            log::info!(
                                "MIR of function (def ID index: {}) built",
                                item_id.owner_id.to_def_id().index.index(),
                            );

                            log::info!(
                                "start borrowck of def_id: {}",
                                item.owner_id.to_def_id().index.index(),
                            );
                            //let borrowck_res = ctx.mir_borrowck(item.owner_id);
                            /*
                            for (local_def_id, ty) in borrowck_res.concrete_opaque_types.iter() {
                                local_def_id.to_def_id();
                                //variables.push()
                            }
                            */
                            /*
                            let Some(err) = borrowck_res.tainted_by_errors {
                                err.k
                            }
                            */
                            log::info!("borrowck finished");
                            let body_cked = consumers::get_body_with_borrowck_facts(
                                ctx,
                                item.owner_id.def_id,
                                consumers::ConsumerOptions::PoloniusOutputFacts,
                            );

                            /*
                            let oos = consumers::calculate_borrows_out_of_scope_at_location(
                                &mir,
                                &body_cked.region_inference_context,
                                &body_cked.borrow_set,
                            );
                            /*
                            for (loc, v) in oos.iter() {
                                println!("{:?}", loc);
                                println!("{:?}", v);
                                println!("");
                            }
                            for constraint in body_cked.region_inference_context.outlives_constraints() {
                                let span = constraint.locations.span(&mir);
                            }
                            */
                            //body_cked.body.
                            //for (id, ty) in borrowck_res.concrete_opaque_types.iter() {}

                            println!("{:?}", mir.basic_blocks);

                            // MIR build
                            log::info!(
                                "start building MIR of def_id: {}",
                                item.owner_id.to_def_id().index.index(),
                            );
                            //let mir = ctx.mir_built(item.owner_id.def_id).borrow();
                            //let node = ctx.hir_node_by_def_id(item_id.owner_id.def_id());
                            //node.impl_block_of_trait;
                            //let mir = ctx.build_mir(item.owner_id.def_id);
                            //for decl in mir.local_decls.iter() {}
                            //let hir_body = ctx.hir_node(fnbid.hir_id);

                            //let local_def_id = ctx.hir().body_owner_def_id(fnbid);
                            //scope_tree.var_scope(local_def_id.into());
                            log::info!("MIR built");
                            /*
                            let scope_tree = ctx.region_scope_tree(item_id.owner_id.to_def_id());
                            for (scope, depth) in scope_tree.parent_map.iter() {
                                let hid = scope.hir_id(&scope_tree);
                                    //let ident = ctx.hir().ident(hid);
                                let scope = scope_tree.var_scope(scope.item_local_id());
                                if let Some(scope) = scope {
                                    let span = Range::from(scope.span(ctx, &scope_tree));
                                    variables.push(Variable::User {
                                        index: scope.item_local_id().index(),
                                        live: Range::from(span),
                                        dead: Range::from(span),
                                    });
                                }
                                /*
                                if let Some(hir_id) = scope.hir_id(&scope_tree) {
                                    let span = scope.span(ctx, &scope_tree);
                                    log::info!("{}-{}", span.lo().0, span.hi().0);
                                    variables.push(Variable::Other {
                                        index: hir_id.local_id.index(),
                                        live: Range::from(span),
                                        dead: Range::from(span),
                                    });
                                }
                                */
                            }
                            */
                            /*
                            for (local_def_id, opaq_type) in borrowck_res.concrete_opaque_types.into_iter() {
                                mir.local_decls.get(local_def_id.to_def_id());
                            }
                            let local_decs = mir.local_decls.clone();
                            let source = &source;
                            */
                                */
                            let mir = MirAnalyzer::analyze(compiler, &body_cked);

                            //let region_scope_tree = ctx.region_scope_tree(item.owner_id.to_def_id());
                            //region_scope_tree.opt_encl_scope().unwrap().
                            //let mut decls = Vec::new();
                            //for (local, decl) in mir.local_decls.iter_enumerated() {}
                            //for bb in mir.basic_blocks.iter() {
                            /*jk
                            //for (local, decls) in mir.local_decls.iter_enumerated() {}
                            for (scope, data) in mir.source_scopes.iter_enumerated() {
                                let span = data.span;
                                variables.push(Variable::Other {
                                    index: scope.index(),
                                    live: Range::from(span),
                                    dead: Range::from(span),
                                });
                            }
                            */
                            //log::info!("checking BasicBlock");
                            //for stmt in &bb.statements {
                            //stmts.push(stmt.clone());
                            /*
                            match stmt.kind {
                                StatementKind::StorageLive(v) => {
                                    let span = stmt.source_info.span;
                                    println!(
                                        "index: {}, live: {}-{}",
                                        v.index(),
                                        span.lo().0,
                                        span.hi().0
                                    );
                                    show_span(&source, span.lo().0, span.hi().0);
                                    //local(v);
                                }
                                StatementKind::StorageDead(v) => {
                                    let span = stmt.source_info.span;
                                    println!(
                                        "index: {}, dead: {}-{}",
                                        v.index(),
                                        span.lo().0,
                                        span.hi().0
                                    );
                                    show_span(&source, span.lo().0, span.hi().0);
                                    //local(v);
                                }
                                _ => {
                                    println!("other statement");
                                }
                            }
                            */
                            //}
                            //}
                            /*
                            log::info!("all BasicBlocks visited");
                            let variables = stmts.collect_variables(&mir);
                            */
                            let item = Item::Function {
                                span: Range::from(fnsig.span),
                                mir,
                            };
                            items.push(item);
                        }
                        _ => {}
                    }
                }
                let collected = CollectedData { items };
                if 0 < ctx.dcx().err_count() {
                    ctx.dcx().reset_err_count();
                    Err((Error::UnknownError, Some(collected)))
                } else {
                    Ok(collected)
                }
            });
            log::info!("returning collected data");
            collected
        })
    })
}
