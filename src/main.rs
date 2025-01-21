use std::io;
use std::io::Write;

fn main() {
    println!("Hello, world!");
    let mut input = String::new();
    print!("Type something: ");
    io::stdout().flush().unwrap();
    let res = io::stdin().read_line(&mut input);
    match res {
        Ok(_) => println!("Read line successfully"),
        Err(e) => println!("Error reading line: {}", e),
    }
    println!("You typed: {}", input);
}
