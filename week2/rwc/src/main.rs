use std::{env, io};
use std::fs::File;
use std::io::BufRead;
use std::process;
//  given an input file, output the number of words, lines, and characters in the file
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    let lines = read_file_lines(filename).expect(&*format!("read from file {} fail", filename));
    println!("words: {}, lines: {}, characters: {}", count_words_in_lines(&lines), lines.len(), count_characters_in_lines(&lines));
}

/// Reads the file at the supplied path, and returns a vector of strings.
fn read_file_lines(filename: &String) -> Result<Vec<String>, io::Error> {
    let file = File::open(filename)?;
    let mut v = Vec::<String>::new();
    for line in io::BufReader::new(file).lines() {
        let line_str = line?;
        v.push(line_str);
    };
    Ok(v)
}

fn count_words_in_lines(lines: &Vec<String>) -> usize {
    let mut count = 0;
    for line in lines {
        let one = count_words_in_line(&line);
        count += one;
    }
    count
}

fn count_words_in_line(line: &String) -> usize {
    let words: Vec<&str> = line.split(" ").collect();
    let mut word_count = 0;
    for w in words.iter() {
        if w == &" " {

        } else {
            word_count += 1;
        }
    }
    word_count
}

fn count_characters_in_lines(lines: &Vec<String>) -> usize {
    let mut count = 0;
    for line in lines {
        count += line.len();
    }
    count
}