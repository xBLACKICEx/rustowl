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

mod analyze;
pub mod models;

use analyze::MirAnalyzer;
use models::*;
use rustc_borrowck::consumers;
use rustc_hir::{ExprKind, ItemKind};
use rustc_interface::interface;
use rustc_middle::mir::{BindingForm, Body, Local, LocalDecl, LocalInfo, LocalKind, StatementKind};
use rustc_session::config;
use rustc_span::{FileName, RealFileName};
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
                            log::info!(
                                "start borrowck of def_id: {}",
                                item.owner_id.to_def_id().index.index(),
                            );
                            let body_cked = consumers::get_body_with_borrowck_facts(
                                ctx,
                                item.owner_id.def_id,
                                consumers::ConsumerOptions::PoloniusInputFacts,
                            );
                            log::info!("borrowck finished");
                            log::info!("MIR built");

                            log::info!("enter MIR analysis");
                            let mir = MirAnalyzer::analyze(compiler, &body_cked);
                            log::info!("MIR analyzed");

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
