
use anyhow::{anyhow, bail, Context, Result};
use tokio::net::{TcpStream};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use std::io::{stdin,stdout,Write};
use std::thread;
use std::time::{Duration};
use chrono::{Local};

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:11211").await?;
    let exptime: i64 = 1;
    let message = format!("set key 0 {} 7\r\nbigdata\r\n", exptime);
    stream.write_all(message.as_bytes()).await.context("Failed to write to server")?;
    println!("Written 2 server");
    let mut stream = BufReader::with_capacity(1024, stream);
    let mut buf = String::new();

    println!("buf made");
    
    stream.read_line(&mut buf).await.context("Failed to read from socket")?;

    println!("Got back: {}", buf);

    thread::sleep(Duration::from_secs(2));

    let mut stream = TcpStream::connect("127.0.0.1:11211").await?;
    stream.write_all(b"get key\r\n").await.context("Failed to write to server")?;
    println!("Written 2 server 2");
    let mut stream = BufReader::with_capacity(1024, stream);
    let mut buf = String::new();

    println!("buf made 2");
    
    stream.read_line(&mut buf).await.context("Failed to read from socket 2")?;
    println!("Got back: {}", buf);
     


    // while true {
    //     let mut stream = TcpStream::connect("127.0.0.1:11211").await?;
    //     let mut first = String::new();
    //     let mut second = String::new();

    //     let b1 = std::io::stdin().read_line(&mut first).unwrap();
    //     let b2 = std::io::stdin().read_line(&mut second).unwrap();

    //     let mut s1 = first.trim_end().to_string();
    //     let s2 = second.trim_end().to_string();

    //     s1.push_str("\r\n");
    //     s1.push_str(&s2);
    //     s1.push_str("\r\n");

    //     println!("{}", s1);

    
    //     stream.write_all(s1.as_bytes()).await.context("Failed to write to server")?;
    //     println!("Written 2 server");
    //     let mut stream = BufReader::with_capacity(1024, stream);
    //     let mut buf = String::new();

    //     println!("buf made");
        
    //     stream.read_line(&mut buf).await.context("Failed to read from socket")?;

    //     println!("Got back: {}", buf);    
    // }

    Ok(())

}