use clap::Parser;
use ecast::file_caster::cast_to_file;
use ecast::grpc_caster::GrpcServer;
use ecast::input_handler::InputHandler;
use ecast::stdout_caster::cast_to_stdout;
use ecast::tcp_caster::TcpCaster;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio::io::stdin;
use tokio::io::BufReader;
use tokio::task::JoinSet;

/// Broadcast stdin to any multiple of the supported protocols
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log stdin to a file at the specified path
    #[arg(short, long)]
    files: Option<Vec<String>>,
    /// Echo stdin back to stdout
    #[arg(short, long, action)]
    echo: bool,
    /// Create a TCP server at the given address that allows any client to stream the data
    #[arg(short, long, action)]
    tcp_address: Option<SocketAddr>,
    /// Creates a GRPC server with reflection at the given address that allows any client to stream
    /// the data.
    #[arg(short, long, action)]
    grpc_address: Option<SocketAddr>,
    /// Use CSV column names from the first line of data. Only useful when using GRPC
    #[arg(short, long, action)]
    use_given_names: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut handles = vec![];
    let files = args.files.unwrap_or_default();

    // Create the files first so that file errors show up early
    let mut file_handles = vec![];
    for file in files {
        let log_file = File::create(file).await?;
        file_handles.push(log_file);
    }
    let input_handler = InputHandler::new_connect(BufReader::new(stdin()));
    let mut data_recv = input_handler.subscribe().unwrap();
    let first_line = tokio::spawn(async move { data_recv.recv().await });
    for file in file_handles.drain(..) {
        let handle = cast_to_file(&input_handler, file);
        handles.push(handle);
    }
    if args.echo {
        let handle = cast_to_stdout(&input_handler);
        handles.push(handle);
    }
    let mut tcp_server = None;
    if let Some(addr) = args.tcp_address {
        tcp_server = Some(TcpCaster::new(input_handler.subscribe().unwrap(), addr).await?);
    }
    let mut grpc_server = None;
    if let Some(addr) = args.grpc_address {
        let first_line: Option<Result<String, String>> = if !args.use_given_names {
            None
        } else {
            let x = first_line
                .await
                .map_err(|e| e.to_string())
                .map(|r| r.map_err(|e| e.to_string()));
            match x {
                Ok(y) => Some(y),
                Err(e) => Some(Err(e)),
            }
        };
        grpc_server = Some(GrpcServer::new(
            input_handler.subscribe().unwrap(),
            addr,
            first_line,
        ));
    }
    input_handler.wait_for_close().await;
    let mut server_stops = JoinSet::new();
    if let Some(tcp_server) = tcp_server {
        server_stops.spawn(tcp_server.disconnect());
    }
    if let Some(grpc_server) = grpc_server {
        server_stops.spawn(grpc_server.disconnect());
    }
    let _ = server_stops.join_all().await;
    Ok(())
}
