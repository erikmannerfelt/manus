use handlebars::{self,handlebars_helper};
use serde_json::Value as Json;
use std::io::Write;

handlebars_helper!(upper: | s: str | s.to_uppercase());
handlebars_helper!(lower: |s:str| s.to_lowercase());


/// Helper to work with error values.
///
/// Given the data:
/// ```
/// {
///     "value": 1.23,
///     "value_pm": 0.45
/// }
/// ```
/// the helper "{{pm value}}" will render as "`1.23$\pm$0.45`".
///
/// If two arguments are given, the first is parsed as the amount of decimals to round:
///
/// "{{pm 1 value}}" => "`1.2$\pm$0.5`"
///
fn pm_helper(h: &handlebars::Helper, _: &handlebars::Handlebars, context: &handlebars::Context, _: &mut handlebars::RenderContext, out: &mut dyn handlebars::Output) -> handlebars::HelperResult {

    // Check that only two arguments were provided.
    if h.param(2).is_some() {
        return Err(handlebars::RenderError::new::<String>("pm only takes two arguments. More were given.".into()));
    };

    // Check if two arguments were given (if more than one, this is assumed to be true).
    let two_arguments = h.param(1).is_some();

    // If two arguments are given, the value key is the second index, else the first.
    let key_index: usize = match two_arguments {
        true => 1,
        false => 0
    };

    // Try to find the key to the value.
    let keys = match h.param(key_index) {
        // If the attribute exists, try to see if it is a path.
        Some(attr) => match attr.context_path() {
            // If a data path was associated, return it.
            Some(v) => v,
            // Otherwise, raise an error.
            None => return Err(handlebars::RenderError::new::<String>(format!("pm argument: {} was not a valid data path.", attr.value().to_string())))
        },
        // It only reaches here if no argument was given.
        None => return Err(handlebars::RenderError::new::<String>("No argument was given for pm".into()))
    };

    // The last key is the value key.
    let value_key = keys[(keys.len() - 1)].to_owned();
    // The first keys are the parent keys (may be empty, but that's fine).
    let parent_keys = &keys[..(keys.len() - 1)];

    // Find the parent json value by iteratively running .get methods.
    let mut parent: &Json = context.data();
    // Loop through each parent key (if any).
    for key in parent_keys {
        parent = parent.get(key).expect("Getter failed on parent json. Shouldn't happen.");
    }

    // Parse the value and plusminus keys as f64.
    let mut value = match parent.get(&value_key).expect("Value not found in parent. Something is wrong.").as_f64() {
        Some(v) => v,
        None => return Err(handlebars::RenderError::new::<String>(format!("Could not parse value {} as float", parent.get(&value_key).unwrap())))
    };
    // The plusminus key might not exist, so this has to be checked.
    let mut pm = match parent.get(&(value_key.to_owned() + "_pm")) {
        Some(v) => {
            match v.as_f64() {
                Some(y) => y,
                None => return Err(handlebars::RenderError::new::<String>(format!("Could not parse pm value {} as float", v.to_string())))
            }
        },
        None => return Err(handlebars::RenderError::new::<String>(format!("{}_pm key not found", value_key)))
    };


    // If two arguments were given, the decimals variable should be used.
    if two_arguments {
        // Read param 0 as the decimal
        let decimals = match h.param(0) {
            // If param 0 exists:
            Some(p) => {
                // Try to parse the first parameter as an integer.
                match json_as_integer(p.value()) {
                    Ok(x) => x,
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e))
                }
            },
            // If it doesn't exist (which should be impossible since param 1 exists.)
            None => return Err(handlebars::RenderError::new::<String>("Could not find the first argument.".into()))
        };

        // Update the value and pm variables with the rounded value.
        value = round_value(value, decimals);
        pm = round_value(pm, decimals);
    }

    // Write the latex notation for value plusminus error.
    out.write(&format!("{}$\\pm${}", value, pm))?;

    Ok(())

}


