# City Info
City info is a RESTful application that takes handles requests on `/{city_name}` and responds with "useful" data for that `city_name`

The main motivation for this toy program is to introduce the reader to a few key concepts through the lens of a "production" application:
* async rust leveraging [tokio](https://tokio.rs/) and [Futures](https://docs.rs/futures/latest/futures/)
    * Most importantly, the Actor/Handle model described by Alice Rhyl in [this fantastic presentation](https://www.youtube.com/watch?v=fTXuGRP1ee4)
* other common crates leveraged in production rust code, including (but not limited to)
    * [tracing](https://docs.rs/tracing/latest/tracing/)
    * [axum](https://docs.rs/axum/latest/axum/)
    * [anyhow](https://docs.rs/anyhow/latest/anyhow/)
    * [serde](https://docs.rs/serde/latest/serde/s)

## Application Architecture
This simple application is broken into a handful of independent tasks:
* An axum router which serves REST API requests/responses. It sends requests via a `tokio::sync::mpsc` to...
* The dispatcher task, which starts a number of "data fetcher" tasks, fans out requests to them, and aggregates their responses into one response to send back to the axum router
* One or more "data fetcher" tasks which handle requests and respond with interesting data

## Repo architecture
This directory is set up as a [cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html). There is a [bin directory](./city_info/bin) which contains the files required to create a running binary for our program, and a [lib directory](./city_info/lib/) which contains the "business logic" of our application broken into smaller "crates". This is done to facilitate testing (which is definitely overkill for this specific application, but representative of how a production repo might be laid out)

## Exercises
As usual, some work is left for the reader. For those who want to skip ahead, a solution can be found on the `solutions` branch

The reader should:
* ensure all tests pass by addressing any `// TODO` comments

The reader may:
* Make the following implementation more async-friendly by addressing the "exercises left for the reader" in [city_info/bin/main.rs](./city_info/bin/src/main.rs) and/or [city_info/lib/dispatcher/lib.rs](./city_info/lib/dispatcher/src/lib.rs). With these changes implemented the application should easily be able to generate more than enough concurrent requests to be rate-limited by the public APIs it leverages (but please don't do this!)

## Compiling/testing/running
### To compile
Ensure you have cargo and all the various rust compilation tools installed (see [rustup.rs](https://rustup.rs/)). Then, to build the source files, run the following command in this directory:
```sh
cargo build
```

### To test
To run all associated unit and module/crate tests (and build again if you've made changes)
```sh
cargo test
```
If you wish to run tests faster, with a fancier output, see [cargo-nextest](http://nexte.st).

** NOTE: You will see that some unit tests are failing on the master branch, that's the exercise! **

### To run
To run the application simply do
```sh
cargo run
```

You can then make requests to the running server:
```sh
$ curl -k http://127.0.0.1:4242/Chicago

$ curl -k http://127.0.0.1:4242/San%20Jose
```
