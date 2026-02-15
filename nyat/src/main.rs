use nyat_core::{
    mapper::MapperBuilder,
    net::{LocalAddr, RemoteAddr},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), nyat_core::Error> {
    let local = LocalAddr::new("0.0.0.0:8080".parse().unwrap());
    let stun = RemoteAddr::from_host("turn.cloudflare.com", 3478, None);

    let mapper = MapperBuilder::new_udp(local, stun)
        .check_per_tick(5.try_into().unwrap())
        .build();

    mapper
        .run(&mut |pub_addr| {
            println!("{pub_addr}");
        })
        .await?;
    Ok(())
}
