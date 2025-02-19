use csv::StringRecord;
use slug::slugify;
use std::env;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::io::{self, Write};
use std::str::FromStr;
use std::thread;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::fs;

#[derive(Debug)]
#[derive(Default)]
enum Operation {
     #[default]
    NoOp,
    Lowercase,
    Uppercase,
    NoSpaces,
    Slugify,
    Csv,
   }

impl FromStr for Operation {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lowercase" => Ok(Operation::Lowercase),
            "uppercase" => Ok(Operation::Uppercase),
            "no-spaces" => Ok(Operation::NoSpaces),
            "slugify" => Ok(Operation::Slugify),
            "csv" => Ok(Operation::Csv),
            "noop" => Ok(Operation::NoOp),
            _ => Err("Invalid option".into()),
        }
    }
}

struct InputData {
    op: Operation,
    data: String,
}

struct CsvRecord {
    header: Vec<String>,
    data: Vec<StringRecord>,
}

impl CsvRecord {
    fn write_row(&self, f: &mut fmt::Formatter) -> Result<(), std::fmt::Error> {
        for _i in 0..self.header.len() {
            write!(f, "+{:-<16}", "")?;
        }
        write!(f, "+\n")?;
        Ok(())
    }
}

impl Display for CsvRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_row(f)?;
        for header in self.header.iter() {
            write!(f, "|{:^16}", header)?;
        }
        write!(f, "|\n")?;
        self.write_row(f)?;
        for record in self.data.iter() {
            for field in record.iter() {
                write!(f, "|{:^16}", field)?;
            }
            write!(f, "|\n")?;
            self.write_row(f)?;
        }
        Ok(())
    }
}
 

fn usage() {
    println!("Usage: helloworld [lowercase, uppercase, no-spaces, slugify, csv] <input>");
}

fn lowercase_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.is_empty() {
        return Err("Empty input".into());
    }
    Ok(input.to_lowercase())
}

fn uppercase_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.is_empty() {
        return Err("Empty input".into());
    }
    Ok(input.to_uppercase())
}

fn no_spaces_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.is_empty() {
        return Err("Empty input".into());
    }
    Ok(input.replace(" ", ""))
}

fn slugify_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.is_empty() {
        return Err("Empty input".into());
    }
    Ok(slugify(input))
}

fn cvs_me() -> Result<String, Box<dyn Error>> {
    let mut csv_records = CsvRecord {
        header: Vec::new(),
        data: Vec::new(),
    };
    let mut cvs_data = csv::Reader::from_reader(std::io::stdin());
    let headers = cvs_data.headers()?;
    for header in headers {
        csv_records.header.push(header.to_string());
    }
    for result in cvs_data.records() {
        let record = result;
        match record {
            Ok(record) => {
                csv_records.data.push(record);
            }
            Err(e) => eprintln!("Error: {}", e.to_string()),
        }
    }
    println!("{}", csv_records);
    Ok("".to_string())
}


fn parse_input_data(raw_data: &String) -> Result<InputData, Box<dyn Error>> {
    let Some(cmddata) = raw_data.trim().split_once(" ") else {
        return Err("Invalid input".into());
    };
    let input = InputData {
        op:  Operation::from_str(cmddata.0)?,
        data: cmddata.1.to_string(),
    };
    Ok(input)
}


fn process_input(input: InputData) -> Result<String, Box<dyn Error>> {
    let result: Result<String, Box<dyn Error>>;
    let InputData { op, data } = input;

      result = match op {
        Operation::Lowercase => {
            lowercase_me(&data)
        },
        Operation::Uppercase => {
            uppercase_me(&data)
        },
        Operation::NoSpaces => {
            no_spaces_me(&data)
        },
        Operation::Slugify => {
            slugify_me(&data)
        },
        Operation::Csv => cvs_me(),
        Operation::NoOp => {
            Ok("NO OP".to_string())
        },
    };

    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}


fn one_shot_cmd(op: &str, text: String) -> Result<(), Box<dyn Error>> {
     let input_data = InputData{
        op: Operation::from_str(op)?,
        data: text,
    };
    let result = process_input(input_data);
    match result {
        Ok(result) => {
            println!("Result: {}", result);
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
    Ok(())
}


fn reader_thr(txch: Sender<String>) {
    let mut input = String::new();  
    let tx = txch.clone();
    loop {
        let _ = io::stdin().read_line(&mut input);
        println!("Received: {}", input);
        let h = tx.send(input.clone());
        if h.is_err() {
            eprintln!("Error: {}", h.err().unwrap());
            break;
        }
        input.clear();
    }
}

fn read_cvs_file(file: &str) -> Result<(), Box<dyn Error>> {
    let mut csv_records = CsvRecord {
        header: Vec::new(),
        data: Vec::new(),
    };
    let file_rec: String  = fs::read_to_string(file)?;
    let mut cvs_data = csv::Reader::from_reader(file_rec.as_bytes());
    let headers = cvs_data.headers()?;
    for header in headers {
        csv_records.header.push(header.to_string());
    }
    for result in cvs_data.records() {
        let record = result;
        match record {
            Ok(record) => {
                csv_records.data.push(record);
            }
            Err(e) => eprintln!("Error: {}", e.to_string()),
        }
    }
    println!("{}", csv_records);
    Ok(())
}



fn process_thr(rxch: Receiver<String>, run: &AtomicBool) {
    while run.load(Ordering::Relaxed) {
        print!("Waiting for input:>");
        io::stdout().flush().unwrap();

        let raw_input = rxch.recv();
        if raw_input.is_err() {
            eprintln!("Error: {}", raw_input.err().unwrap());
            continue;
        }
        let input_data = parse_input_data(&raw_input.unwrap());
        if input_data.is_err() {
            eprintln!("Error: {}", input_data.err().unwrap());
            continue;
        }
        let result = process_input(input_data.unwrap());
        match result {
            Ok(result) => {
                println!("Result: {}", result);
            },
            Err(e) => {
                eprintln!("Error: {}", e);
            },
        }
    }
}



fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let (txch, rxch) = channel();
    let run_thread = Arc::new(AtomicBool::new(true));
    let run_proc = run_thread.clone();
    // I saw better solution provided by  you in the course, but I leave it as is 
    // I saw crates with better argument parsing, so next time I will use them
    let _ = match args.len() {
        3 => {
            if args[1] == "-f" {
                read_cvs_file(&args[2])?;
                return Ok(());
            }
            one_shot_cmd(&args[1], args[2].clone())
        },
        1 => {
            let rdhandle = thread::Builder::new()
                .name("reader".to_string())
                .spawn(move || reader_thr(txch));
            let wkhandle = thread::Builder::new()
            .name("process".to_string())
            .spawn(move || process_thr(rxch, &run_proc)); 
            let _ =  rdhandle.unwrap().join();
            run_thread.store(false, Ordering::Relaxed);
            let _ = wkhandle.unwrap().join();
            Ok(())
        },
        _ => {
            usage();
            Ok(())
            }
    };
    Ok(())
}
