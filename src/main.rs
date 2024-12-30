use std::process;
use std::sync::Arc;

use axum::Router;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, limit::RequestBodyLimitLayer, validate_request::ValidateRequestHeaderLayer};
use tracing_subscriber::{fmt::{writer::BoxMakeWriter, Layer}, layer::SubscriberExt, EnvFilter, Registry};

use routes::auth::AuthService;
use db::auth::AuthRepository;

mod db;
mod routes;

#[tokio::main]
async fn main() {
    
    // mandatory fields
    let db_url = dotenv::var("DATABASE_URL").unwrap();
    let jwt_secret = dotenv::var("JWT_SECRET").unwrap_or("your-jwt-secret".to_string());
    // optional fields
    let max_connection_pooling = dotenv::var("MAX_CONNECTION_POOLING").unwrap_or("5".to_string()).parse::<u32>().unwrap();
    let port = dotenv::var("PORT").unwrap_or("3000".to_string()).parse::<u16>().unwrap();    
    let log_file = dotenv::var("LOG_FILE").unwrap_or("app.log".to_string());

    // add tracing layer
    let file_appender = tracing_appender::rolling::never(".", &log_file);
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);
    let (stdout_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());

    // use tracer to log inotf files
    let file_layer = Layer::new().json().with_writer(BoxMakeWriter::new(move || file_writer.clone()));
    let stdout_layer = Layer::new().with_writer(BoxMakeWriter::new(move || stdout_writer.clone()));

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(file_layer)
        .with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");

    let database_pool = match process_database(&db_url, max_connection_pooling).await {
        Ok(db) => {
            tracing::info!("Connected to database");
            db
        },
        Err(err) => {
            tracing::error!("Failed to connect to database: {}", err);
            process::exit(1);
        }
    };

    let listener = match TcpListener::bind(("0.0.0.0", port)).await {
        Ok(port) => {
            tracing::info!("Listening on port: {}", port.local_addr().unwrap().port());
            port
        }
        Err(err) => {
            tracing::error!("Failed to bind to port: {}", err);
            process::exit(1);
        }
    };

    let router = match process_begin(database_pool, jwt_secret) {
        Ok(router) => {
            tracing::info!("Routes constructed successfully");
            router
        }
        Err(err) => {
            tracing::error!("Failed to construct routes: {}", err);
            process::exit(1);
        }
    };

    //start the http service
    let http_service = axum::serve(listener, router);
    if let Err(err) = http_service.await {
        tracing::error!("Failed to start server: {}", err);
        process::exit(1);
    }
}

fn process_begin(db_pool: PgPool, jwt_secret: String) -> Result<Router, String> {
    let head_route = Router::new();

    let repo = AuthRepository::new(db_pool.clone());
    let service = Arc::new(AuthService::new(repo, jwt_secret));

    let auth_routes = routes::auth::auth_routes(service.clone());
    let user_routes = routes::user::user_routes(service.clone(), db_pool.clone())
        .route_layer(ValidateRequestHeaderLayer::accept("Authorization"));
    let transfer_routes = routes::tx::tx_route(service.clone(), db_pool.clone())
        .route_layer(ValidateRequestHeaderLayer::accept("Authorization"))
        .route_layer(CompressionLayer::new().gzip(true));

    let router = head_route
        .nest("/v1", auth_routes)
        .nest("/v1", user_routes)
        .nest("/v1", transfer_routes)
        .route_layer(RequestBodyLimitLayer::new(1024 * 1024 * 10)); //10KB limit

    Ok(router)
}

async fn process_database(url: &str, max_conn_pool: u32) -> Result<PgPool, String> {
    // create a connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(max_conn_pool)
        .connect(url)
        .await
        .map_err(|err| format!("Failed to connect to database: {}", err))?;

    match sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .map_err(|err| format!("Failed to run migrations: {}", err))
    {
        Ok(_) => {
            tracing::info!("Migrations run successfully");
        },
        Err(err) => {
            // if it fails we assume to continue believing that the database is already migrated
            tracing::warn!("Failed to run migrations: {err}");
        },
    }

    Ok(db_pool)
}