/// Helper to round a value up or down.
///
/// If one argument is given, it will round this to the nearest integer.
/// If two are given, the first is parsed as the `decimals` and the second as `value`.
fn round_helper(h: &handlebars::Helper, _: &handlebars::Handlebars, _: &handlebars::Context, _: &mut handlebars::RenderContext, out: &mut dyn handlebars::Output) -> handlebars::HelperResult {


    // Establish the decimals and value arguments which will soon be assigned.
    let decimals: i32;
    let value: f64;

    // If the helper has an argument of index 1, it is assumed to have two arguments.
    let two_arguments = h.param(1).is_some();
    if two_arguments {
        // Read param 0 as the decimal
        decimals = match h.param(0) {
            // If param 0 exists:
            Some(p) => {
                // Try to parse the first parameter as an integer.
                match json_as_integer(p.value()) {
                    Ok(x) => x,
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e))
                }
            },
            // If it doesn't exist (which should be impossible since param 1 exists.)
            None => return Err(handlebars::RenderError::new::<String>("Could not find the first argument.".into()))
        };

        // Read param 1 as the value (we already know that 1 exists, so just unwrap it). 
        value = match json_as_float(h.param(1).unwrap().value()) {
                    Ok(x) => x,
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e))
        }

    // If only one argument was specified, default to a decimal of 0
    } else {

        decimals = 0;

        // Read param 0 as the value
        value = match h.param(0) {
            // If param 0 exists:
            Some(p) => {
                // Try to parse the first parameter as a float.
                match json_as_float(p.value()) {
                    Ok(x) => x,
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e))
                }
            },
            // If it doesn't exist.
            None => return Err(handlebars::RenderError::new::<String>("Could not read the first argument.".into()))
        };

    }

    out.write(&format!("{}", round_value(value, decimals)))?;

    Ok(())


}

/// Helper to round a value upwards.
///
/// Requires two arguments: 'power' (the power of ten to consider) and 'value' (the value to round)
fn roundup_helper(h: &handlebars::Helper, _: &handlebars::Handlebars, _: &handlebars::Context, _: &mut handlebars::RenderContext, out: &mut dyn handlebars::Output) -> handlebars::HelperResult {

    let decimals = match h.param(0) {
        Some(p) => {
            match json_as_integer(p.value()) {
                Ok(x) => x,
                Err(e) => return Err(handlebars::RenderError::new::<String>(e))
            }
        },
        None => return Err(handlebars::RenderError::new::<String>("No arguments provided.".into()))
    };

    let value = match h.param(1) {
        Some(p) => {
            match json_as_float(p.value()) {
                Ok(x) => x,
                Err(e) => return Err(handlebars::RenderError::new::<String>(e))
            }
        },
        None => return Err(handlebars::RenderError::new::<String>("Only one argument provided. Requires: 'power' 'value'".into()))
    };

    out.write(&format!("{}", round_value(value, -decimals)))?;

    Ok(())
}

/// Try to parse a JSON value as i32.
///
///
/// # Examples
/// ```
/// let v = json!["2"];
///
/// assert_eq!(json_as_integer(v), 2);
/// ```
fn json_as_integer(value: &Json) -> Result<i32, String> {
    let parsed: Option<i32> = match value {
        Json::Number(n) => {
            match n.as_i64() {
                Some(x) => Some(x as i32),
                None => None
            }
        },
        Json::String(s) => match s.to_string().parse::<i32>() {
            Ok(x) => Some(x),
            Err(_) => None
        }
        _ => None
    };
    match parsed {
        Some(n) => Ok(n),
        None => Err(format!("Could not parse {} as an integer.", value))
    }
}

/// Try to parse a JSON value as f64.
///
/// # Examples
/// ```
/// let v = json!["2.2"];
/// 
/// assert_eq!(json_as_float(v), 2.2);
/// ```
fn json_as_float(value: &Json) -> Result<f64, String> {
    let parsed: Option<f64> = match value {
        Json::Number(n) => n.as_f64(),
        Json::String(s) => match s.to_string().parse::<f64>() {
            Ok(x) => Some(x),
            Err(_) => None
        }
        _ => None
    };
    match parsed {
        Some(n) => Ok(n),
        None => Err(format!("Could not parse {} as a floating point value.", value))
    }
}

