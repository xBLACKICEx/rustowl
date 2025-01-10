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
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::{def_id::LocalDefId, ItemKind};
use rustc_interface::interface;
use rustc_middle::{query::queries::mir_borrowck::ProvidedValue, ty::TyCtxt, util::Providers};
use rustc_session::config;
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, LazyLock, Mutex};

pub struct RustcCallback;
impl Callbacks for RustcCallback {}

thread_local! {
    static MIRS: LazyLock<Mutex<HashMap<LocalDefId, consumers::BodyWithBorrowckFacts<'static>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
}

fn override_queries(_session: &rustc_session::Session, local: &mut Providers) {
    local.mir_borrowck = mir_borrowck;
}
fn mir_borrowck<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> ProvidedValue<'tcx> {
    log::info!("start borrowck of def_id: {def_id:?}");
    let facts = consumers::get_body_with_borrowck_facts(
        tcx,
        def_id,
        consumers::ConsumerOptions::PoloniusOutputFacts,
    );
    MIRS.with(|v| {
        v.lock()
            .unwrap()
            .insert(def_id, unsafe { std::mem::transmute(facts) })
    });
    log::info!("borrowck finished");
    log::info!("MIR built");

    let mut providers = Providers::default();
    rustc_borrowck::provide(&mut providers);
    let original_mir_borrowck = providers.mir_borrowck;
    original_mir_borrowck(tcx, def_id)
}

pub struct AnalyzerCallback {
    path: String,
}
impl AnalyzerCallback {
    pub fn new() -> Self {
        Self {
            path: "".to_owned(),
        }
    }
}
impl Callbacks for AnalyzerCallback {
    fn config(&mut self, config: &mut interface::Config) {
        config.opts.unstable_opts.mir_opt_level = Some(0);
        config.opts.unstable_opts.polonius = config::Polonius::Next;
        config.override_queries = Some(override_queries);
        self.path.push_str(
            &*config
                .input
                .source_name()
                .display(rustc_span::FileNameDisplayPreference::Local)
                .to_string_lossy(),
        );
    }
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        //println!("{}", compiler.sess.opts.unstable_opts.nll_facts);
        //compiler.sess.source_map();
        log::info!("compiler.enter called");
        let Ok(mut gcx) = queries.global_ctxt() else {
            log::warn!("unknown error");
            panic!();
            //return Err((Error::UnknownError, None));
        };
        gcx.enter(|ctx| {
            let mut items = Vec::new();
            log::info!("gcx.enter called");
            for item_id in ctx.hir().items() {
                let item = ctx.hir().item(item_id);
                match item.kind {
                    ItemKind::Fn(fnsig, _, _fnbid) => {
                        let mir = MIRS.with(|facts| {
                            let facts = facts.lock().unwrap();
                            let facts = facts.get(&item.owner_id.def_id).unwrap();
                            log::info!("enter MIR analysis");
                            let mut analyzer = MirAnalyzer::new(compiler, &facts);
                            let mir = analyzer.analyze();
                            log::info!("MIR analyzed");
                            mir
                        });

                        let item = Item::Function {
                            span: Range::from(fnsig.span),
                            mir,
                        };
                        items.push(item);
                    }
                    _ => {}
                }
            }
            let collected = File { items };
            log::info!("print collected data");
            let workspace = Workspace(HashMap::from([(self.path.clone(), collected)]));
            println!("{}", serde_json::to_string(&workspace).unwrap());
        });
        Compilation::Continue
    }
}

pub fn run_compiler(args: &[String]) -> i32 {
    for arg in args {
        if arg == "-vV" {
            return rustc_driver::catch_with_exit_code(|| {
                RunCompiler::new(&args, &mut RustcCallback).run()
            });
        }
    }
    rustc_driver::catch_with_exit_code(|| {
        RunCompiler::new(&args, &mut AnalyzerCallback::new())
            .set_using_internal_features(Arc::new(AtomicBool::new(true)))
            .run()
    })
}
