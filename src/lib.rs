use actix_rt::net::UnixStream;
use actix_service::Service;
use actix_tls::connect::{Connect, ConnectError, Connection};
use awc::http::Uri;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct UdsConnector(PathBuf);

impl UdsConnector {
    pub fn new(path: impl AsRef<Path>) -> Self {
        UdsConnector(path.as_ref().to_path_buf())
    }
}

impl Service<Connect<Uri>> for UdsConnector {
    type Response = Connection<Uri, UnixStream>;
    type Error = ConnectError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: Connect<Uri>) -> Self::Future {
        let uri = req.request().clone();
        let path = self.0.clone();
        let fut = async {
            let stream = UnixStream::connect(path).await.unwrap();
            Ok(Connection::new(stream, uri))
        };
        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn it_works() {
        use super::UdsConnector;
        use actix_web::{get, App, HttpRequest, HttpServer};
        use awc::{ClientBuilder, Connector};
        use std::time::Duration;
        use tempfile::tempdir;

        let socket_dir = tempdir().unwrap();
        let socket_path = socket_dir.path().join("awc-uds.sock");
        let socket_path2 = socket_path.clone();

        #[get("/")]
        async fn hello(_req: HttpRequest) -> String {
            "hello".to_string()
        }
        let _handle = actix_rt::spawn(async {
            let _server = HttpServer::new(|| App::new().service(hello))
                .bind_uds(socket_path)
                .unwrap()
                .run()
                .await;
        });

        actix_rt::time::sleep(Duration::from_secs(1)).await;
        let connector = Connector::new().connector(UdsConnector::new(socket_path2));
        let client = ClientBuilder::new().connector(connector).finish();
        let resp = client
            .get("http://localhost/")
            .send()
            .await
            .unwrap()
            .body()
            .await
            .unwrap();
        assert_eq!(resp, b"hello".as_ref());
        println!("{:?}", resp);
    }
}
