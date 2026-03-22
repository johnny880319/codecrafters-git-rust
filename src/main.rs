use flate2::read::GzDecoder;
use std::env;
use std::fs;
use std::io::Read;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory");
    } else if args[1] == "cat-file" {
        if args[2] != "-p" {
            eprintln!("Usage: cat-file -p <object>");
            return;
        }
        let object_hash = &args[3];
        let object_path = format!(".git/objects/{}/{}", &object_hash[0..2], &object_hash[2..]);
        let mut decoder = GzDecoder::new(fs::File::open(object_path).unwrap());
        let mut contents = String::new();
        decoder.read_to_string(&mut contents).unwrap();
        println!("{contents}");
    } else {
        println!("unknown command: {}", args[1]);
    }
}
