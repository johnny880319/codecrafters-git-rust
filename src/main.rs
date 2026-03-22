use flate2::read::ZlibDecoder;
use std::env;
use std::fs;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git")?;
        fs::create_dir(".git/objects")?;
        fs::create_dir(".git/refs")?;
        fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
        println!("Initialized git directory");
    } else if args[1] == "cat-file" {
        if args[2] != "-p" {
            eprintln!("Usage: cat-file -p <object>");
            return Ok(());
        }
        let object_hash = &args[3];
        let object_path = format!(".git/objects/{}/{}", &object_hash[0..2], &object_hash[2..]);
        let mut decoder = ZlibDecoder::new(fs::File::open(object_path)?);
        let mut contents = String::new();
        decoder.read_to_string(&mut contents)?;
        let (_, contents) = contents.split_once('\0').ok_or("Invalid object format")?;
        print!("{contents}");
    } else {
        println!("unknown command: {}", args[1]);
    }
    Ok(())
}
