use std::collections::HashMap;

pub fn parse_template(template: &str, title: &str, arguments: HashMap<&str, &str>) -> String {
    let mut tmpl = template.to_owned();

    for (key, value) in arguments {
        tmpl = tmpl.replace(&format!("{{{{{}}}}}", key), value);
    }

    include_str!("../templates/app.html")
        .replace("{{title}}", title)
        .replace("{{content}}", &tmpl)
}
