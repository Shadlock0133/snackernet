use std::{
    fs::File,
    io::{self, Read, Result, Write},
    mem::size_of,
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::PathBuf,
    time::Duration,
};

use structopt::StructOpt;

const UDP_PORT: u16 = 3402;
const TCP_PORT: u16 = 3403;

#[derive(StructOpt)]
enum Opt {
    Server,
    Client { file: PathBuf },
}

fn main() -> io::Result<()> {
    match Opt::from_args() {
        Opt::Client { file } => {
            eprintln!("Opening file");
            let file = File::open(file)?;
            eprintln!("Connecting by UDP");
            let addr = udp_client(Some(Duration::from_secs(3)))?;
            eprintln!("Connecting by TCP");
            tcp_client(addr, file)?;
            eprintln!("Finished");
        }
        Opt::Server => {
            eprintln!("Starting UDP server");
            udp_server()?;
            eprintln!("Starting TCP server");
            tcp_server()?;
            eprintln!("Finished");
        }
    }
    Ok(())
}

fn udp_server() -> io::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, UDP_PORT))?;
    loop {
        let mut buf = [0; 4];
        match socket.recv_from(&mut buf)? {
            (4, addr) if buf == *b"SNAK" => {
                socket.send_to(b"SNEK", addr)?;
                return Ok(());
            }
            _ => {}
        }
    }
}

fn tcp_server() -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, TCP_PORT))?;
    for _ in 0..3 {
        match listener.accept() {
            Ok((socket, _)) => match tcp_server_connection(socket) {
                Ok(()) => return Ok(()),
                Err(e) => eprintln!("Connection error: {}", e),
            },
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Client not found"))
}

const LIMIT: u64 = 4 * 1024 * 1024;
fn tcp_server_connection(mut socket: TcpStream) -> io::Result<()> {
    let mut buf = [0; size_of::<u64>()];
    socket.read_exact(&mut buf)?;
    let len = u64::from_le_bytes(buf);
    if len > LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("File size bigger then limit ({})", LIMIT),
        ));
    }
    {
        let mut file_stream = socket.take(len);
        let mut file = File::create("./file")?;
        io::copy(&mut file_stream, &mut file)?;
    }
    Ok(())
}

fn udp_client(timeout: Option<Duration>) -> io::Result<SocketAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_write_timeout(timeout)?;
    socket.set_read_timeout(timeout)?;
    socket.set_broadcast(true)?;
    socket.send_to(b"SNAK", (Ipv4Addr::BROADCAST, UDP_PORT))?;
    for _ in 0..10 {
        let mut buf = [0; 4];
        match socket.recv_from(&mut buf)? {
            (4, addr) if buf == *b"SNEK" => return Ok(addr),
            _ => {}
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Server not found"))
}

fn tcp_client(mut addr: SocketAddr, mut file: File) -> Result<()> {
    addr.set_port(TCP_PORT);
    let len = file.metadata()?.len();
    let mut socket = TcpStream::connect(addr)?;
    socket.write_all(&len.to_le_bytes())?;
    io::copy(&mut file, &mut socket)?;
    Ok(())
}
