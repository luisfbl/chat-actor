use std::net::SocketAddr;

use axum::http::{HeaderValue, StatusCode, header};
use axum::{
    Json, Router,
    response::{Html, IntoResponse},
    routing::{get, get_service},
};
use serde::Serialize;
use tokio::signal;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};

#[tokio::main]
async fn main() {
    let static_service = get_service(ServeDir::new("../website/dist")).handle_error(
        |error: std::io::Error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Erro interno ao servir arquivo estático: {}", error),
            )
        },
    );

    let app = Router::new()
        .route("/", get(root))
        .route("/api/hello", get(api_hello))
        .nest_service("/static", static_service)
        .layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000"),
        ));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Webserver rodando em http://{}", addr);

    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(err) = graceful.await {
        eprintln!("Erro no servidor: {}", err);
    }
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("Falha ao escutar sinal Ctrl+C");
    println!("🛑 Sinal de shutdown recebido, encerrando o servidor...");
}

async fn root() -> impl IntoResponse {
    Html("<h1>Bem-vindo ao Webserver!</h1>")
}

#[derive(Serialize)]
struct Hello {
    msg: String,
}

async fn api_hello() -> Json<Hello> {
    Json(Hello {
        msg: "Olá Luís!".to_string(),
    })
}
