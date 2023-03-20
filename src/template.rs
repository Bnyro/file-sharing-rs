use std::collections::HashMap;

use include_dir::{include_dir, Dir};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn parse_template(template: &str, title: &str, arguments: HashMap<&str, &str>) -> String {
    let mut tmpl = template.to_owned();

    for (key, value) in arguments {
        tmpl = tmpl.replace(&format!("{{{{{}}}}}", key), value);
    }

    get_template("app.html")
        .replace("{{title}}", title)
        .replace("{{content}}", &tmpl)
}

pub fn get_template(name: &str) -> &str {
    return TEMPLATES.get_file(name).unwrap().contents_utf8().unwrap();
}
