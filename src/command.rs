use flate2::read::ZlibDecoder;
use sha1::Digest;
use sha1::Sha1;
use std::fs;
use std::io::Read;
use std::io::Write;

pub fn dispatch_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args[1].as_str() {
        "init" => cmd_init(args),
        "cat-file" => cmd_cat_file(args),
        "hash-object" => cmd_hash_object(args),
        _ => {
            println!("unknown command: {}", args[1]);
            Ok(())
        }
    }
}

fn cmd_init(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}

fn cmd_cat_file(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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

fn cmd_hash_object(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args[2] != "-w" {
        eprintln!("Usage: hash-object -w <file>");
        return Ok(());
    }
    let file_path = &args[3];
    let data = fs::read(file_path)?;
    let header = format!("blob {}\0", data.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(&data);
    let hash = hasher.finalize();
    let hash_str = format!("{hash:x}");
    let object_path = format!(".git/objects/{}/{}", &hash_str[0..2], &hash_str[2..]);
    fs::create_dir_all(format!(".git/objects/{}", &hash_str[0..2]))?;
    let mut encoder = flate2::write::ZlibEncoder::new(
        fs::File::create(object_path)?,
        flate2::Compression::default(),
    );
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(&data)?;
    encoder.finish()?;
    println!("{hash_str}");
    Ok(())
}
