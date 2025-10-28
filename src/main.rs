// Ether: High-Performance language focused on machine learning and AI

// ============================================================================
// CLI / ENTRYPOINT
// ============================================================================

use std::{
  env::{
    self,
    consts::{ARCH, OS},
  },
  io::{self, Write},
  process::{self, exit},
};

use ether::Ether;

fn main() {
  let args: Vec<String> = env::args().collect();

  match args.len() {
    1 => run_repl(),
    2 => run_file(&args[1]),
    _ => {
      eprintln!("Usage: {} [script]", args[0]);
      process::exit(1);
    }
  }
}

fn run_repl() {
  println!(
    "Ether v0.1.0 REPL/Interactive mode = ON; running on {}/{}",
    OS, ARCH
  );
  println!("Type '.help' for help, '.exit' to quit");
  println!();

  let mut lang = Ether::new();
  let mut input_buffer = String::new();
  let prompt = ">>> ";

  loop {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    input_buffer.clear();
    match io::stdin().read_line(&mut input_buffer) {
      Ok(0) => break, // EOF
      Ok(_) => {
        let input = input_buffer.trim();

        if input.is_empty() {
          continue;
        }

        match input {
          ".exit" | ".quit" => break,
          ".help" => print_help(),
          ".clear" => {
            print!("\x1B[2J\x1B[1;1H");
            continue;
          }
          _ => match lang.execute_repl(input) {
            Ok(_) => {}
            Err(e) => {
              eprintln!("Error: {}", e);
            }
          },
        }
      }
      Err(e) => {
        eprintln!("Error reading input: {}", e);
        break;
      }
    }
  }

  println!("\nGoodbye!");
}

fn run_file(path: &str) {
  let mut lang = Ether::new();

  match lang.execute_file(path) {
    Ok(_) => exit(0),
    Err(e) => {
      eprintln!("Error: {}", e);
      process::exit(1);
    }
  }
}

fn print_help() {
  println!("Commands:");
  println!("  .help    - Show this help message");
  println!("  .clear   - Clear the screen");
  println!("  .exit    - Exit the REPL");
  println!("  .quit    - Exit the REPL");
  println!();
}
