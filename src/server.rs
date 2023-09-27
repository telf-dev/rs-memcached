// use tokio::net::{TcpListener, TcpSocket, TcpStream};
// use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
// use tokio::task::{self, JoinHandle};
//use tokio::sync::mpsc::{self, Receiver, Sender};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, BufReader};
use std::thread::{self};
use std::sync::mpsc::{self};

use std::collections::{HashMap, LinkedList};
use std::sync::{Arc, Mutex};

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeZone, Utc,
    offset::Local};

const SERVER_ADDR: &str = "127.0.0.1";

const STORED: &[u8] = b"STORED\r\n";
const NOT_STORED: &[u8] = b"NOT STORED\r\n";

pub struct Data {
    pub flags: u16,
    pub exptime: i64, //TODO: alter to be a proper time
    pub bytes: usize,
    pub data: String,
}

impl Data {
    pub fn new(flags: u16, exptime: i64, bytes: usize, data: String) -> Data{
        Data { flags, exptime, bytes, data }
    }
}


//TODO: likely pointless, remove
struct Worker {
    task: thread::JoinHandle<()>,
}
impl Worker{
    fn new(task: thread::JoinHandle<()>) -> Worker {
        Worker{
            task,
        }
    }
}

type RecordTable=  Arc<Mutex<HashMap<String, Arc<Data>>>>;
type RecordList = Arc<Mutex<LinkedList<Data>>>;

// TODO: Make more robust to memory limit of current system: want to be able to fit threadpool, server AND requested mem size in memory
struct ThreadPool {
    pool: Vec<Worker>,
    cache_mem: Arc<MemoryHandler>,
    tx: mpsc::Sender<(TcpStream, fn(TcpStream, RecordList, RecordTable) -> Result<()>)>,
}
impl ThreadPool {
    fn new(s: usize, memory: Arc<MemoryHandler>) -> ThreadPool {
        //TODO: find out whether bounded/unbounded number of tasks is better (sync_channel blocks after certain number)
        let (tx, rx) = mpsc::channel::<(TcpStream, fn(TcpStream, RecordList, RecordTable) -> Result<()>)>();
        let rx = Arc::new(Mutex::new(rx));
        let mut pool: Vec<Worker> = vec![];

        for i in 0..s {
            let thread_rx = Arc::clone(&rx);
            let thread_memory = Arc::clone(&memory);
            let handle = thread::spawn( move ||{
                loop{
                    let (stream, job) = thread_rx.lock().unwrap().recv().unwrap();
                    job(stream, Arc::clone(&record_list), Arc::clone(&record_table)).context(format!("Thread {i} failed to process a job")).unwrap();

                }
            });
            pool.push(Worker::new(handle));
        }
        
        ThreadPool {
            pool,
            cache_mem: memory,
            tx,
        }
        
    }

    fn execute(&self, stream: TcpStream, f: fn(TcpStream, RecordList, RecordTable) -> Result<()>)
    {
        self.tx.send((stream, f)).unwrap();
    }
}

pub struct MemoryHandler{
    max_size: usize,
    mem_size: usize,
    recordptrs: HashMap<String, Data>,
    records: LinkedList<Data>,
}

impl MemoryHandler {
    fn new( max_size: usize, recordptrs: HashMap<String, Data>, records: LinkedList<Data>) -> MemoryHandler{
        MemoryHandler {
            max_size,
            mem_size: 0,
            recordptrs,
            records,
        }
    }

    fn push_new(record: Data, key: &str){
        
    }
    fn update(record: Data, key: &str){

    }


}

// TODO: Option to pin to core? Would avoid memory dumps; check whether most (?server?) systems use 
// shared RAM across cores or not.
pub struct Server{
    port: String,
    memory: Arc<MemoryHandler>,
    threadpool: ThreadPool,
}

impl<'a> Server{
    pub fn new(s: usize, max_size: usize, port: String) -> Server {   
        let mem = Arc::new(MemoryHandler::new(max_size, HashMap::new(), LinkedList::new()));
        Server {
            port,
            memory: Arc::clone(&mem),
            threadpool: ThreadPool::new(s, Arc::clone(&m), Arc::clone(&r)),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", SERVER_ADDR, self.port))
                .context(format!("Could not bind to port {}", self.port))?;
            
        loop {
            let (socket, sockaddr) = listener.accept().context("Failed to accept connection")?;
            
            self.threadpool.execute(socket, process);

            //self.process(&mut socket).await.context("Failed to process socket")?;
        }
    }

}

