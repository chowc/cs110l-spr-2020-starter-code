use grid::Grid; // For lcs()
use std::{env, fs};
use std::fs::File; // For read_file_lines()
use std::io::{self, BufRead}; // For read_file_lines()
use std::process;

pub mod grid;

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

fn lcs(seq1: &Vec<String>, seq2: &Vec<String>) -> Grid {
    // Note: Feel free to use unwrap() in this code, as long as you're basically certain it'll
    // never happen. Conceptually, unwrap() is justified here, because there's not really any error
    // condition you're watching out for (i.e. as long as your code is written correctly, nothing
    // external can go wrong that we would want to handle in higher-level functions). The unwrap()
    // calls act like having asserts in C code, i.e. as guards against programming error.
    let rows = seq1.len()+1;
    let cols = seq2.len()+1;
    let mut grid = Grid::new(rows, cols);
    // for i := 0..m
    // for j := 0..n
    // if X[i] = Y[j]
    // C[i+1,j+1] := C[i,j] + 1
    // else
    // C[i+1,j+1] := max(C[i+1,j], C[i,j+1])

    for (i, s1) in seq1.iter().enumerate() {
        for (j, s2) in seq2.iter().enumerate() {
            if s1 == s2 {
                grid.set(i+1, j+1, grid.get(i, j).unwrap()+1);
            } else {
                let m = std::cmp::max(grid.get(i+1, j).unwrap(), grid.get(i, j+1).unwrap());
                grid.set(i+1, j+1, m);
            }
        }
    }
    grid
    // Be sure to delete the #[allow(unused)] line above
}

// if i > 0 and j > 0 and X[i-1] = Y[j-1]
// print_diff(C, X, Y, i-1, j-1)
// print "  " + X[i-1]
// else if j > 0 and (i = 0 or C[i,j-1] ≥ C[i-1,j])
// print_diff(C, X, Y, i, j-1)
// print "> " + Y[j-1]
// else if i > 0 and (j = 0 or C[i,j-1] < C[i-1,j])
// print_diff(C, X, Y, i-1, j)
// print "< " + X[i-1]
// else
// print ""
fn print_diff(lcs_table: &Grid, lines1: &Vec<String>, lines2: &Vec<String>, i: usize, j: usize) {
    if i > 0 && j > 0 && lines1[i-1] == lines2[j-1] {
        print_diff(lcs_table, lines1, lines2, i-1, j-1);
        println!(" {}", lines1[i-1]);
    } else if j > 0 && (i==0 || lcs_table.get(i, j-1).unwrap() >= lcs_table.get(i-1, j).unwrap()) {
        print_diff(lcs_table, lines1, lines2, i, j-1);
        println!("> {}", lines2[j-1]);
    } else if i > 0 && (j==0 || lcs_table.get(i, j-1).unwrap() < lcs_table.get(i-1, j).unwrap()) {
        print_diff(lcs_table, lines1, lines2, i-1, j);
        println!("< {}", lines1[i-1]);
    }
}

#[allow(unused)] // TODO: delete this line when you implement this function
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename1 = &args[1];
    let filename2 = &args[2];

    let contents1 = read_file_lines(filename1).expect(&*format!("read file {} fail", filename1));
    let contents2 = read_file_lines(filename2).expect(&*format!("read file {} fail", filename2));
    let grid = lcs(&contents1, &contents2);
    print_diff(&grid, &contents1, &contents2, contents1.len(), contents2.len());
    // Be sure to delete the #[allow(unused)] line above
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_file_lines() {
        let lines_result = read_file_lines(&String::from("handout-a.txt"));
        assert!(lines_result.is_ok());
        let lines = lines_result.unwrap();
        assert_eq!(lines.len(), 8);
        assert_eq!(
            lines[0],
            "This week's exercises will continue easing you into Rust and will feature some"
        );
    }

    #[test]
    fn test_lcs() {
        let mut expected = Grid::new(5, 4);
        expected.set(1, 1, 1).unwrap();
        expected.set(1, 2, 1).unwrap();
        expected.set(1, 3, 1).unwrap();
        expected.set(2, 1, 1).unwrap();
        expected.set(2, 2, 1).unwrap();
        expected.set(2, 3, 2).unwrap();
        expected.set(3, 1, 1).unwrap();
        expected.set(3, 2, 1).unwrap();
        expected.set(3, 3, 2).unwrap();
        expected.set(4, 1, 1).unwrap();
        expected.set(4, 2, 2).unwrap();
        expected.set(4, 3, 2).unwrap();

        println!("Expected:");
        expected.display();
        let result = lcs(
            &"abcd".chars().map(|c| c.to_string()).collect(),
            &"adb".chars().map(|c| c.to_string()).collect(),
        );
        println!("Got:");
        result.display();
        assert_eq!(result.size(), expected.size());
        for row in 0..expected.size().0 {
            for col in 0..expected.size().1 {
                assert_eq!(result.get(row, col), expected.get(row, col));
            }
        }
    }
}
