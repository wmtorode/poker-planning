use crate::{
    configuration::get_configuration,
    handlers::{health_check, index, index_playground, index_ws},
    schema::{MutationRoot, QueryRoot, SubscriptionRoot},
    types::Storage,
};
use actix_cors::Cors;

use std::{
    fs::File,
    io::{self, Read as _},
};

use actix_web::{
    guard, middleware,
    web::{self, Data},
    App, HttpServer,
};
use openssl::{
    pkey::{PKey, Private},
    ssl::{SslAcceptor, SslMethod},
};


use async_graphql::Schema;
use env_logger::Env;

mod configuration;
mod domain;
mod handlers;
mod schema;
mod simple_broker;
mod types;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world! This is the server for pokerplanning.org");

    let env = Env::default().filter_or("RUST_LOG", "actix_web=trace");
    env_logger::init_from_env(env);

    let settings = get_configuration().expect("Failed to read settings.");
    let server_address = settings.get_server_address();

    let storage: Storage = Storage::default();

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(storage.clone())
        .finish();

    // build TLS config from files
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    // set the unencrypted private key
    builder
        .set_private_key_file(settings.get_key_file(), openssl::ssl::SslFiletype::PEM)
        .unwrap();

    // set the certificate chain file location
    builder.set_certificate_chain_file(settings.get_cert_file()).unwrap();

    println!("Playground: http://{}", server_address);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(schema.clone()))
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
            .service(web::resource("/").guard(guard::Post()).to(index))
            .service(
                web::resource("/")
                    .guard(guard::Get())
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(index_ws),
            )
            .service(web::resource("/").guard(guard::Get()).to(index_playground))
            .service(
                web::resource("/health_check")
                    .guard(guard::Get())
                    .to(health_check),
            )
    })
    .bind_openssl(server_address, builder)?
    .run()
    .await
}
