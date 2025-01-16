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
use rustc_hir::def_id::LocalDefId;
use rustc_interface::interface;
use rustc_middle::{query::queries::mir_borrowck::ProvidedValue, ty::TyCtxt, util::Providers};
use rustc_session::{config, EarlyDiagCtxt};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::{runtime::Builder, task::JoinSet};

pub struct RustcCallback;
impl Callbacks for RustcCallback {}

/*
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .worker_threads(8)
        .thread_stack_size(1024 * 1024 * 1024)
        .build()
        .unwrap()
});
static ANALYZE: LazyLock<Mutex<JoinSet<MirAnalyzer<'static, 'static>>>> =
    LazyLock::new(|| Mutex::new(JoinSet::new()));
*/

thread_local! {
    static BODIES: RefCell<Vec<(String, String, u32, consumers::BodyWithBorrowckFacts<'static>)>> = RefCell::new(Vec::new());
}

fn override_queries(_session: &rustc_session::Session, local: &mut Providers) {
    local.mir_borrowck = mir_borrowck;
}
fn mir_borrowck<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> ProvidedValue<'tcx> {
    log::info!("start borrowck of {def_id:?}");
    let facts = consumers::get_body_with_borrowck_facts(
        tcx,
        def_id,
        consumers::ConsumerOptions::PoloniusOutputFacts,
    );
    let source_map = tcx.sess.source_map();
    let filename = source_map.span_to_filename(facts.body.span);

    let source_file = source_map.get_source_file(&filename).unwrap();
    let offset = source_file.start_pos.0;

    let filename = filename
        .display(rustc_span::FileNameDisplayPreference::Local)
        .to_string_lossy()
        .to_string();
    let source = fs::read_to_string(&filename).unwrap();

    //let filename = tcx.hir().def_path(def_id).to_filename_friendly_no_crate();
    BODIES.with(|b| {
        b.borrow_mut().push((filename, source, offset, unsafe {
            std::mem::transmute(facts)
        }))
    });
    //log::info!("borrowck finished");

    /*
    log::info!("start analyze of {def_id:?}");
    let analyzer = MirAnalyzer::new(unsafe { std::mem::transmute(&tcx) }, unsafe {
        std::mem::transmute(&facts)
    });
    ANALYZE.lock().unwrap().spawn_on(analyzer, RUNTIME.handle());
    */
    /*
    let task = spawn(move || {
        let analyzed = analyzer.join().unwrap().analyze();
        log::info!("analyze of {def_id:?} finished");
        analyzed
        //println!("{}", serde_json::to_string(&analyzed).unwrap());
    });
    */
    //ANALYZE.lock().unwrap().push(task);

    let mut providers = Providers::default();
    rustc_borrowck::provide(&mut providers);
    (providers.mir_borrowck)(tcx, def_id)
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
        config.opts.incremental = None;
        config.override_queries = Some(override_queries);
        config.make_codegen_backend = None;
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
        _compiler: &interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        let rt = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(8)
            .thread_stack_size(1024 * 1024 * 1024)
            .build()
            .unwrap();
        let mut set = JoinSet::new();
        let collected = queries.global_ctxt().unwrap().enter(|tcx| {
            let bodies = BODIES.take();
            for i in 0..bodies.len() {
                let (filename, source, offset, body) = &bodies[i];
                let task = MirAnalyzer::new(
                    filename.clone(),
                    source.clone(),
                    *offset,
                    unsafe { std::mem::transmute(&tcx) },
                    unsafe { std::mem::transmute(body) },
                );
                set.spawn_on(task, rt.handle());
                //set.spawn_on(async move { task.await.analyze() }, rt.handle());
            }
            let mut files = HashMap::new();
            rt.block_on(async move {
                while let Some(analyzed) = set.join_next().await {
                    let analyze = analyzed.unwrap();
                    let (filename, analyzed) = analyze.analyze();
                    let File { items: push } = match files.get_mut(&filename) {
                        Some(v) => v,
                        None => {
                            files.insert(filename.clone(), File { items: Vec::new() });
                            files.get_mut(&filename).unwrap()
                        }
                    };
                    push.push(analyzed);
                }
                files
            })
        });
        let workspace = Workspace(collected);
        log::info!("print collected data of {}", self.path);
        println!("{}", serde_json::to_string(&workspace).unwrap());
        Compilation::Continue
    }
}

pub fn run_compiler() -> i32 {
    let ctxt = EarlyDiagCtxt::new(config::ErrorOutputType::default());
    let args = rustc_driver::args::raw_args(&ctxt).unwrap();
    let args = &args[1..];
    for arg in args {
        if arg == "-vV" || arg.starts_with("--print") {
            let mut callback = RustcCallback;
            let runner = RunCompiler::new(&args, &mut callback);
            return rustc_driver::catch_with_exit_code(|| runner.run());
        }
    }
    let mut callback = AnalyzerCallback::new();
    let mut runner = RunCompiler::new(&args, &mut callback);
    runner.set_make_codegen_backend(None);
    rustc_driver::catch_with_exit_code(|| {
        runner
            .set_using_internal_features(Arc::new(AtomicBool::new(true)))
            .run()
    })
}
