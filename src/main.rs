use anyhow::{anyhow, bail, Context, Result};

use std::env;

mod server;

use server::Server;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    eprintln!("args: {:?}", args);
    let port = if args.len() > 1{
        parse_args(args)?
    } else{
        "11211".to_string()
    };
    eprintln!("port set to: {}", port);

    let mut server = Server::new(5, port);

    server.run().context("Server Error")?;

    Ok(())

}

fn parse_args(args: Vec<String>) -> Result<String> {
    if args.len() == 3{
        if args[1] == "-p" {
            match args[2].parse::<u16>() {
                Ok(_) => return Ok(args[2].clone()),
                _ => bail!("Invalid port number (expect range 0 to 65535)")
            }
        }
    }
    return Err(anyhow!("memcached-server arguments: -p [port]"))
}

