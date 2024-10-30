#![feature(rustc_private)]

use axum::{routing, serve, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use visuarust_core::{models::*, run_compiler};

#[derive(Serialize)]
pub struct ApiError {
    success: bool,
    cause: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AnalyzeRequest {
    name: String,
    code: String,
}
#[derive(Serialize, Clone, Debug)]
pub struct AnalyzeResponse {
    success: bool,
    compile_error: bool,
    collected: CollectedData,
}
async fn analyze(req: Json<AnalyzeRequest>) -> Result<Json<AnalyzeResponse>, Json<ApiError>> {
    log::info!("start analyze");
    let result = std::panic::catch_unwind(|| {
        run_compiler(&req.name, &req.code).map_err(|(e, collected)| {
            log::warn!("compile error: {:?}", e);
            if let Some(collected) = collected {
                Ok(Json(AnalyzeResponse {
                    success: true,
                    compile_error: true,
                    collected,
                }))
            } else {
                Err(Json(ApiError {
                    success: false,
                    cause: format!("{:?}", e),
                }))
            }
        })
    });
    let collected = match result {
        Ok(v) => match v {
            Ok(v) => v,
            Err(e) => return e,
        },
        Err(e) => {
            return match e.downcast::<&dyn std::fmt::Debug>() {
                Ok(e) => Err(Json(ApiError {
                    success: false,
                    cause: format!("{:?}", e),
                })),
                Err(_) => Err(Json(ApiError {
                    success: false,
                    cause: format!("panic: {:?}", Error::UnknownError),
                })),
            }
        }
    };
    log::info!("analyze finished");
    let resp = AnalyzeResponse {
        success: true,
        compile_error: false,
        collected,
    };
    Ok(Json(resp))
}

#[tokio::main]
async fn main() {
    simple_logger::init().unwrap();
    //let filename = env::args().nth(1).unwrap();
    //run_compiler(&filename, &fs::read_to_string(&filename).unwrap());
    //println!("Hello, world!");
    let router = Router::new().route("/analyze", routing::post(analyze));
    serve(TcpListener::bind("0.0.0.0:8000").await.unwrap(), router)
        .await
        .unwrap();
}
