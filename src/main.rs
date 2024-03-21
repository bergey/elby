use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

const upstream_url: &str = "http://httpbin.org/ip";

async fn proxy(_: Request<hyper::body::Incoming>) -> anyhow::Result<Response<Full<Bytes>>> {
    // Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
    let url = upstream_url.parse::<hyper::Uri>()?;

    let host = url.host().expect("uri has no host");
    // TODO default depends on http(s)
    let port = url.port_u16().unwrap_or(80);
    let address = format!("{}:{}", host, port);

    let stream = TcpStream::connect(address).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        // TODO what does this await actually do?
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    // The authority of our URL will be the hostname of the httpbin remote
    // TODO MDN says should include port (if not default), not include username, password
    let authority = url.authority().unwrap().clone();

    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    // Await the response...
    let res = sender.send_request(req).await?;
    let body = res.into_body().collect().await?.to_bytes();
    Ok(Response::new(Full::new(body)))

    // println!("Response status: {}", res.status());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = {
        // TODO configurable port & bind address
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        TcpListener::bind(addr).await?
    };

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(proxy))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
