#![feature(rustc_private)]

pub extern crate indexmap;
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
pub extern crate smallvec;

mod analyze;

use analyze::MirAnalyzer;
use models::*;
use rustc_borrowck::consumers;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::def_id::LocalDefId;
use rustc_interface::interface;
use rustc_middle::{
    mir::BorrowCheckResult, query::queries::mir_borrowck::ProvidedValue, ty::TyCtxt,
    util::Providers,
};
use rustc_session::{config, EarlyDiagCtxt};
use std::cell::{LazyCell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::sync::{atomic::AtomicBool, Arc, LazyLock, Mutex};
use tokio::{
    runtime::Builder,
    task::JoinSet,
    time::{sleep, timeout, Duration},
};

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
*/

static TASKS: LazyLock<Mutex<JoinSet<MirAnalyzer<'static, 'static>>>> =
    LazyLock::new(|| Mutex::new(JoinSet::new()));
static TASK_LEN: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
thread_local! {
    //static TASK: RefCell<JoinSet<MirAnalyzer<'static, 'static>>> = RefCell::new(JoinSet::new());
    static ANALYZERS: RefCell<Vec<MirAnalyzer<'static, 'static>>> = RefCell::new(Vec::new());
    //static BODIES: RefCell<Vec<(String, String, u32, consumers::BodyWithBorrowckFacts<'static>)>> = RefCell::new(Vec::new());
    //static IDS: RefCell<Vec<LocalDefId>> = RefCell::new(Vec::new());
}

fn override_queries(_session: &rustc_session::Session, local: &mut Providers) {
    local.mir_borrowck = mir_borrowck;
    //local.analysis = |_, _| Ok(());
}
fn mir_borrowck<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> ProvidedValue<'tcx> {
    log::info!("start borrowck of {def_id:?}");
    //IDS.with(|v| v.borrow_mut().push(def_id));

    let rt = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(8)
        .thread_stack_size(1024 * 1024 * 1024)
        .build()
        .unwrap();

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

    /*
    BODIES.with(|b| {
        b.borrow_mut().push((filename, source, offset, unsafe {
            std::mem::transmute(facts)
        }))
    });
    */
    //log::info!("borrowck finished");

    log::info!("start analyze of {def_id:?}");
    let analyzer = MirAnalyzer::new(
        filename,
        source,
        offset,
        unsafe { std::mem::transmute(&tcx) },
        unsafe { std::mem::transmute(&facts) },
    );
    {
        TASKS.lock().unwrap().spawn_on(analyzer, rt.handle());
        let mut len = TASK_LEN.lock().unwrap();
        *len += 1;
        if *len == 1 {
            drop(len);
            rt.block_on(async move {
                loop {
                    sleep(Duration::from_millis(100)).await;
                    if { TASKS.lock().unwrap().len() } == { *TASK_LEN.lock().unwrap() } {
                        break;
                    }
                }
                while let Some(task) = TASKS.lock().unwrap().join_next().await {
                    let (filename, analyzed) = task.unwrap().analyze();
                    let ws = Workspace(HashMap::from([(
                        filename,
                        File {
                            items: vec![analyzed],
                        },
                    )]));
                    println!("{}", serde_json::to_string(&ws).unwrap());
                }
            });
        }
    }
    //ANALYZE.lock().unwrap().spawn_on(analyzer, RUNTIME.handle());
    /*
    let task = spawn(move || {
        let analyzed = analyzer.join().unwrap().analyze();
        log::info!("analyze of {def_id:?} finished");
        analyzed
        //println!("{}", serde_json::to_string(&analyzed).unwrap());
    });
    */
    //ANALYZE.lock().unwrap().push(task);

    let result = BorrowCheckResult {
        concrete_opaque_types: indexmap::IndexMap::default(),
        closure_requirements: None,
        used_mut_upvars: smallvec::SmallVec::new(),
        tainted_by_errors: None,
    };
    tcx.arena.alloc(result)
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

    /*
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        log::info!("after analysis");
        let rt = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(8)
            .thread_stack_size(1024 * 1024 * 1024)
            .build()
            .unwrap();
        let mut set = JoinSet::new();
        let collected = queries.global_ctxt().unwrap().enter(|tcx| {
            let ids = IDS.take();
            for id in &ids {
                log::info!("start borrowck of {id:?}");
                let facts = consumers::get_body_with_borrowck_facts(
                    tcx,
                    *id,
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

                log::info!("{:?}", facts.body);
                let task = MirAnalyzer::new(
                    filename.clone(),
                    source.clone(),
                    offset,
                    unsafe { std::mem::transmute(&tcx) },
                    unsafe { std::mem::transmute(&facts) },
                );
                set.spawn_on(task, rt.handle());
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
    */
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
