use handlebars::{self, handlebars_helper};
use serde_json::Value as Json;
use std::io::Write;

handlebars_helper!(upper: | s: str | s.to_uppercase());
handlebars_helper!(lower: |s:str| s.to_lowercase());

/// Helper to make large numbers more readable using a 1000s separator.
///
/// Given the data:
/// ```
/// {
///     "separator": ",",
///     "large_value": 123456789
/// }
/// ```
/// the helper "{{sep value}}" will render: "`123,456,789`".
///
/// Note that the "separator" key needs to exist in the data file.
///
fn sep_helper(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    context: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    // Check that only argument was provided.
    if h.param(1).is_some() {
        return Err(handlebars::RenderError::new::<String>(
            "pm only takes two arguments. More were given.".into(),
        ));
    };

    let data = context.data();

    let separator = match data.get("separator") {
        Some(v) => v.as_str().unwrap(),
        None => {
            return Err(handlebars::RenderError::new::<String>(
                "Could not find the \"separator\" key in the data file. Please add it.".into(),
            ))
        }
    };

    let value = match h.param(0) {
        Some(p) => match p.value().as_str() {
            Some(s) => s.to_owned(),
            None => p.value().to_string(),
        },
        None => {
            return Err(handlebars::RenderError::new::<String>(
                "Could not read the second argument..".into(),
            ))
        }
    };

    let mut new_value = String::new();

    let mut number_buffer = String::new();
    let mut in_digit = false;
    let mut n_periods = 0;
    for c in value.chars() {
        if c == '.' {
            n_periods += 1;
        } else {
            n_periods = 0;
        };
        in_digit = c.is_ascii_digit() | (in_digit & (n_periods == 1));

        if in_digit {
            number_buffer.push(c);
        } else {
            if !number_buffer.is_empty() {
                let number = number_buffer.parse::<f64>().unwrap();
                new_value += &add_separators(number, separator);
                number_buffer.clear();
            }
            new_value.push(c);
        };
    }
    if !number_buffer.is_empty() {
        let number = number_buffer.parse::<f64>().unwrap();
        new_value += &add_separators(number, separator);
    }

    out.write(&new_value)?;

    Ok(())
}

