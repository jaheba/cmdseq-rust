
use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;

extern crate crypto;

use crypto::digest::Digest;
use crypto::sha2::Sha256;


extern crate getopts;

use getopts::Options;



fn print_usage(error: String, opts: &Options) {
    if !error.is_empty() {
        println!("{}", error);    
    }

    print!("{}", opts.usage(&"Usage: cmdseq [-d <count dir>] <count1> <cmd1> [... <countn> <cmdn>]"));
}

/// build hash from input str and return first 16 chars of it
fn hash(input: &str) -> String {
    let mut sha = Sha256::new();
    sha.input_str(input);

    sha.result_str()[..16].to_string()
}


/// return index of element and index +1 or 0
///
/// cycle(&vec!(3, 2, 1), 0) -> (0, 1)
/// cycle(&vec!(3, 2, 1), 3) -> (1, 4)
/// cycle(&vec!(3, 2, 1), 5) -> (2, 0)
/// 
fn cycle(repetitions: &Vec<usize>, index: usize) -> (usize, usize) {
    let mut sum : usize = 0;

    for (i, &repetition) in repetitions.iter().enumerate() {
        sum += repetition;

        // cycle check
        // twice we compare index to len (that's why i+1).
        // check for last iteration && last command in that iteration
        if i+1 == repetitions.len() && index+1 == sum {
            return (i, 0);
        }

        if index < sum {
            return (i, index+1);
        }
    }

    // todo: decide what happens on invalid input
    // i.e. sum(repetitions) < index + 1
    panic!("out of range index {}", index);
}


/// execute command without piping stdout or stderr
fn execute_command(command: &str) {
    Command::new("sh")
        .arg("-c").arg(command)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
    });
}

fn parse_commands(args: &Vec<String>) -> Result<(Vec<usize>, Vec<String>), String> {

    //check len of args is even
    if args.len() == 0 {
        return Err("Error: missing commands".to_string());
    }
    if args.len() %2 != 0 {
        return Err("pairs of commands are needed".to_string());
    }

    let mut repetitions: Vec<usize> = vec!();
    let mut commands: Vec<String> = vec!();

    match args.into_iter() {
        mut iter => loop {
            match iter.next() {
                Some(n_str) => {
                    let n = match n_str.parse::<usize>(){
                        Ok(v) => v,
                        Err(e) => return Err(format!("could not prase {} into number", e))
                    };
                    let cmd = iter.next().unwrap();

                    repetitions.push(n);
                    commands.push(cmd.clone());
                },
                None => break,
            }
        }
    };

    Ok((repetitions, commands))

}

struct CLIOption {
    directory: String,
    repetitions: Vec<usize>,
    commands: Vec<String>
}

fn build_options() -> Options {
    let mut opts = Options::new();

    opts.optopt("d", "", "countdir", "dir");
    opts.optflag("h", "help", "print this help menu");   

    opts
}

fn parse_options(opts: &Options) -> Result<CLIOption, String>{
    let args: Vec<String> = std::env::args().collect();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => {
            return Err(f.to_string());
        }
    };

    if matches.opt_present("h") {
        // print_usage(&program, opts);
        return Err("".to_string());
    }

    let dir = match matches.opt_str("d") {
        Some(v) => v,
        None => "/tmp/".to_string()
    };


    let (rep, cmds) = match parse_commands(&matches.free) {
        Ok((rep, cmds)) => (rep, cmds),
        Err(e) => return Err(e.to_string())
    };
    
    Ok(CLIOption {
        directory: dir,
        repetitions: rep,
        commands: cmds
    })
}

fn read_cookie(path: &Path) -> usize{
    // if file exists, read it
    if path.is_file() {
        let mut f = File::open(&path).unwrap_or_else(|_|
            { panic!("unable to open file {}", path.display()) }
        );
        let mut buffer = String::new();
        f.read_to_string(&mut buffer).unwrap();
        return buffer.trim().parse::<usize>()
            .ok()
            .expect("could not parse number from cache file");
    }

    let mut f = File::create(&path).unwrap_or_else(|_| {
        panic!("unable to create new cookie file {}", path.display())
    });
    
    f.write_all(b"0").unwrap();

    0
}

fn write_cookie(path: &Path, value: usize) {
    let mut f = OpenOptions::new().
        write(true)
        .open(&path).unwrap();
    
    f.write_all(&value.to_string().as_bytes()).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // build hash from argv
    let hash_value = hash(&args[1..].join("-"));
    
    let opts = build_options();
    // get cli options
    let options = match parse_options(&opts) {
        Ok(opts) => opts,
        Err(e) => {
            print_usage(e, &opts);
            std::process::exit(1);
        }
    };

    // path to cookie file
    let path_buf = Path::new(&options.directory).join("cmdseq.".to_string() + &hash_value);
    let path = path_buf.as_path();

    // read cookie
    let current_cmd_index = read_cookie(&path);

    // get command to execute
    let (cmd_vector_index, next_cmd_index) = cycle(&options.repetitions, current_cmd_index);

    execute_command(&options.commands[cmd_vector_index]);

    write_cookie(&path, next_cmd_index);
}