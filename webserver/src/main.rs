use actix_web::{post, web, App, HttpServer, HttpResponse, Responder};
use serde::Deserialize;
use dotenv::dotenv;
use sqlx::PgPool;
use time::OffsetDateTime;

#[derive(Deserialize)]
struct ChatMessage {
    user_id: i32,
    content: String,
    /// Optional ISO 8601 timestamp; se omitido, o default do DB (`now()`) será usado
    #[serde(with = "time::serde::rfc3339::option")]
    timestamp: Option<OffsetDateTime>,
}

#[post("/messages")]
async fn create_message(
    pool: web::Data<PgPool>,
    msg: web::Json<ChatMessage>
) -> impl Responder {
    let result = sqlx::query!(
        r#"
        INSERT INTO messages (user_id, content, timestamp)
        VALUES ($1, $2, $3)
        "#,
        msg.user_id,
        msg.content,
        msg.timestamp
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Created().body("Message saved"),
        Err(err) => {
            eprintln!("Database error: {}", err);
            HttpResponse::InternalServerError().body("Failed to save message")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Carrega variáveis de ambiente de `.env`
    dotenv().ok();

    // Lê a URL do banco
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL deve estar definido em .env ou no ambiente");

    // Cria pool de conexões
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Falha ao conectar no Postgres");

    // Inicia o servidor HTTP
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(create_message)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
