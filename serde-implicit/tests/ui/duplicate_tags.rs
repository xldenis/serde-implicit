#[derive(serde_implicit_proc::Deserialize)]
enum MultiTagFields {
    DoubleTagged {
        #[serde_implicit(tag)]
        primary_tag: String,
        #[serde_implicit(tag)]
        secondary_tag: bool,
        value: u32,
    },
    SingleTagged {
        #[serde_implicit(tag)]
        only_tag: u32,
        value: String,
    },
}

fn main() {}
