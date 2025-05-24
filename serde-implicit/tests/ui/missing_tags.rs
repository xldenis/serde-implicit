#[derive(serde_implicit_proc::Deserialize)]
enum OopsTag {
    MissingTag {
        field1: String,
        field2: bool,
        value: u32,
    },
    SingleTagged {
        #[serde_implicit(tag)]
        only_tag: u32,
        value: String,
    },
}

fn main() {}
