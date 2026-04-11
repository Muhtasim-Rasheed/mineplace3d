use std::{collections::HashMap, sync::LazyLock};

static EMOJI_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(":glungus:", "🐱");
    m.insert(":st:", "\u{0336}");
    m
});

pub fn replace_emojis(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        if let Some((emoji, replacement)) =
            EMOJI_MAP.iter().find(|(e, _)| input[i..].starts_with(*e))
        {
            out.push_str(replacement);
            i += emoji.len();
            continue;
        }

        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}
