use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use std::net::SocketAddr;

use tonic::{transport::Server, Request, Response, Status};

use tokio_stream::wrappers::ReceiverStream;

use grpcast_proto::grpcast_server::{Grpcast, GrpcastServer};
use grpcast_proto::{DataLine, SubscribeRequest};

pub mod grpcast_proto {
    tonic::include_proto!("grpcast_package");
}

pub mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("grpcast_descriptor");
}

#[derive(Debug)]
pub struct GrpcastServe{
    data: broadcast::Receiver<String>
}

impl GrpcastServe {
    pub fn new(data: broadcast::Receiver<String>) -> Self {
        Self {
            data
        }
    }
}

impl From<String> for DataLine {
    fn from(s: String) -> Self {
        DataLine {
            line: s
        }
    }
}

#[tonic::async_trait]
impl Grpcast for GrpcastServe {
    type SubscribeStream = ReceiverStream<Result<DataLine, Status>>;
    async fn subscribe(
        &self,
        _req: Request<SubscribeRequest>
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let (tx, rx) = mpsc::channel(16);
        let mut datastream = self.data.resubscribe();
        tokio::spawn(async move {
            let mut data = datastream.recv().await;
            loop {
                while let Err(broadcast::error::RecvError::Lagged(_)) = data {
                    data = datastream.recv().await;
                }
                if Err(broadcast::error::RecvError::Closed) == data {
                    return;
                }
                let data_obj = data.unwrap().into();
                let res = tx.send(Ok(data_obj)).await;
                if res.is_err() {
                    break;
                }
                data = datastream.recv().await;
            }
        });
        Ok(Response::new(Self::SubscribeStream::new(rx)))
    }
}

pub fn grpc_cast_server(
    input: broadcast::Receiver<String>,
    addr: SocketAddr,
) -> JoinHandle<()> {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1alpha()
        .unwrap();
    let grpc_serve = GrpcastServe::new(input);
    tokio::spawn(async move {
        let _res = Server::builder()
            .add_service(reflection_service)
            .add_service(GrpcastServer::new(grpc_serve))
            .serve(addr)
            .await;
    })
}
