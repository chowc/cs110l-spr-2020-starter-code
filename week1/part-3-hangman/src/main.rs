// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;
use std::process::exit;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    let mut find_index = vec![0; secret_word_chars.len()];
    let mut mask_chars = vec!["-".to_string(); secret_word_chars.len()];
    let mut guessed_chars = vec![];
    // Uncomment for debugging:
    println!("random word: {}", secret_word);
    println!("Welcome to CS110L Hangman!");
    let mut count = 0;
    let mut find_count = 0;
    loop {
        if count >= NUM_INCORRECT_GUESSES {
            println!("Sorry, you ran out of guesses!");
            exit(0);
        }
        println!("The word so far is  {}", mask_chars.join(""));
        println!("You have guessed the following letters: {}", guessed_chars.join(""));
        println!("You have {} guesses left", NUM_INCORRECT_GUESSES-count);
        print!("Please guess a letter: ");
        io::stdout()
            .flush()
            .expect("Error flushing stdout.");
        let mut guess = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");
        guess = guess.replace('\n', "");
        let char = guess.chars().next().expect("read input line fail");
        let mut i = 0;
        let mut find = false;
        while i < secret_word_chars.len() {
            if find_index[i] == 0 && secret_word_chars[i] == char {
                find_index[i] = 1;
                mask_chars[i] = secret_word_chars[i].to_string();
                find = true;
                find_count += 1;
                break;
            }
            i += 1;
        }
        // 排除已经猜中的 char
        guessed_chars.push(guess);
        if !find {
            count += 1;
            println!("Sorry, that letter is not in the word");
        } else if find_count == secret_word_chars.len() {
            println!("Congratulations you guessed the secret word: {}!", secret_word);
            exit(0);
        }
        println!();
    }
}
