use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;

pub fn dispatch_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args[1].as_str() {
        "init" => cmd_init(args),
        "cat-file" => cmd_cat_file(args),
        _ => {
            println!("unknown command: {}", args[1]);
            Ok(())
        }
    }
}

pub fn cmd_init(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}

pub fn cmd_cat_file(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}
