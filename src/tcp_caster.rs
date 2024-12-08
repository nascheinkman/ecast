use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::{broadcast, oneshot};
use tokio::task::{JoinHandle, JoinSet};

pub struct TcpCaster {
    stop_signal: oneshot::Sender<()>,
    server_handle: JoinHandle<()>,
}

fn clear_clients(set: &mut JoinSet<()>) {
    while set.try_join_next().is_some() {}
}

impl TcpCaster {
    pub async fn new<A: ToSocketAddrs + std::fmt::Debug + Send + 'static + Copy>(
        input: broadcast::Receiver<String>,
        addr: A,
    ) -> std::io::Result<Self> {
        let (they_stop_send, my_stop_recv) = oneshot::channel();
        let listener = TcpListener::bind(addr).await?;
        let handle = tokio::spawn(async move {
            let mut clients: JoinSet<()> = JoinSet::new();
            tokio::pin!(my_stop_recv);
            loop {
                clear_clients(&mut clients);
                let res = tokio::select! {
                    _ = (&mut my_stop_recv) => {
                        break
                    }
                    res = listener.accept() => {
                        res
                    }
                };
                let (mut stream, _client) = match res {
                    Ok((stream, client)) => (stream, client),
                    Err(_) => continue,
                };
                let mut new_input = input.resubscribe();
                clients.spawn(async move {
                    let mut data = new_input.recv().await;
                    loop {
                        while let Err(broadcast::error::RecvError::Lagged(_)) = data {
                            data = new_input.recv().await;
                        }
                        if Err(broadcast::error::RecvError::Closed) == data {
                            return;
                        }
                        let data_str = data.unwrap();
                        let res = stream.write_all(&data_str.into_bytes()).await;
                        if res.is_err() {
                            return;
                        }
                        data = new_input.recv().await;
                    }
                });
            }
            clients.abort_all();
            clients.join_all().await;
        });
        Ok(Self {
            stop_signal: they_stop_send,
            server_handle: handle,
        })
    }

    pub async fn disconnect(self) {
        let _ = self.stop_signal.send(());
        let _ = self.server_handle.await;
    }
}
