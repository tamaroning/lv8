mod driver;
mod runtime;

fn main() {
    match driver::run() {
        Ok(code) => {
            std::process::exit(code);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
