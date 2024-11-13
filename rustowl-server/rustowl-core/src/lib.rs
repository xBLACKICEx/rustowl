#![feature(rustc_private)]

pub extern crate polonius_engine;
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
use rustc_hir::ItemKind;
use rustc_interface::interface;
use rustc_session::config;
use rustc_span::{FileName, RealFileName};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};

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
        lint_caps: HashMap::default(),
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
                        ItemKind::Fn(fnsig, _, _fnbid) => {
                            log::info!(
                                "start borrowck of def_id: {}",
                                item.owner_id.to_def_id().index.index(),
                            );
                            let facts = consumers::get_body_with_borrowck_facts(
                                ctx,
                                item.owner_id.def_id,
                                consumers::ConsumerOptions::PoloniusInputFacts,
                            );
                            log::info!("borrowck finished");
                            log::info!("MIR built");

                            log::info!("enter MIR analysis");
                            let mut analyzer = MirAnalyzer::new(compiler, &facts);
                            let mir = analyzer.analyze();
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
