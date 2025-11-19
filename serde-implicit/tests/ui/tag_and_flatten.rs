#[derive(serde::Deserialize)]
struct Helper(String, bool);

#[derive(serde_implicit_proc::Deserialize)]
enum BadAnnotation {
    Normal(bool),
    Conflicting(#[serde_implicit(tag, flatten)] Helper),
}

fn main() {}
