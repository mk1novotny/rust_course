use slug::slugify;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: helloworld [lowercase, uppercase, no-spaces, slugify] <input>");
        return;
    }
    
    let input: String = if args.len() > 3 {
        let mut input: String = "".to_string(); 
        for i in 2..args.len() {
            input.push(' ');
            input.push_str( &args[i]);
        }
        input
    } else {
        args[2].clone()
    };

    let result: String = match args[1].as_str() {
        "lowercase" => {
            input.to_lowercase()
        },
        "uppercase" => {
            input.to_uppercase()

        },
        "no-spaces" => {
            input.replace(" ", "")
            
        },
        "slugify" => {
            slugify(&input)
        },
        _ => {
            "Unknown command".to_string()
        }
    };
    println!("{}", result);
}  