/// Round a value to the nearest decimal.
///
/// Uses the f64::round() method on decimal-shifted values.
///
/// If a negative decimal number is given, rounding is done upwards.
///
/// # Arguments
/// * `value`: The value to round.
/// * `decimals`: The number of decimals to round to (can be negative).
///
/// # Returns
/// * `decimal > 0`: The value rounded to the nearest N decimal.
/// * `decimal == 0`: The value rounded to the nearest integer.
/// * `decimal < 0`: The value rounded to the nearest -N factor of ten.
///
/// # Examples
/// ```
/// assert_eq!(round_value(1.234, 1), 1.2);
/// ```
///
/// ```
/// assert_eq!(round_value(8999.0, -3), 9000.0);
/// ```
fn round_value(value: f64, decimals: i32) -> f64 {
    (value * 10_f64.powi(decimals)).round()  / 10_f64.powi(decimals)
}

/// Fill a vector of text with data using templating.
pub fn fill_data(lines: &[String], data: &serde_json::Value) -> Vec<String> {
    let mut new_lines: Vec<String> = Vec::new();

    let mut reg = handlebars::Handlebars::new();
    reg.register_helper("upper", Box::new(upper));
    reg.register_helper("lower", Box::new(lower));
    reg.register_helper("round", Box::new(round_helper));
    reg.register_helper("roundup", Box::new(roundup_helper));
    reg.register_helper("pm", Box::new(pm_helper));
    reg.set_strict_mode(true);

    for (i, line) in lines.iter().enumerate() {
        match reg.render_template(line, data) {
            Ok(l) => new_lines.push(l),
            Err(e) => {
                let re = e.as_render_error();

                let col = match re {
                    Some(re2) => match re2.column_no {
                        Some(no) => no,
                        None => 0_usize
                    },
                    None => 0_usize,
                };

                let desc = match re {
                    Some(re2) => re2.desc.replace(" in strict mode", ""),
                    None => "Unknown failure".into()
                };


                let err = format!(
                    "WARNING L{}C{}: {}\n",
                    i + 1,
                    col,
                    desc
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

    #[test]
    fn test_round_helpers() {
        let lines: Vec<String> = vec![
            "Hello".into(),
            "{{large_value}} rounded to the nearest 1000 is {{roundup 3 large_value}}".into(),
            "{{decimal_value}} rounded to one decimal is {{round 1 decimal_value}}".into()
        ];

        // Try the large value as an integer and decimal_value as a string.
        let data = serde_json::json!({"large_value": 8699, "decimal_value": "1.234"});

        let new_lines = fill_data(&lines, &data);

        assert_eq!(round_value(1.234, 1), 1.2);
        assert_eq!(round_value(8699_f64, -3), 9000.0);

        assert_eq!(new_lines[0], "Hello");
        assert_eq!(new_lines[1], "8699 rounded to the nearest 1000 is 9000");
        assert_eq!(new_lines[2], "1.234 rounded to one decimal is 1.2");
    }

    #[test]
    fn test_pm_helper() {

        let lines: Vec<String> = vec![
            "The value is {{pm data.value}}".into(),
            "The value is {{pm 1 data.value}}".into(),
            "The other value is {{pm value2}}".into()
        ];

        let data = serde_json::json!({"data": {"value": 1.2345, "value_pm": 0.2345}, "value2": 2, "value2_pm": 0.1});

        let new_lines = fill_data(&lines, &data);


        assert_eq!(new_lines[0], "The value is 1.2345$\\pm$0.2345");
        assert_eq!(new_lines[1], "The value is 1.2$\\pm$0.2");
        assert_eq!(new_lines[2], "The other value is 2$\\pm$0.1");

    }
}
