use nyat_core::{
    mapper::MapperBuilder,
    net::{LocalAddr, RemoteAddr},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), nyat_core::Error> {
    let local = LocalAddr::new("0.0.0.0:8080".parse().unwrap());
    let stun = RemoteAddr::from_host("turn.cloudflare.com", 3478, None);
    let remote = RemoteAddr::from_host("bing.com", 80, None);

    let mapper = MapperBuilder::new(local, stun)
        .tcp_remote(remote)
        .check_per_tick(5.try_into().unwrap())
        .build_udp();
    mapper
        .run(|pub_addr| {
            println!("{pub_addr}");
        })
        .await?;
    Ok(())
}