/// Add 1000s separators for a number
///
/// # Arguments
/// * `number`: The number to make more readable
/// * `separator`: The string to separate 1000s with
///
/// # Examples
/// ```
/// assert_eq!(add_separators(12345.678, ","), "12,345.678")
/// ```
///
/// # Returns
/// A more readable string representation of the number.
fn add_separators(number: f64, separator: &str) -> String {
    // Convert the number into a string.
    let number_str = format!("{}", number);

    let separator_backwards = separator.chars().rev().collect::<String>();

    // Convert the real part of the number into a string.
    let real_part_str = format!("{}", number.trunc() as i64);

    // Create an empty string of the new more readable real part of the number
    // It will be filled in backwards, so it will be reversed.
    let mut new_real_str_rev = String::new();
    // Loop over the real part in reverse.
    for (i, c) in real_part_str
        .chars()
        .collect::<Vec<char>>()
        .iter()
        .rev()
        .enumerate()
    {
        // If the numbering is divisible by 3, add a separator.
        if (i % 3 == 0) & (i > 0) {
            new_real_str_rev.push_str(&separator_backwards);
        };
        // Push the digit as owned.
        new_real_str_rev.push(*c);
    }
    // Reorder the characters to be in the right direction.
    let new_real_str: String = new_real_str_rev.chars().rev().collect::<String>();

    // Replace the real part of the number with the new more readable version.
    number_str.replace(&real_part_str, &new_real_str)
}

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
fn pm_helper(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    context: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    // Check that only two arguments were provided.
    if h.param(2).is_some() {
        return Err(handlebars::RenderError::new::<String>(
            "pm only takes two arguments. More were given.".into(),
        ));
    };

    // Check if two arguments were given (if more than one, this is assumed to be true).
    let two_arguments = h.param(1).is_some();

    // If two arguments are given, the value key is the second index, else the first.
    let key_index: usize = match two_arguments {
        true => 1,
        false => 0,
    };

    // Try to find the key to the value.
    let keys = match h.param(key_index) {
        // If the attribute exists, try to see if it is a path.
        Some(attr) => match attr.context_path() {
            // If a data path was associated, return it.
            Some(v) => v,
            // Otherwise, raise an error.
            None => {
                let e = match attr.relative_path() {
                    Some(rp) => format!("pm got invalid data path: {:?}", rp),
                    None => match attr.value() {
                        Json::Null => "No argument was found.".to_string(),
                        v => format!("pm argument: {} is not a valid data path.", v.to_string()),
                    },
                };
                return Err(handlebars::RenderError::new::<String>(e));
            }
        },
        // It only reaches here if no argument was given.
        None => {
            return Err(handlebars::RenderError::new::<String>(
                "No argument was given for pm".into(),
            ))
        }
    };

    // The last key is the value key.
    let value_key = keys[(keys.len() - 1)].to_owned();
    // The first keys are the parent keys (may be empty, but that's fine).
    let parent_keys = &keys[..(keys.len() - 1)];

    // Find the parent json value by iteratively running .get methods.
    let mut parent: &Json = context.data();
    // Loop through each parent key (if any).
    for key in parent_keys {
        parent = parent
            .get(key)
            .expect("Getter failed on parent json. Shouldn't happen.");
    }

    // Parse the value and plusminus keys as f64.
    let mut value = match parent
        .get(&value_key)
        .expect("Value not found in parent. Something is wrong.")
        .as_f64()
    {
        Some(v) => v,
        None => {
            return Err(handlebars::RenderError::new::<String>(format!(
                "Could not parse value {} as float",
                parent.get(&value_key).unwrap()
            )))
        }
    };
    // The plusminus key might not exist, so this has to be checked.
    let mut pm = match parent.get(&(value_key.to_owned() + "_pm")) {
        Some(v) => match v.as_f64() {
            Some(y) => y,
            None => {
                return Err(handlebars::RenderError::new::<String>(format!(
                    "Could not parse pm value {} as float",
                    v.to_string()
                )))
            }
        },
        None => {
            return Err(handlebars::RenderError::new::<String>(format!(
                "{}_pm key not found",
                value_key
            )))
        }
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
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
                }
            }
            // If it doesn't exist (which should be impossible since param 1 exists.)
            None => {
                return Err(handlebars::RenderError::new::<String>(
                    "Could not find the first argument.".into(),
                ))
            }
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
fn round_helper(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
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
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
                }
            }
            // If it doesn't exist (which should be impossible since param 1 exists.)
            None => {
                return Err(handlebars::RenderError::new::<String>(
                    "Could not find the first argument.".into(),
                ))
            }
        };

        // Read param 1 as the value (we already know that 1 exists, so just unwrap it).
        value = match json_as_float(h.param(1).unwrap().value()) {
            Ok(x) => x,
            Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
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
                    Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
                }
            }
            // If it doesn't exist.
            None => {
                return Err(handlebars::RenderError::new::<String>(
                    "Could not read the first argument.".into(),
                ))
            }
        };
    }

    out.write(&format!("{}", round_value(value, decimals)))?;

    Ok(())
}

/// Helper to round a value upwards.
///
/// Requires two arguments: 'power' (the power of ten to consider) and 'value' (the value to round)
fn roundup_helper(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let decimals = match h.param(0) {
        Some(p) => match json_as_integer(p.value()) {
            Ok(x) => x,
            Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
        },
        None => {
            return Err(handlebars::RenderError::new::<String>(
                "No arguments provided.".into(),
            ))
        }
    };

    let value = match h.param(1) {
        Some(p) => match json_as_float(p.value()) {
            Ok(x) => x,
            Err(e) => return Err(handlebars::RenderError::new::<String>(e)),
        },
        None => {
            return Err(handlebars::RenderError::new::<String>(
                "Only one argument provided. Requires: 'power' 'value'".into(),
            ))
        }
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
        Json::Number(n) => match n.as_i64() {
            Some(x) => Some(x as i32),
            None => None,
        },
        Json::String(s) => match s.to_string().parse::<i32>() {
            Ok(x) => Some(x),
            Err(_) => None,
        },
        _ => None,
    };
    match parsed {
        Some(n) => Ok(n),
        None => Err(format!("Could not parse {} as an integer.", value)),
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
            Err(_) => None,
        },
        _ => None,
    };
    match parsed {
        Some(n) => Ok(n),
        None => Err(format!(
            "Could not parse {} as a floating point value.",
            value
        )),
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
    (value * 10_f64.powi(decimals)).round() / 10_f64.powi(decimals)
}

