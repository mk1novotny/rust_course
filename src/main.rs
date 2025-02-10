use slug::slugify;
use std::error::Error;
use std::io;
use std::env;
use std::fmt;
use std::fmt::Display;
use csv::StringRecord;

fn usage() {
    println!("Usage: helloworld [lowercase, uppercase, no-spaces, slugify, csv] <input>");

}


fn lowercase_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.len() == 0 {
        return Err("Empty input".into());
    }
   Ok(input.to_lowercase())
}

fn uppercase_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.len() == 0 {
        return Err("Empty input".into());
    }
    Ok(input.to_uppercase())
}

fn no_spaces_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.len() == 0 {
        return Err("Empty input".into());
    }
    Ok(input.replace(" ", ""))
}

fn slugify_me(input: &String) -> Result<String, Box<dyn Error>> {
    if input.len() == 0 {
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
            },
            Err(e) => eprintln!("Error: {}", e.to_string()),
        }
    }
    println!("{}", csv_records);
    Ok("".to_string())
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
        return Err("Invalid number of arguments".into());
    }
       
    let mut input = String::new();
    
    let result: Result<String, Box<dyn Error>>  = match args[1].as_str() {
    "lowercase" => {
            io::stdin().read_line(&mut input)?;
            lowercase_me(&input)
        },
    "uppercase" => {
            io::stdin().read_line(&mut input)?;
            uppercase_me(&input)
        },
    "no-spaces" => {
            io::stdin().read_line(&mut input)?;
            no_spaces_me(&input)
        },
    "slugify" => {
            io::stdin().read_line(&mut input)?;
            slugify_me(&input)
        },
    "csv" => {
            cvs_me()
        },
    _ => {
            usage();
           Err("Invalid option".into()) 
        }
    };

    match &result {
        Ok(result) => println!("{}", result.clone()),
        Err(e) => eprintln!("Error: {}", e.to_string()),
    }

    // println!("{}", result.unwrap());
    Ok(())

}  
