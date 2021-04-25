use handlebars::{self, handlebars_helper};
use std::io::Write;
handlebars_helper!(upper: | s: str | s.to_uppercase());
handlebars_helper!(lower: |s:str| s.to_lowercase());
handlebars_helper!(round: |v: f64| v.round());
handlebars_helper!(round_1: |v: f64| format!("{:.1}", v));
handlebars_helper!(round_2: |v: f64| format!("{:.2}", v));
handlebars_helper!(round_3: |v: f64| format!("{:.3}", v));

/// Fill a vector of text with data using templating.
pub fn fill_data(lines: &[String], data: &serde_json::Value) -> Vec<String> {
    let mut new_lines: Vec<String> = Vec::new();

    let mut reg = handlebars::Handlebars::new();
    reg.register_helper("upper", Box::new(upper));
    reg.register_helper("lower", Box::new(lower));
    reg.register_helper("round", Box::new(round));
    reg.register_helper("round-1", Box::new(round_1));
    reg.register_helper("round-2", Box::new(round_2));
    reg.register_helper("round-3", Box::new(round_3));
    reg.set_strict_mode(true);

    for line in lines {
        match reg.render_template(line, data) {
            Ok(l) => new_lines.push(l),
            Err(e) => {
                let err = format!(
                    "WARNING: {}\n",
                    e.as_render_error()
                        .unwrap()
                        .desc
                        .replace(" in strict mode", "")
                );
                std::io::stderr().write_all(err.as_bytes()).unwrap();
                new_lines.push(line.to_owned())
            }
        };

        /*
        new_lines.push(reg.render_template(line, data).expect("Templating failed"));
            Err(ref e) if e.kind() == handlebars::RenderError => {
                new_lines.push(line.to_owned());
                io::stderr().write_all(e.as_render_error().unwrap().desc.as_bytes());
            }
        */
    }

    new_lines
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_fill_data() {
        let lines: Vec<String> = vec![
            "Hello".into(),
            "I am {{years}} years old.".into(),
            "Goodbye.".into(),
        ];

        let data = serde_json::json!({"years": 24});

        let new_lines = fill_data(&lines, &data);

        assert_eq!(new_lines[1], "I am 24 years old.");
    }

    #[test]
    fn test_read_data() {
        let path = PathBuf::from("tests/data/case2/data.json");

        let data = crate::io::read_data(&path).unwrap();

        assert_eq!(data.get("year").unwrap(), 2000);
        assert_eq!(data.get("year_str").unwrap(), "two thousand");

        let lines: Vec<String> = vec![
            "The year was once {{year}}".into(),
            "This package is called {{package_name}}.".into(),
        ];

        let new_lines = fill_data(&lines, &data);

        assert_eq!(new_lines[0], "The year was once 2000");
        assert_eq!(new_lines[1], "This package is called manus.")
    }
}
