use poem::{handler, listener::TcpListener, route, route::post, web::Multipart, Server};

#[handler]
async fn index(mut multipart: Multipart) {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().map(ToString::to_string);
        let file_name = field.file_name().map(ToString::to_string);
        if let Ok(bytes) = field.bytes().await {
            println!(
                "name={:?} filename={:?} length={}",
                name,
                file_name,
                bytes.len()
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug")
    }
    tracing_subscriber::fmt::init();

    let app = route().at("/", post(index));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