/// Fill a vector of text with data using templating.
pub fn fill_data(lines: &[String], data: &serde_json::Value) -> Result<Vec<String>, String> {
    let parsed_data = evaluate_all_expressions(data)?;

    let mut new_lines: Vec<String> = Vec::new();

    let mut reg = handlebars::Handlebars::new();
    reg.register_helper("upper", Box::new(upper));
    reg.register_helper("lower", Box::new(lower));
    reg.register_helper("round", Box::new(round_helper));
    reg.register_helper("roundup", Box::new(roundup_helper));
    reg.register_helper("pm", Box::new(pm_helper));
    reg.register_helper("sep", Box::new(sep_helper));
    reg.set_strict_mode(true);

    for (i, line) in lines.iter().enumerate() {
        match reg.render_template(line, &parsed_data) {
            Ok(l) => new_lines.push(l),
            Err(e) => {
                let re = e.as_render_error();

                let col = match re {
                    Some(re2) => re2.column_no.unwrap_or(0_usize),
                    None => 0_usize,
                };

                let desc = match re {
                    Some(re2) => re2.desc.replace(" in strict mode", ""),
                    None => "Template render error.".into(),
                };

                let err = format!("WARNING L{}C{}: {}\n", i + 1, col, desc);
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

    Ok(new_lines)
}

/// Recursively find all expressions (strings starting with "expr:") in a json object.
///
/// # Arguments
/// * `data`: The json to find expressions in.
/// * `parent`: Parent keys to append to the output (only matters internally for recursion)
///
/// # Returns
/// A vector of expressions, where each expression is (vector of keys to find it, expression).
/// If no expressions are found, this will be empty.
fn find_expressions(data: &Json, parent: Option<&Vec<String>>) -> Vec<(Vec<String>, String)> {
    // The parent relative to the current tree is empty if parent was None or the given parent.
    let relative_parent: Vec<String> = match parent {
        Some(p) => p.to_owned(),
        None => Vec::new(),
    };

    // Create an empty output variable.
    let mut output: Vec<(Vec<String>, String)> = Vec::new();

    // If the json is an array, parse all expressions in the array.
    if let Json::Array(arr) = data {
        // Loop through the array
        for val in arr {
            // Recursively find all expressions in the json value.
            // The parent argument helps retaining the right tree structure.
            let expressions = find_expressions(val, Some(&relative_parent));

            // Push all found expressions into the output.
            for expression in expressions {
                output.push(expression);
            }
        }
    };
    // If the json is an object (mental note: equivalent to a python dictionary)
    if let Json::Object(obj) = data {
        // Loop through all key-value pairs.
        for (key, val) in obj {
            // The relative parent of this pair is the upper relative parent plus the key.
            let mut relative_parent2 = relative_parent.to_owned();
            relative_parent2.push(key.to_owned());

            // Find all expressions in that value.
            let expressions = find_expressions(val, Some(&relative_parent2));

            // Push all expressions to the output.
            for expression in expressions {
                output.push(expression);
            }
        }
    };
    // If the json is a string and it countains "expr:", push it to the output.
    if let Json::String(string) = data {
        if string.trim().starts_with("expr:") {
            output.push((relative_parent, string.to_owned()));
        }
    };
    // If the json is any other type of value, it will be skipped.

    output
}

/// Evaluate a mathematical expression return a useful error if it fails.
///
/// It is basically just calling the "eval" crate, but handles error messages better than the
/// crate does per default.
///
/// # Arguments
/// * `expr_string`: The expression to evaluate.
/// * `data`: The data "context" to get variables from.
///
/// # Returns
/// The result of the evaluated expression, or an error detailing why it failed.
fn run_eval(expr_string: &str, data: &Json) -> Result<Json, String> {
    // Create an expression object from the string.
    let mut expr = eval::Expr::new(expr_string);

    // All this is to implement the round function. Oboy!
    expr = expr.function("round", |args: Vec<Json>| {
        // Parse the first argument as the value to round.
        let value = match args.get(0) {
            Some(Json::Number(x)) => x.as_f64().unwrap(),
            _ => return Err(eval::Error::ExpectedNumber),
        };
        // Parse the second argument as the decimal.
        let decimals = match args.get(1) {
            // If it's a number, parse it as f64
            Some(Json::Number(x)) => x.as_f64().unwrap(),
            // If it's anything else than a number, return an error
            Some(_) => return Err(eval::Error::ExpectedNumber),
            // If the argument was not given, default to 0
            None => 0.0,
        };

        // Return an error if the decimal number is not equivalent to an integer.
        if decimals.fract() > 0.0 {
            return Err(eval::Error::Custom(format!(
                "Second rounding argument must be an integer. Given value: {}",
                decimals
            )));
        };

        // Round the number and return it appropriately.
        let rounded = round_value(value, decimals as i32);

        // If the value is equivalent of an integer, return an integer form of it.
        match rounded.fract() == 0.0 {
            true => Ok(serde_json::json!(rounded as i64)),
            false => Ok(serde_json::json!(rounded)),
        }
    });

    // Fill the expression with variables from the data.
    // TODO: Look into if the "json has to be object" check may have side-effects.
    if let Json::Object(obj) = data {
        for (key, val) in obj {
            expr = expr.value(key, val);
        }
    };

    // Execute the expression.
    match expr.exec() {
        Err(err) => {
            let mut err_str = err.to_string();

            // If a null was encountered, it is likely that a conexistent key was indexed.
            if err_str.contains("Null") {
                err_str += ". Perhaps a key is misspelled?"
            }

            Err(format!(
                "Error in expression: '{}': {}",
                expr_string, err_str
            ))
        }
        Ok(Json::Null) => Err(format!("Expression '{}' returned Null value", expr_string)),
        Ok(v) => Ok(v),
    }
}

/// Evaluate an expression. If needed, recursively evaluate other expressions that it depends on.
///
/// # Arguments
/// * `expression`: The expression to evaluate.
/// * `data`: The "context" data to parse variables from.
/// * `recursion_depth`: The current recursion depth (only needed internally).
fn evaluate_expression(
    expression: &str,
    data: &Json,
    recursion_depth: usize,
) -> Result<Json, String> {
    // Avoid circular expressions by setting a max recursion depth.
    if recursion_depth > 1000 {
        return Err(format!(
            "Max recursion depth reached for expression: '{}'. Maybe due to a circular expression?",
            expression
        ));
    };

    // Format the expression string and remove the "expr:" part.
    let mut expr_string = expression.replacen("expr:", "", 1).trim().to_owned();

    // Find any expressions in the data and check if an associated key is referred to in the
    // expression.
    let expressions = find_expressions(data, None);
    for (keys, expression_str) in &expressions {
        // If the key exists in the current expression, evaluate the referred expression first.
        // TODO: Maybe make data mutable so all expressions only have to be evaluated once?
        if expr_string.contains(&keys.join(".")) {
            // Evaluate the referred expression.
            let value = evaluate_expression(&expression_str, &data, recursion_depth + 1)?;
            // Replace its key in the current expression with the evaluated value.
            expr_string = expr_string.replace(&keys.join("."), &value.to_string());
        }
    }

    // Now that all potential referred expressions have been evaluated, evaluate the current one.
    run_eval(&expr_string, &data)
}

/// Set data in a json at an arbitrary tree depth.
///
/// It does not set new keys, it only replaces the content of an existing key.
///
/// # Arguments
/// * `data`: The json to set a value in.
/// * `keys`: A vector of keys to index `data` with.
/// * `value`: The new value to set
///
/// # Returns
/// Nothing if it worked, or an error saying "Key not found" if the key did not exist.
fn replace_value_in_data(data: &mut Json, keys: &[String], value: Json) -> Result<(), String> {
    // If the keys is not just a single key, recursively dive into the tree.
    if keys.len() > 1 {
        // Extract the first key.
        let first_key = &keys[0];

        // Try to get the value of the first key.
        let mut subset = match data.get_mut(first_key) {
            Some(s) => s,
            None => return Err("Key not found".into()),
        };
        // Run the function again on the next keys.
        replace_value_in_data(&mut subset, &keys[1..], value)?;
    } else {
        // If the keys is a single key (it will be reached using recursion if not)...
        // ... try to set the value.

        match data.get_mut(&keys[0]) {
            Some(v) => (*v = value),
            None => return Err("Key not found".into()),
        };
    };

    Ok(())
}

/// Try to evaluate all expressions in a data file.
///
/// # Arguments
/// `data`: The data file to evaluate expressions inside.
///
/// # Returns
/// A copy of the data file with expressions filled, or an error detailing why it failed.
fn evaluate_all_expressions(data: &Json) -> Result<Json, String> {
    let mut new_data = data.clone();

    // Find all expressions and evaluate them (recursively if needed)
    for (keys, expr_string) in find_expressions(data, None) {
        let new_value = match evaluate_expression(&expr_string, &new_data, 0) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "Error for expression in '{}' ('{}'): {:?}",
                    keys.join("."),
                    expr_string,
                    e
                ))
            }
        };
        // Replace the expression with the evaluated value.
        match replace_value_in_data(&mut new_data, &keys, new_value) {
            Ok(_) => (),
            Err(e) => return Err(format!("Error setting key '{}': {}", keys.join("."), e)),
        };
    }
    Ok(new_data)
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

        let new_lines = fill_data(&lines, &data).unwrap();

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

        let new_lines = fill_data(&lines, &data).unwrap();

        assert_eq!(new_lines[0], "The year was once 2000");
        assert_eq!(new_lines[1], "This package is called manus.")
    }

    #[test]
    fn test_round_helpers() {
        let lines: Vec<String> = vec![
            "Hello".into(),
            "{{large_value}} rounded to the nearest 1000 is {{roundup 3 large_value}}".into(),
            "{{decimal_value}} rounded to one decimal is {{round 1 decimal_value}}".into(),
        ];

        // Try the large value as an integer and decimal_value as a string.
        let data = serde_json::json!({"large_value": 8699, "decimal_value": "1.234"});

        let new_lines = fill_data(&lines, &data).unwrap();

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
            "The other value is {{pm value2}}".into(),
        ];

        let data = serde_json::json!({"data": {"value": 1.2345, "value_pm": 0.2345}, "value2": 2, "value2_pm": 0.1});

        let new_lines = fill_data(&lines, &data).unwrap();

        assert_eq!(new_lines[0], "The value is 1.2345$\\pm$0.2345");
        assert_eq!(new_lines[1], "The value is 1.2$\\pm$0.2");
        assert_eq!(new_lines[2], "The other value is 2$\\pm$0.1");
    }

    #[test]
    fn test_sep_helper() {
        let lines: Vec<String> = vec![
            "10000 is a large number.".into(),
            "{{sep 10000}} looks better.".into(),
            "{{sep str_with_numerics}}".into(),
            "{{sep (pm value)}}".into(),
        ];

        assert_eq!(add_separators(10000., ","), "10,000");
        assert_eq!(add_separators(123456.78901, ","), "123,456.78901");
        assert_eq!(add_separators(123456., "\\,"), "123\\,456");

        let data = serde_json::json!({
            "separator": ",",
            "str_with_numerics": "Data are 12345 years old with a mean of 1.4858",
            "value": -123456789,
            "value_pm": 12456
        });

        let new_lines = fill_data(&lines, &data).unwrap();

        assert_eq!(new_lines[0], "10000 is a large number.");
        assert_eq!(new_lines[1], "10,000 looks better.");
        assert_eq!(
            new_lines[2],
            "Data are 12,345 years old with a mean of 1.4858"
        );
        assert_eq!(new_lines[3], "-123,456,789$\\pm$12,456");
    }

    #[test]
    fn test_expressions() {
        let lines: Vec<String> = vec![
            "The percentage of {{small}} out of {{large}} is {{round percentage}}".into(),
            "Adding one percentage point, it becomes: {{round added_percentage}}".into(),
        ];

        let data = serde_json::json!({
            "large": 10000,
            "small": 200,
            "percentage": "expr: 100 * small / large",
            "added_percentage": "expr: percentage + 1",
            "three": "expr: 1 + 2",
            "nested_expressions": {
                "value_sum": "expr: large + small",
            }
        });

        assert_eq!(run_eval(&"100 * 3", &data), Ok(serde_json::json!(300)));
        assert_eq!(
            run_eval("round(1.23, 1)", &data),
            Ok(serde_json::json!(1.2))
        );
        assert_eq!(run_eval("round(1.23)", &data), Ok(serde_json::json!(1)));
        // Check that the second argument has an integer-check
        match run_eval("round(1.23, 1.2)", &data) {
            Ok(v) => panic!("This should have failed!: {:?}", v),
            Err(e) => assert!(e.contains("must be an integer")),
        }

        // This will fail because of a misspelled key.
        match run_eval(&"largee + small", &data) {
            Ok(v) => panic!("This should have failed!: {:?}", v),
            Err(e) => assert!(e.contains("Perhaps a key is misspelled?")),
        };

        println!("{:?}", find_expressions(&data, None));

        let parsed_data = evaluate_all_expressions(&data).unwrap();
        let new_lines = fill_data(&lines, &parsed_data).unwrap();

        assert_eq!(parsed_data["three"], serde_json::json!(3));
        assert_eq!(parsed_data["percentage"], serde_json::json!(2.0));
        assert_eq!(
            parsed_data["nested_expressions"]["value_sum"],
            serde_json::json!(10200)
        );

        assert_eq!(new_lines[0], "The percentage of 200 out of 10000 is 2");
        assert_eq!(new_lines[1], "Adding one percentage point, it becomes: 3");

        // Make some expressions with circular dependencies (should raise a recursion error).
        let data = serde_json::json!({
            "ex1": "expr: ex2 + 1",
            "ex2": "expr: ex1 + 1",
            "ex3": "expr: ex3 + 1"
        });

        assert!(data.is_object());

        match evaluate_expression("ex1 + ex2", &data, 0) {
            Ok(v) => panic!("This should have failed!: {:?}", v),
            Err(s) => assert!(s.contains("recursion"), "{}", s),
        };
    }
}
