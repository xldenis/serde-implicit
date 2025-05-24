#[derive(serde_implicit_proc::Deserialize)]
enum RepeatedTag {
    Var1 {
        #[serde_implicit(tag)]
        primary_tag: String,
        value: u32,
    },
    Var2 {
        #[serde_implicit(tag)]
        primary_tag: String,
        value: String,
    },
}

fn main() {}
