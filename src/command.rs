use anyhow::Context;
use anyhow::Result;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::Digest;
use sha1::Sha1;
use std::fs;
use std::fs::DirEntry;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

pub fn dispatch_command(args: &[String]) -> Result<()> {
    match args[1].as_str() {
        "init" => cmd_init(args),
        "cat-file" => cmd_cat_file(args),
        "hash-object" => cmd_hash_object(args),
        "ls-tree" => cmd_ls_tree(args),
        "write-tree" => cmd_write_tree(args),
        "commit-tree" => cmd_commit_tree(args),
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
    let contents = read_object(object_hash)?;
    let (_, contents) = split_at_byte(&contents, 0)?;
    print!("{}", std::str::from_utf8(contents)?);
    Ok(())
}

/// Returns the SHA-1 hash of the blob object created from the file at `file_path`.
/// ```
/// blob <size>\0<content>
/// ```
fn cmd_hash_object(args: &[String]) -> Result<()> {
    if args[2] != "-w" {
        eprintln!("Usage: hash-object -w <file>");
        return Ok(());
    }
    let file_path = &args[3];
    let hash_str = hash_blob(file_path)?;
    println!("{hash_str}");
    Ok(())
}

fn cmd_ls_tree(args: &[String]) -> Result<()> {
    let name_only = args[2] == "--name-only";
    let tree_sha = args.last().context("Missing tree SHA")?;

    let contents = read_object(tree_sha)?;
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

/// Recursively writes the current directory as a tree object and returns the SHA-1 hash of the tree object.
///
/// ```
/// tree <size>\0
/// <mode> <name>\0<20_byte_sha>
/// <mode> <name>\0<20_byte_sha>
/// ```
fn cmd_write_tree(_args: &[String]) -> Result<()> {
    let hash_str = dfs_write_tree(".")?;
    println!("{hash_str}");
    Ok(())
}

///```
/// commit <size>\0tree <tree_sha>
/// parent <parent_sha>
/// author <name> <<email>> <timestamp> <timezone>
/// committer <name> <<email>> <timestamp> <timezone>
///
/// <commit message>
/// ```
fn cmd_commit_tree(args: &[String]) -> Result<()> {
    if args[3] != "-p" || args[5] != "-m" {
        eprintln!("Usage: commit-tree <tree_sha> -p <parent_commit_sha> -m <message>");
        return Ok(());
    }
    let tree_sha = &args[2];
    let commit_sha = &args[4];
    let message = &args[6];
    let data = format!(
        "tree {tree_sha}\nparent {commit_sha}\nauthor Code Crafters <> 0 +0000\ncommitter Code Crafters <> 0 +0000\n\n{message}\n"
    );
    let hash_str = hash_and_save(data.as_bytes(), "commit")?;
    println!("{hash_str}");
    Ok(())
}

struct TreeEntry {
    mode: String,
    entry_type: String,
    sha: String,
    name: String,
}

fn read_object(sha: &str) -> Result<Vec<u8>> {
    let object_path = format!(".git/objects/{}/{}", &sha[0..2], &sha[2..]);
    let mut decoder = ZlibDecoder::new(fs::File::open(object_path)?);
    let mut contents = Vec::new();
    decoder.read_to_end(&mut contents)?;
    Ok(contents)
}

/// Returns a list of (mode, tree/blob, sha, name) entries
fn decode_tree_object(contents: &[u8]) -> Result<Vec<TreeEntry>> {
    let (_, mut contents) = split_at_byte(contents, 0)?;

    let mut result = Vec::new();
    while !contents.is_empty() {
        let (mode, rest) = split_at_byte(contents, b' ')?;
        let (name, rest) = split_at_byte(rest, 0)?;
        let sha = &rest[0..20];
        contents = &rest[20..];

        result.push(get_tree_entry(mode, name, sha)?);
    }
    Ok(result)
}

fn split_at_byte(contents: &[u8], byte: u8) -> Result<(&[u8], &[u8])> {
    let pos = contents
        .iter()
        .position(|&b| b == byte)
        .context("Invalid object format")?;
    let first_part: &[u8] = &contents[0..pos];
    let second_part: &[u8] = &contents[pos + 1..];
    Ok((first_part, second_part))
}

fn get_tree_entry(mode: &[u8], name: &[u8], sha: &[u8]) -> Result<TreeEntry> {
    let mode = std::str::from_utf8(mode)?.to_string();
    let sha = hex::encode(sha);
    let name = std::str::from_utf8(name)?.to_string();
    let entry_type = if mode == "40000" {
        "tree".to_string()
    } else {
        "blob".to_string()
    };
    Ok(TreeEntry {
        mode,
        entry_type,
        sha,
        name,
    })
}

fn dfs_write_tree(path: &str) -> Result<String> {
    // get all entries in the directory
    let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(DirEntry::file_name);
    let mut result = Vec::new();
    for entry in entries {
        if entry.file_name() == ".git" {
            continue;
        }
        let metadata = entry.metadata()?;
        let entry_path = entry.path();
        let entry_path = &entry_path.to_string_lossy();
        let (mode, sha) = if metadata.is_dir() {
            ("40000", dfs_write_tree(entry_path)?)
        } else if metadata.permissions().mode() & 0o111 != 0 {
            ("100755", hash_blob(entry_path)?) // executable
        } else {
            ("100644", hash_blob(entry_path)?) // regular file
        };
        result.extend(format!("{mode} {}\0", entry.file_name().to_string_lossy()).as_bytes());
        result.extend(hex::decode(sha)?);
    }

    let hash_str = hash_and_save(&result, "tree")?;
    Ok(hash_str)
}

fn hash_blob(file_path: &str) -> Result<String> {
    let data = fs::read(file_path)?;
    let hash_str = hash_and_save(&data, "blob")?;
    Ok(hash_str)
}

fn hash_and_save(data: &[u8], object_type: &str) -> Result<String> {
    let header = format!("{object_type} {}\0", data.len());
    let mut hasher = Sha1::new();

    hasher.update(header.as_bytes());
    hasher.update(data);
    let hash = hasher.finalize();
    let hash_str = format!("{hash:x}");

    let object_path = format!(".git/objects/{}/{}", &hash_str[0..2], &hash_str[2..]);
    fs::create_dir_all(format!(".git/objects/{}", &hash_str[0..2]))?;

    let mut encoder = ZlibEncoder::new(fs::File::create(object_path)?, Compression::default());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(data)?;
    encoder.finish()?;
    Ok(hash_str)
}
