use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

use tonic::{transport::Server, Request, Response, Status};

use tokio_stream::wrappers::ReceiverStream;

use grpcast_proto::grpcast_server::{Grpcast, GrpcastServer};
use grpcast_proto::{DataLine, DataPacket, SubscribeCsvRequest, SubscribeRequest};

pub mod grpcast_proto {
    tonic::include_proto!("grpcast_package");
}

pub mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("grpcast_descriptor");
}

#[derive(Debug)]
pub struct GrpcastServe {
    data: broadcast::Receiver<String>,
    column_names: HashMap<usize, String>,
    first_line_err: Option<String>,
}

impl GrpcastServe {
    pub fn new(
        data: broadcast::Receiver<String>,
        first_line: Option<Result<String, String>>,
    ) -> Self {
        let mut column_names = HashMap::new();
        let mut first_line_err = None;
        if let Some(res) = first_line {
            match res {
                Ok(first_line) => {
                    for (idx, col_name) in first_line.split(',').enumerate() {
                        let col_name = col_name.trim();
                        if col_name.is_empty() {
                            continue;
                        }
                        column_names.insert(idx, col_name.trim().to_string());
                    }
                }
                Err(e) => {
                    first_line_err = Some(e);
                }
            }
        }
        Self {
            data,
            column_names,
            first_line_err,
        }
    }

    pub fn get_column_names(&self) -> HashMap<usize, String> {
        self.column_names.clone()
    }
}

pub fn get_datapacket(data_line: String, col_names: &HashMap<usize, String>) -> DataPacket {
    let mut packet = HashMap::new();
    for (idx, unit_data) in data_line.split(',').enumerate() {
        let name = col_names
            .get(&idx)
            .map(|s| s.to_string())
            .unwrap_or(idx.to_string());
        packet.insert(name, unit_data.to_string());
    }

    DataPacket { data: packet }
}

impl From<String> for DataLine {
    fn from(s: String) -> Self {
        DataLine { line: s }
    }
}

#[tonic::async_trait]
impl Grpcast for GrpcastServe {
    type SubscribeStream = ReceiverStream<Result<DataLine, Status>>;
    type SubscribeCsvStream = ReceiverStream<Result<DataPacket, Status>>;
    async fn subscribe_csv(
        &self,
        _req: Request<SubscribeCsvRequest>,
    ) -> Result<Response<Self::SubscribeCsvStream>, Status> {
        if let Some(e) = &self.first_line_err {
            return Err(Status::data_loss(format!(
                "Received an error when grabbing column names from the first line: `{}`",
                e
            )));
        }
        let (tx, rx) = mpsc::channel(16);
        let mut datastream = self.data.resubscribe();
        let col_names = self.column_names.clone();
        tokio::spawn(async move {
            let mut data = datastream.recv().await;
            loop {
                while let Err(broadcast::error::RecvError::Lagged(_)) = data {
                    data = datastream.recv().await;
                }
                if Err(broadcast::error::RecvError::Closed) == data {
                    return;
                }
                let line = data.unwrap();
                let packet = get_datapacket(line, &col_names);
                let res = tx.send(Ok(packet)).await;
                if res.is_err() {
                    break;
                }
                data = datastream.recv().await;
            }
        });
        Ok(Response::new(Self::SubscribeCsvStream::new(rx)))
    }
    async fn subscribe(
        &self,
        _req: Request<SubscribeRequest>,
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

pub struct GrpcServer {
    stop_signal: oneshot::Sender<()>,
    server_handle: JoinHandle<()>,
}

impl GrpcServer {
    pub fn new(
        input: broadcast::Receiver<String>,
        addr: SocketAddr,
        first_line: Option<Result<String, String>>,
    ) -> Self {
        let (they_stop_send, my_stop_recv) = oneshot::channel();

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build_v1alpha()
            .unwrap();
        let grpc_serve = GrpcastServe::new(input, first_line);
        let handle = tokio::spawn(async move {
            tokio::pin!(my_stop_recv);
            tokio::select! {
                _ = Server::builder()
                .add_service(reflection_service)
                .add_service(GrpcastServer::new(grpc_serve))
                .serve(addr) => {}
                _ = (&mut my_stop_recv) => {}
            }
        });
        Self {
            stop_signal: they_stop_send,
            server_handle: handle,
        }
    }

    pub async fn disconnect(self) {
        let _ = self.stop_signal.send(());
        let _ = self.server_handle.await;
    }
}
