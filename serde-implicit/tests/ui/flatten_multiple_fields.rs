#[derive(serde::Deserialize)]
struct Helper(String, bool);

#[derive(serde_implicit_proc::Deserialize)]
enum BadFlatten {
    Normal(bool),
    MultiField(u64, #[serde_implicit(flatten)] Helper),
}

fn main() {}