fn process(socket: TcpStream, memory: Arc<MemoryHandler>) -> Result<()> {
    eprintln!("Processing!");
    let mut buffered_socket = BufReader::with_capacity(1024, socket);
    let mut buf:[u8; 1024] = [0; 1024];
    
    //Need to read until EOF, sort out vulnerabilty later
    let size = buffered_socket.read(&mut buf).context("Failed to read from socket")?;
    let buf = String::from_utf8(buf[0..size].to_vec()).unwrap();

    eprintln!("Read {size} bytes from socket!");
    
    let mut socket = buffered_socket.into_inner();

    //TODO: proper check for \r\n rather than this + the read above; if there's only one line, can end in EOF or nothing at all which 
    //is an error
    let command: Vec<Arc<str>> = buf.split_terminator("\r\n").map(|s| s.into()).collect();
    if command.len() < 1 || command.len() > 3{
        socket.write_all(b"Error: Invalid data format")?;
    }

    //Get the components of the first line of the command
    let args: Vec<Arc<str>> = command[0].split(' ').map(|s| s.into()).collect();
    eprintln!("args:");
    for part in args.iter() {
        eprintln!("{}", part);
    }
    for part in command.iter(){
        eprint!("{}", part)
    }
    println!("command length: {}", command.len());

    //TODO: Add remaining commmand types
    if command.len() == 2{
        //Storage Comands
        if args.len() != 5 && args.len() != 6 {
            socket.write_all(b"Error: Malformed data request")?;
            return Ok(())
        }
        match &*args[0] {
            "set" => store(args, &command[1], socket, records).context("Set command failed")?,
            _ => {
                socket.write_all(b"Error: Invalid command")?;
                return Ok(())
            }
        }
    }
    else{
        match &*args[0] {
            "get" => get(args, socket, records).context("Get command failed")?,
            _ => {
                socket.write_all(b"Error: Invalid command")?;
                return Ok(())
            }
        }
    }
    
    eprintln!("Written back to socket!");
    Ok(())
}

// TODO: Add cas command (check if exists and was modified since last retrieval, and set)
fn store(args: Vec<Arc<str>>, data: &str, mut socket: TcpStream, records: Arc<MemoryHandler>) -> Result<()> {
    if args[1].len() > 250 {
        return write_error(b"Error: Key too large (>250 bytes))", socket);
    }

    let bytes = match args[4].parse::<usize>() {
        Ok(x) => x,
        _ => {
            return write_error(b"Error: data length must be in range 0 - 4294967295", socket);
        }
    };

    eprintln!("bytes: {}\ndata length: {}", bytes, data.len());

    if data.len() != bytes {
        return write_error(b"Error: Mismatch between stated and actual length of data", socket);
    }

    let flags = match args[2].parse::<u16>() {
        Ok(x) => x,
        _ => {
            return write_error(b"Error: provided flag must be in range 0 - 65535", socket);
        }
    };
    
    //We assume that exptime is given as an offset in seconds for the future, add to UNIX timestamp
    //of current time
    let mut exptime = match (&(*(args[3]))).parse::<i64>() {
        Ok(x) => {
            if x == 0 { 
                std::i64::MAX
            } else if x > 0{
                x + Local::now().timestamp()
            } else{
                //Instantly expires
                return Ok(())
            }
        },
        _ => return write_error(b"Error: Invalid UTC timestamp", socket)
    };

    // TODO: Sort out add/replace duplicate code
    let command = &(*args[0]);
    let mut response: &[u8] = match command {
        "set" => match records.lock().unwrap().insert((*args[1]).to_string(), 
            Data::new(flags, exptime, bytes, data.to_string())) {
                Some(x) => {STORED},
                _ => return server_error("Failed to store data", socket)
            }
        "add" => {
            let mut storage = records.lock().unwrap();
            if !storage.contains_key(&(*args[1])) {
                storage.insert((*args[1]).to_string(), 
                Data::new(flags, exptime, bytes, data.to_string()));
                STORED
            } else{ NOT_STORED }
        }
        "replace" =>{
            let mut storage = records.lock().unwrap();
            if storage.contains_key(&(*args[1])){
                storage.insert((*args[1]).to_string(), 
                Data::new(flags, exptime, bytes, data.to_string()));
                STORED
            } else{ NOT_STORED }
        }
        "append" => {
            let mut storage = records.lock().unwrap();
            if let Some(record) = storage.get_mut(&(*args[1])) {
                let idx = record.data.len();
                match update_data(idx, record, bytes, data) {
                    Ok(_) => STORED,
                    err => return err
                }
            } else {
                return client_error(&format!("Key {} does not exist in database", &(*args[1])), socket);
            }

        }
        "prepend" => {
            let mut storage = records.lock().unwrap();
            if let Some(record) = storage.get_mut(&(*args[1])) {
                let idx = 0;
                match update_data(idx, record, bytes, data) {
                    Ok(_) => STORED,
                    err => return err
                }
            } else {
                return client_error(&format!("Key {} does not exist in database", &(*args[1])), socket);
            }
        }
        _ => return client_error("Command does not exist", socket)
    };

    if args.len() == 6 { 
            if &*args[5] == "1" {
                return Ok(())
            }
            else {
                return write_error(b"Error: noreply must be a 0", socket);
            } 
    }
    
    socket.write_all(response).context("Failed to send response to set")?;

    Ok(())
}

