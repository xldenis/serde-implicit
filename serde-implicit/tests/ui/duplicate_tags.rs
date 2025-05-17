#[derive(serde_implicit_proc::Deserialize)]
enum MultiTagFields {
    DoubleTagged {
        #[tag]
        primary_tag: String,
        #[tag]
        secondary_tag: bool,
        value: u32,
    },
    SingleTagged {
        #[tag]
        only_tag: u32,
        value: String,
    },
}

fn main() {}
