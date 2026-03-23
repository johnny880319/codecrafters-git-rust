use anyhow::Context;
use anyhow::Result;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::Digest;
use sha1::Sha1;
use std::fs;
use std::io::Read;
use std::io::Write;

pub fn dispatch_command(args: &[String]) -> Result<()> {
    match args[1].as_str() {
        "init" => cmd_init(args),
        "cat-file" => cmd_cat_file(args),
        "hash-object" => cmd_hash_object(args),
        "ls-tree" => cmd_ls_tree(args),
        _ => {
            println!("unknown command: {}", args[1]);
            Ok(())
        }
    }
}

fn cmd_init(_args: &[String]) -> Result<()> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}

fn cmd_cat_file(args: &[String]) -> Result<()> {
    if args[2] != "-p" {
        eprintln!("Usage: cat-file -p <object>");
        return Ok(());
    }
    let object_hash = &args[3];
    let object_path = format!(".git/objects/{}/{}", &object_hash[0..2], &object_hash[2..]);
    let mut decoder = ZlibDecoder::new(fs::File::open(object_path)?);
    let mut contents = String::new();
    decoder.read_to_string(&mut contents)?;
    let (_, contents) = contents.split_once('\0').context("Invalid object format")?;
    print!("{contents}");
    Ok(())
}

fn cmd_hash_object(args: &[String]) -> Result<()> {
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

    let mut encoder = ZlibEncoder::new(fs::File::create(object_path)?, Compression::default());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(&data)?;
    encoder.finish()?;
    println!("{hash_str}");
    Ok(())
}

fn cmd_ls_tree(args: &[String]) -> Result<()> {
    let name_only = args[2] == "--name-only";
    let tree_sha = args.last().context("Missing tree SHA")?;

    let object_path = format!(".git/objects/{}/{}", &tree_sha[0..2], &tree_sha[2..]);
    let mut decoder = ZlibDecoder::new(fs::File::open(object_path)?);
    let mut contents = Vec::new();
    decoder.read_to_end(&mut contents)?;
    let entries = decode_tree_object(&contents)?;
    if name_only {
        for entry in entries {
            println!("{}", entry.name);
        }
    } else {
        for entry in entries {
            println!(
                "{} {} {}\t{}",
                entry.mode, entry.entry_type, entry.sha, entry.name
            );
        }
    }
    Ok(())
}

struct TreeEntry {
    mode: String,
    entry_type: String,
    sha: String,
    name: String,
}

/// Returns a list of (mode, tree/blob, sha, name) entries
fn decode_tree_object(contents: &[u8]) -> Result<Vec<TreeEntry>> {
    let (_, mut contents) = split_at_byte(contents, 0)?;

    let mut result = Vec::new();
    while !contents.is_empty() {
        let (mode, rest) = split_at_byte(contents, b' ')?;
        let mode = std::str::from_utf8(mode)?;
        let (name, rest) = split_at_byte(rest, 0)?;
        let name = std::str::from_utf8(name)?;
        let sha = &rest[0..20];
        contents = &rest[20..];

        let entry_type = if mode == "40000" { "tree" } else { "blob" };
        result.push(TreeEntry {
            mode: mode.to_string(),
            entry_type: entry_type.to_string(),
            sha: hex::encode(sha),
            name: name.to_string(),
        });
    }
    Ok(result)
}

fn split_at_byte(contents: &[u8], byte: u8) -> Result<(&[u8], &[u8])> {
    let pos = contents
        .iter()
        .position(|&b| b == byte)
        .context("Invalid object format")?;
    Ok(contents.split_at(pos))
}
