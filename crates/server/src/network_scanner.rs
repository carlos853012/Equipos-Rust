
use std::net::IpAddr;
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use anyhow::Result;
use tokio::time::timeout;
use std::sync::Arc;

pub struct NetworkScanner {
    client: Client,
}

impl NetworkScanner {
    pub fn new() -> Result<Self> {
        let config = Config::default();
        let client = Client::new(&config)?;
        Ok(Self { client })
    }

    pub async fn ping(&self, ip: &str) -> bool {
        let ip_addr: IpAddr = match ip.parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        };

        let mut pinger = self.client.pinger(ip_addr, PingIdentifier(0)).await;
        pinger.timeout(Duration::from_millis(800));

        // Corregido: pinger.ping() devuelve un Future, no necesitamos el .await interno dentro de timeout
        match timeout(Duration::from_secs(1), pinger.ping(PingSequence(0), &[])).await {
            Ok(Ok(_)) => true, // Si recibimos cualquier respuesta válida de ICMP
            _ => false,
        }
    }
}

pub async fn run_scan(scanner: Arc<NetworkScanner>, ips: Vec<String>) -> Vec<(String, bool)> {
    use futures::stream::{self, StreamExt};
    
    let limit = 40; // Límite de concurrencia óptimo y seguro para redes industriales

    stream::iter(ips)
        .map(|ip| {
            let scanner_clone = Arc::clone(&scanner);
            async move {
                let status = scanner_clone.ping(&ip).await;
                (ip, status)
            }
        })
        .buffer_unordered(limit)
        .collect::<Vec<(String, bool)>>()
        .await
}
