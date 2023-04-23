use hadron::app::App;

fn main() {
    enable_backtrace();

    println!("Hadron!");

    let app = App::new();
}

fn enable_backtrace() {
    std::env::set_var("RUST_BACKTRACE", "1");
}
