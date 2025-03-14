use anyhow::{Context, Result};
use clap::{self, Parser};
use clap_repl::reedline::{DefaultPrompt, DefaultPromptSegment, Reedline};
use clap_repl::ClapEditor;
use serde_cbor::{from_slice, to_vec};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File { name: String, data: Vec<u8> },
}

impl MessageType {
    fn serialize_cbor(&self) -> Result<Vec<u8>> {
        Ok(to_vec(&self)?)
    }

    fn deserialize_cbor(data: &[u8]) -> Result<Self> {
        Ok(from_slice(data)?)
    }

    fn send_messages(&self, stream: &mut TcpStream) -> Result<()> {
        let data = self.serialize_cbor()?;
        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(&data)?;
        Ok(())
    }

    fn read_message(stream: &mut TcpStream) -> Result<Self> {
        let mut len = [0u8; 4];
        stream.read_exact(&mut len)?;
        let len = u32::from_be_bytes(len) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data)?;
        let message = Self::deserialize_cbor(&data)?;
        Ok(message)
    }
}

fn srv_handle(stream: &mut TcpStream, clients: &Arc<RwLock<HashMap<SocketAddr, TcpStream>>>) {
    loop {
        let Ok(addr) = stream.peer_addr() else {
            eprintln!(" no address");
            continue;
        };
        let message = MessageType::read_message(stream);
        match &message {
            Ok(MessageType::Text(text)) => {
                println!("text: {}", text);
            }
            Ok(MessageType::Image(data)) => {
                println!("image: {} bytes", data.len());
            }
            Ok(MessageType::File { name, data }) => {
                println!("file: {} bytes", data.len());
                std::fs::write(name, data).unwrap();
            }
            Err(e) => {
                eprintln!("unable read from stream {:?}", e);
                let _ = stream.shutdown(Shutdown::Both);
                break;
            }
        }

        for client in clients.write().unwrap().values_mut() {
            let Ok(cl_addr) = client.peer_addr() else {
                continue;
            };
            if addr == cl_addr {
                continue;
            };
            match &message {
                Ok(m) => {
                    let resp = m.send_messages(client);
                    match resp {
                        Ok(_) => {}
                        Err(ref e)
                            if e.downcast_ref::<std::io::Error>().unwrap().kind()
                                == io::ErrorKind::UnexpectedEof =>
                        {
                            eprintln!("End of file before the filing buffer");
                            clients.write().unwrap().remove(&cl_addr);
                            let _ = client.shutdown(Shutdown::Both);
                            continue;
                        }
                        Err(e) => {
                            eprintln!("send message {e}");
                            let _ = client.shutdown(Shutdown::Both);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("unable write to stream {:?}", e);
                    let _ = client.shutdown(Shutdown::Both);
                    break;
                    //Err(anyhow::anyhow!("Client is closed"))
                }
            };
        }
    }
}

fn server(host: &str, port: u16) -> Result<()> {
    let tcp_listener = TcpListener::bind((host, port))?;
    let clients: HashMap<SocketAddr, TcpStream> = HashMap::new();
    let rc_client = Arc::new(RwLock::new(clients));
    let clients_cloned = rc_client.clone();

    for stream in tcp_listener.incoming() {
        let Ok(stream) = stream else {
            eprintln!("failed: get stream");
            continue;
        };
        let Ok(addr) = stream.peer_addr() else {
            eprintln!("failed: get port");
            continue;
        };
        println!("new client: {:?}", addr);
        {
            let cl = clients_cloned.clone();
            cl.write()
                .unwrap()
                .insert(addr, stream.try_clone().unwrap());
        }
        let cl_cl = clients_cloned.clone();
        let _srv_handle = thread::Builder::new()
            .name("server".to_owned())
            .spawn(move || {
                srv_handle(&mut stream.try_clone().unwrap(), &cl_cl);
            });
    }
    Ok(())
}

fn client(rxch: Receiver<MessageType>, host: &str, port: u16) -> Result<()> {
    let stream = TcpStream::connect((host, port))?;
    let mut reader = stream.try_clone()?;
    let mut writer = stream.try_clone()?;

    let _reader_thread = thread::spawn(move || -> Result<()> {
        loop {
            let response = MessageType::read_message(&mut reader)?;
            match response {
                MessageType::Text(text) => println!("\n\rReceive text: {}", text),
                MessageType::Image(data) => {
                    std::fs::create_dir_all("images")?;
                    println!("Receiving image ....");
                    let now = chrono::Local::now();
                    let fmt_date = now.format("%Y-%m-%d-%H-%M-%S").to_string();
                    std::fs::write(format!("images/{fmt_date}.png"), data)?;
                }
                MessageType::File { name, data } => {
                    println!("Receiving file ....");
                    std::fs::create_dir_all("files")?;
                    std::fs::write(format!("files/{name}"), data)?;
                }
            }
        }
    });

    let _writer_thread = thread::spawn(move || -> Result<()> {
        loop {
            let message = rxch.recv()?;
            message.send_messages(&mut writer)?;
        }
    });
    Ok(())
}

#[derive(Parser, Debug)]
#[command(version = "0.0.1", author = "Marek N", about = "lesson 09 home work", long_about = None)]
struct CliArgs {
    #[arg(short, long, default_value = "localhost")]
    address: String,
    #[arg(short, long, default_value = "11111")]
    port: u16,
    #[arg(short, long, default_value = "server")]
    mode: String,
}

#[derive(Debug, Parser)]
#[command(name = "")]
enum Commands {
    #[command(about = "send file")]
    Filename { path: String },
    #[command(about = "send image")]
    Image { path: String },
    #[command(about = "send text")]
    Text { text: String },
    #[command(about = "quit")]
    Quit,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();
    let (txch, rxch) = channel();

    match args.mode.as_str() {
        "server" => {
            server(&args.address, args.port)?;
            println!("server done");
        }
        "client" => {
            let _thr_handle =
                thread::Builder::new()
                    .name("client".to_string())
                    .spawn(move || -> Result<()> {
                        client(rxch, &args.address, args.port)?;
                        Ok(())
                    });
            let prompt = DefaultPrompt {
                left_prompt: DefaultPromptSegment::Basic("cmd".to_owned()),
                ..DefaultPrompt::default()
            };
            let rl = ClapEditor::<Commands>::builder()
                .with_prompt(Box::new(prompt))
                .build();
            rl.repl(|command| match command {
                Commands::Filename { path } => {
                    let data = std::fs::read(&path);
                    if data.is_err() {
                        eprintln!("unable to read file");
                    } else {
                        let message = MessageType::File {
                            name: path,
                            data: data.unwrap(),
                        };
                        let tx = txch.clone();
                        let e = tx.send(message.clone());
                        if e.is_err() {
                            eprintln!("send error {:?}", e.err().unwrap())
                        }
                    }
                }
                Commands::Image { path } => {
                    let data = std::fs::read(&path);
                    if data.is_err() {
                        eprintln!("unable to read file");
                    } else {
                        let message = MessageType::Image(data.unwrap());
                        let tx = txch.clone();
                        let e = tx.send(message.clone());
                        if e.is_err() {
                            eprintln!("send error {:?}", e.err().unwrap())
                        }
                    }
                }
                Commands::Text { text } => {
                    let message = MessageType::Text(text);
                    let tx = txch.clone();
                    let e = tx.send(message.clone());
                    if e.is_err() {
                        eprintln!("send error {:?}", e)
                    }
                }
                Commands::Quit => {
                    println!("quit");
                    std::process::exit(0);
                }
            });
        }
        _ => {
            eprintln!("unknown mode");
            Err(anyhow::anyhow!("unknown mode"))
        }?,
    };
    Ok(())
}
