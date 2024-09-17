# Welcome to Telephone Game!
This is a very simple toy program to get folks used to navigating a rust repo and editing source files

## To compile
Ensure you have cargo and all the various rust compilation tools installed (see [rustup.rs](https://rustup.rs/). Then, to build the source files, run the following in this directory:
```sh
cargo build
```

## To test
Once you have a successful build you can run
```sh
cargo test
```
to run all associated unit and module/crate tests in a file. If you wish to run tests faster, with a fancier output, see [cargo-nextest](http://nexte.st). 
You can also skip the build step above, as running tests will of course compile the project first if you have made any changes. 

** NOTE: You will see currently that one of the unit tests is failing, fixing that test is the exercise! **

## To run
To run the program simply do 
```sh
cargo run
```

