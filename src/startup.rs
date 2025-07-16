
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use igd::SearchOptions;
use igd::{aio::search_gateway, PortMappingProtocol};
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::event::Event;

#[derive(Debug)]
pub enum StartupEvent {
    ProgressHint(String),
    Complete { 
        public_ip: Ipv4Addr,
        local_ip: Ipv4Addr,
    }
}

#[derive(Debug)]
pub enum StartupError {
    Ipv6NotSupported,
    IpNotFound(local_ip_address::Error),
    SearchFailed(igd::SearchError),
    AddPortFailed(igd::AddPortError),
    GetIpFailed(igd::GetExternalIpError),
}

pub async fn startup(
    port: Option<u16>,
    ev_tx: mpsc::Sender<Event>,
) {
    // get the local ip 
    let port = port.unwrap_or(42069);
    let local_ip = match local_ip_address::local_ip() {
        Ok(IpAddr::V4(ip)) => ip,
        Ok(_) => {
            let _ = ev_tx.send(Event::StartupError(StartupError::Ipv6NotSupported)).await;
            return;
        },
        Err(e) => {
            let _ = ev_tx.send(Event::StartupError(StartupError::IpNotFound(e))).await;
            return;
        }
    };

    let _ = ev_tx.send(Event::Startup(StartupEvent::ProgressHint(format!("Found local ip address: {}", local_ip)))).await;

    let options = SearchOptions {
        bind_addr: SocketAddr::new(IpAddr::V4(local_ip), 0),
        broadcast_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 1900),
        timeout: Some(Duration::from_secs(5)),
    };

    // Discover Gateway
    let gateway = match search_gateway(options).await {
        Ok(gw) => gw,
        Err(e) => {
            let _ = ev_tx.send(Event::StartupError(StartupError::SearchFailed(e))).await;
            return;
        }
    };

    let _ = ev_tx.send(Event::Startup(StartupEvent::ProgressHint("Gateway found successfully.".into()))).await;

    // add port
    if let Err(e) = gateway.add_port(
        PortMappingProtocol::UDP,
        port,
        SocketAddrV4::new(local_ip, port),
        86400,
        "p2p-rendezvous-server-apt-1003"
    ).await {
        let _ = ev_tx.send(Event::StartupError(StartupError::AddPortFailed(e))).await;
        return;
    }

    let _ = ev_tx.send(Event::Startup(StartupEvent::ProgressHint("Port opened successfully.".into()))).await;

    // get external ip (that people use to connect)
    let public_ip = match gateway.get_external_ip().await {
        Ok(public_ip) => public_ip,
        Err(e) => {
            let _ = ev_tx.send(Event::StartupError(StartupError::GetIpFailed(e))).await;
            return;
        }
    };

    let _ = ev_tx.send(Event::Startup(StartupEvent::ProgressHint("External IP found.".into()))).await;

    let _ = ev_tx.send(Event::Startup(
        StartupEvent::Complete {
            public_ip, local_ip
        }
    )).await;
}