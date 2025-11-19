#[derive(serde::Deserialize)]
struct Helper(String, bool);

#[derive(serde_implicit_proc::Deserialize)]
enum BadOrder {
    Flatten(#[serde_implicit(flatten)] Helper),
    Normal(bool),
}

fn main() {}
