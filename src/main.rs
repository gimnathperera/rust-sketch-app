mod gen_server;
mod ws_client;
mod line;

use axum::{
    extract::{
        ws::WebSocketUpgrade,
        TypedHeader,
    },
    response::IntoResponse,
    routing::get,
    Router,
    Extension
};
use axum::extract::connect_info::ConnectInfo;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::mpsc::Sender;
use gen_server::GSMsg;

const LISTEN_ON : &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
    //.with(tracing_subscriber::EnvFilter::try_from_default_env()
	.with(tracing_subscriber::EnvFilter::try_from_env("SKETCH_IO")
              .unwrap_or_else(|_| "sketch_io=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let gs_tx = gen_server::spawn(); 

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pub");
    
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir)
			  .append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))	
	.layer(Extension(gs_tx))
        .layer(TraceLayer::new_for_http()
               .make_span_with(DefaultMakeSpan::default()
			       .include_headers(false)));
        
    let addr : SocketAddr = LISTEN_ON.parse()?;
    tracing::info!("listening on {}", addr);    
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(gs_tx): Extension<Sender<GSMsg>>,
    user_agent: Option<TypedHeader<axum::headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    tracing::info!("{addr} connected [{user_agent}].");
    ws.on_upgrade(move |socket| ws_client::handle_socket(socket, addr, gs_tx))
}
