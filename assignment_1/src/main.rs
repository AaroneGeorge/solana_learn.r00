// Objective
// Build the Guessing Game project from the Rust Book.
// 
// What to do
// Follow the Rust Book and implement the Guessing Game in Rust.
// Make sure your project compiles and runs correctly.
// 
// Submission
// Create a GitHub repository with your project.
// Attach the link to your GitHub repository as your submission.

use std::io;
use std::cmp::Ordering;
use rand::RngExt;

fn main() {
    println!("Guess the number between 1 and 10!\n");

    let secret_number = rand::rng().random_range(1..=10);

    loop { 

        println!("Enter your guess: ");
        let mut guess = String::new();

        io::stdin()
            .read_line(&mut guess)
            .expect("Failed to read line");

        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };


        match guess.cmp(&secret_number) {
            Ordering::Less => println!("Too small!"),
            Ordering::Greater => println!("Too big!"),
            Ordering::Equal => {
                println!("You win!\n your guess \"{guess}\" is correct!");
                break;
            }
        }

    }
}

// Output
// └─$ cargo run  
//     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
//      Running `target/debug/assignment_1`
// Guess the number between 1 and 10!

// Enter your guess: 
// 4
// Too big!
// Enter your guess: 
// 5
// Too big!
// Enter your guess: 
// 2
// Too big!
// Enter your guess: 
// 1
// You win!
//  your guess "1" is correct!