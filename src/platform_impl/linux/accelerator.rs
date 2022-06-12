pub fn to_gtk_menemenoic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("&&", "[~~]")
        .replace("&", "_")
        .replace("[~~]", "&&")
}

pub fn to_gtk_accelerator<S: AsRef<str>>(accelerator: S) -> String {
    let accelerator = accelerator.as_ref();
    let mut s = accelerator.split("+");
    let count = s.clone().count();
    let (mod1, mod2, key) = {
        if count == 2 {
            (s.next().unwrap(), None, s.next().unwrap())
        } else if count == 3 {
            (
                s.next().unwrap(),
                Some(s.next().unwrap()),
                s.next().unwrap(),
            )
        } else {
            panic!("Unsupported accelerator format: {}", accelerator)
        }
    };

    let mut gtk_accelerator = parse_mod(mod1).to_string();
    if let Some(mod2) = mod2 {
        gtk_accelerator.push_str(parse_mod(mod2));
    }
    gtk_accelerator.push_str(key);

    gtk_accelerator
}

fn parse_mod(modifier: &str) -> &str {
    match modifier.to_uppercase().as_str() {
        "SHIFT" => "<Shift>",
        "CONTROL" | "CTRL" | "COMMAND" | "COMMANDORCONTROL" | "COMMANDORCTRL" => "<Ctrl>",
        "ALT" => "<Alt>",
        "SUPER" | "META" | "WIN" => "<Meta>",
        _ => panic!("Unsupported modifier: {}", modifier),
    }
}