fn get(args: Vec<Arc<str>>, mut socket: TcpStream, records: Arc<MemoryHandler>) -> Result<()> {
    if args[1].len() > 250 {
        return write_error(b"Error: Key too large (>250 bytes))", socket);
    }

    let mut remove = false;
    match records.lock().unwrap().get(&*args[1]) {
        Some(data) => {
            if check_timestamp(data.exptime){
                println!("here 2");
                socket.write_all(format!("VALUE {} {} {}", data.data, data.flags, data.bytes).as_bytes()).context("Failed to write response to GET")?;
            } else {
                socket.write_all(b"End").context("Failed to write response to GET")?;
                remove = true; 
            }

        },
        None => socket.write_all(b"End").context("Failed to write response to GET")?,
    }
    if remove {
        records.lock().unwrap().remove(&*args[1]).context("Failed to remove expired data")?;
    }

    Ok(())
}

/*
    If the records's key is already in the queue, remove the record from the queue + requeue
    Pop as many items in the queue as are necessary to keep memory usage below max
    Need to take into account:
        - Total size of other records
        - Extra memory used by hashmap
        - Extra memory used by list pointers
    All of the above can be calculated quickly, just need to get data length from new record
    and records to be popped
    TODO: slab-based memory alloc
*/
fn resolve_queue(mem_size: Arc<Mutex<usize>>) {

}

/*
    Append or prepend data to a record
    TODO: modify so if idx = 0, concat to existing data instead of insert if this would be faster
*/
fn update_data(idx: usize, record: &mut Data, bytes: usize, data: &str) -> Result<()> {
    if record.bytes + bytes <= record.bytes {
        record.bytes += bytes;
        record.data.insert_str(idx, data);
        return Ok(())
    }
    return Err(anyhow!("Append would make data too large"));
}

/*
    Check whether a record has expired
*/
fn check_timestamp(exptime: i64) -> bool {
    println!("in check_timestamp");
    return exptime >= Local::now().timestamp()
}

/*
    Send a generic error message to the client
    TODO: Convert all of these into client errors/server errors
*/
fn write_error(s: &[u8], mut socket: TcpStream) -> Result<()>{
    socket.write_all(s).context("Failed to send error message")?;
    Ok(())
}

/*
    Report client error e.g. mismatch between reported and actual data lengths
*/
fn client_error(s: &str, mut socket: TcpStream) -> Result<()> {
    socket.write_all(format!("CLIENT ERROR {}\r\n", s).as_bytes()).context("Failed to send error message")?;
    Ok(())
}

/*
    Report server error to client e.g. couldn't access hash table
*/
fn server_error(s: &str, mut socket: TcpStream) -> Result<()> {
    socket.write_all(format!("SERVER ERROR {}\r\n", s).as_bytes()).context("Failed to send error message")?;
    Ok(())
}
